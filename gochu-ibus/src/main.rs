use std::env;
use std::error::Error;
use std::path::PathBuf;

mod engine;
mod text;

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

    eprintln!("gochu-ibus: registered, waiting for input contexts...");

    std::future::pending::<()>().await;
    Ok(())
}

async fn connect_ibus() -> Result<zbus::Connection, Box<dyn Error>> {
    if let Ok(addr) = env::var("IBUS_ADDRESS") {
        eprintln!("gochu-ibus: connecting to {addr}");
        let address: zbus::Address = addr.as_str().try_into()?;
        let conn = zbus::connection::Builder::address(address)?
            .build()
            .await?;
        return Ok(conn);
    }

    if let Some(addr) = find_ibus_address() {
        eprintln!("gochu-ibus: found IBus at {addr}");
        let address: zbus::Address = addr.as_str().try_into()?;
        let conn = zbus::connection::Builder::address(address)?
            .build()
            .await?;
        return Ok(conn);
    }

    eprintln!("gochu-ibus: using session bus (no IBUS_ADDRESS found)");
    Ok(zbus::Connection::session().await?)
}

fn find_ibus_address() -> Option<String> {
    let machine_id = read_machine_id()?;
    let display = env::var("DISPLAY").unwrap_or_else(|_| ":0".into());
    let display_num = display.rsplit(':').next()?.split('.').next()?;

    let hostname = std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unix".to_string());

    let filename = format!("{machine_id}-{hostname}-{display_num}");
    let config_home = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        format!("{}/.config", env::var("HOME").unwrap_or_default())
    });
    let path: PathBuf = [&config_home, "ibus", "bus", &filename].iter().collect();

    let contents = std::fs::read_to_string(path).ok()?;
    for line in contents.lines() {
        if let Some(addr) = line.strip_prefix("IBUS_ADDRESS=") {
            return Some(addr.to_string());
        }
    }
    None
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
