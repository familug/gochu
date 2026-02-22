use std::env;
use std::error::Error;
use std::path::PathBuf;

mod engine;
mod text;

const BUS_NAME: &str = "org.freedesktop.IBus.Gochu";
const LOG_PATH: &str = "/tmp/gochu-ibus.log";

pub(crate) fn log(msg: &str) {
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

    // Tell IBus which component we are so it can route CreateEngine to us
    conn.request_name(BUS_NAME).await?;

    eprintln!("gochu-ibus: ready (name {BUS_NAME} acquired)");

    std::future::pending::<()>().await;
    Ok(())
}

async fn connect_ibus() -> Result<zbus::Connection, Box<dyn Error>> {
    // Strategy 1: IBUS_ADDRESS environment variable (set by IBus daemon for children)
    if let Ok(addr) = env::var("IBUS_ADDRESS") {
        eprintln!("gochu-ibus: connecting via IBUS_ADDRESS={addr}");
        let address: zbus::Address = addr.as_str().try_into()?;
        let conn = zbus::connection::Builder::address(address)?
            .build()
            .await?;
        return Ok(conn);
    }

    // Strategy 2: read address from ~/.config/ibus/bus/ files
    if let Some(addr) = find_ibus_address() {
        eprintln!("gochu-ibus: connecting via socket file: {addr}");
        let address: zbus::Address = addr.as_str().try_into()?;
        let conn = zbus::connection::Builder::address(address)?
            .build()
            .await?;
        return Ok(conn);
    }

    // Strategy 3: session bus (unlikely to work for IBus, but last resort)
    eprintln!("gochu-ibus: warning: no IBus address found, trying session bus");
    Ok(zbus::Connection::session().await?)
}

/// Discover the IBus private bus address from ~/.config/ibus/bus/.
///
/// IBus names the file `{machine_id}-{hostname}-{display_num}`.
/// Instead of guessing the hostname format (which varies between
/// /etc/hostname and uname nodename), we list the directory and match
/// by machine-id prefix, picking the most recently modified file.
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
                if best.as_ref().map_or(true, |(t, _)| mtime > *t) {
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
