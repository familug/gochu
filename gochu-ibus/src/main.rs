use std::env;
use std::error::Error;
use std::path::PathBuf;

mod engine;
mod text;

const BUS_NAME: &str = "org.freedesktop.IBus.Gochu";
const LOG_PATH: &str = "/tmp/gochu-ibus.log";

pub(crate) fn log(msg: &str) {
    // Debug logging is intended for development only. It does not record any
    // user text content and is disabled unless GOCHU_DEBUG is set.
    if env::var_os("GOCHU_DEBUG").is_none() {
        return;
    }

    eprintln!("gochu: {msg}");
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_PATH)
    {
        let _ = writeln!(f, "{msg}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // These tests mutate process-wide state (env vars + shared log file),
    // so they must not run concurrently with each other.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_log_file() {
        let _ = std::fs::remove_file(LOG_PATH);
    }

    fn read_log() -> Option<String> {
        std::fs::read_to_string(LOG_PATH).ok()
    }

    #[test]
    fn log_is_noop_without_debug_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_log_file();
        unsafe { env::remove_var("GOCHU_DEBUG") };

        log("USER_INPUT_SHOULD_NOT_BE_LOGGED");

        // Without GOCHU_DEBUG, log must not create or write the file.
        assert!(
            !std::path::Path::new(LOG_PATH).exists(),
            "log() should not create a log file when GOCHU_DEBUG is unset"
        );
    }

    #[test]
    fn log_writes_only_when_debug_env_set() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_log_file();
        unsafe { env::set_var("GOCHU_DEBUG", "1") };

        log("GOCHU_DEBUG_TEST_LINE");

        let contents = read_log().expect("log file should exist when GOCHU_DEBUG is set");
        assert!(
            contents.contains("GOCHU_DEBUG_TEST_LINE"),
            "log file did not contain expected debug line"
        );

        // Clean up: remove env var so other tests aren't affected.
        unsafe { env::remove_var("GOCHU_DEBUG") };
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    if !env::args().any(|a| a == "--ibus") {
        eprintln!("Usage: gochu-ibus --ibus");
        eprintln!();
        eprintln!("IBus input method engine for Vietnamese Telex.");
        eprintln!("This binary is launched by the IBus daemon.");
        eprintln!();
        eprintln!("To install:");
        eprintln!("  cargo build --release -p gochu-ibus");
        eprintln!("  sudo install -Dm755 target/release/gochu-ibus /usr/local/bin/gochu-ibus");
        eprintln!("  sudo install -Dm644 gochu-ibus/data/gochu.xml /usr/share/ibus/component/gochu.xml");
        eprintln!("  ibus restart");
        std::process::exit(1);
    }

    let conn = connect_ibus().await?;

    let factory = engine::GochuFactory::new(conn.clone());
    conn.object_server()
        .at("/org/freedesktop/IBus/Factory", factory)
        .await?;

    log("ready");

    std::future::pending::<()>().await;
    Ok(())
}

/// Connect to the IBus private bus in peer-to-peer mode.
///
/// Using p2p mode is critical: libibus engines communicate directly with
/// the daemon over the raw socket. The daemon's engine proxy (GDBusProxy
/// with name=NULL) listens for signals on the direct connection, NOT
/// through bus signal routing. A zbus "bus" connection (which calls Hello)
/// sends signals through the bus routing layer, which the daemon's proxy
/// never sees.
///
/// We connect p2p, then manually call Hello + RequestName on the daemon's
/// org.freedesktop.DBus interface so it assigns us a name and knows which
/// component we are.
async fn connect_ibus() -> Result<zbus::Connection, Box<dyn Error>> {
    let addr = find_ibus_address_str()?;
    log(&format!("connecting to {addr}"));

    let address: zbus::Address = addr.as_str().try_into()?;
    let conn = zbus::connection::Builder::address(address)?
        .p2p()
        .build()
        .await?;

    // Manually call Hello — the IBus daemon implements org.freedesktop.DBus
    let reply = conn
        .call_method(
            None::<&str>,
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "Hello",
            &(),
        )
        .await?;
    let name: String = reply.body().deserialize()?;
    log(&format!("Hello: {name}"));

    // Request our well-known name so IBus associates us with the component
    let reply = conn
        .call_method(
            None::<&str>,
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "RequestName",
            &(BUS_NAME, 0u32),
        )
        .await?;
    let result: u32 = reply.body().deserialize()?;
    log(&format!("RequestName({BUS_NAME}): {result}"));

    Ok(conn)
}

fn find_ibus_address_str() -> Result<String, Box<dyn Error>> {
    if let Ok(addr) = env::var("IBUS_ADDRESS") {
        return Ok(addr);
    }
    if let Some(addr) = find_ibus_address() {
        return Ok(addr);
    }
    Err("no IBus address found (IBUS_ADDRESS not set, no socket file)".into())
}

fn find_ibus_address() -> Option<String> {
    let machine_id = read_machine_id()?;
    let config_home = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        format!("{}/.config", env::var("HOME").unwrap_or_default())
    });
    let bus_dir = PathBuf::from(&config_home).join("ibus").join("bus");

    let mut best: Option<(std::time::SystemTime, String)> = None;
    let entries = std::fs::read_dir(&bus_dir).ok()?;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with(&machine_id) {
            continue;
        }
        let contents = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for line in contents.lines() {
            if let Some(addr) = line.strip_prefix("IBUS_ADDRESS=") {
                let addr = addr.trim_matches(|c| c == '\'' || c == '"');
                let mtime = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                if best.as_ref().is_none_or(|(t, _)| mtime > *t) {
                    best = Some((mtime, addr.to_string()));
                }
            }
        }
    }
    best.map(|(_, addr)| addr)
}

fn read_machine_id() -> Option<String> {
    for path in [
        "/etc/machine-id",
        "/var/lib/dbus/machine-id",
        "/var/db/dbus/machine-id",
    ] {
        if let Ok(id) = std::fs::read_to_string(path) {
            let id = id.trim().to_string();
            if !id.is_empty() {
                return Some(id);
            }
        }
    }
    None
}
