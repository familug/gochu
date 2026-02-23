## gochu

Rust Vietnamese Telex engine with two frontends:

- **Web**: WASM demo under `docs/` (for GitHub Pages).
- **IBus**: native input method engine for Linux desktops.

This guide focuses on setting up the IBus engine.

### Install using prebuilt binary (recommended)

If a prebuilt binary for your platform is available on the project’s GitHub Releases page, prefer this method.

Prerequisites:

- IBus running on your desktop (e.g. GNOME, KDE, etc.).
- `curl` and `sudo` available.

1. **Download the prebuilt binary**

   Go to the project’s **Releases** page on GitHub and download the latest `gochu-ibus-linux-x86_64` binary (or the binary that matches your platform) to a directory of your choice.

2. **Install the binary**

   ```bash
   chmod +x gochu-ibus-linux-x86_64
   sudo install -Dm755 gochu-ibus-linux-x86_64 /usr/local/bin/gochu-ibus
   ```

3. **Install the IBus component file**

   ```bash
   curl -fsSL https://raw.githubusercontent.com/familug/gochu/main/gochu-ibus/data/gochu.xml \
     | sudo install -Dm644 /dev/stdin /usr/share/ibus/component/gochu.xml
   ```

On most Linux distributions this will install:

- The binary to `/usr/local/bin/gochu-ibus`.
- The component file to `/usr/share/ibus/component/gochu.xml`.

If your system uses a different IBus component directory (for example on some BSDs), replace `/usr/share/ibus/component/gochu.xml` with the appropriate path.

### Build and install from source (if no prebuilt binary)

If there is no suitable prebuilt binary for your system, or you prefer to build from source, use this method.

Prerequisites:

- A recent Rust toolchain (`rustup` + `cargo`).
- IBus running on your desktop (e.g. GNOME, KDE, etc.).

Build the engine:

```bash
cd gochu
cargo build --release -p gochu-ibus
```

Install the binary and IBus component:

```bash
cd gochu-ibus
sudo ./install.sh
```

By default this:

- Installs the binary to `/usr/local/bin/gochu-ibus`.
- Installs the component file to `/usr/share/ibus/component/gochu.xml`.

You can override the prefix if you prefer:

```bash
cd gochu-ibus
sudo PREFIX=/usr ./install.sh
```

### Restart IBus

After installation, restart the IBus daemon so it picks up the new engine:

```bash
ibus restart
```

If that command is not available on your distro, log out and back in, or kill and restart `ibus-daemon` from a terminal.

### Enable the engine in your desktop

On most desktops:

1. Open your system’s *Region & Language* / *Keyboard* / *Input Method* settings.
2. Add a new input method.
3. Look for an entry named **gochu** or **Gochu Telex** (depending on how your environment lists IBus engines).
4. Add it to your input source list.
5. Use your usual keybinding (often `Super`+`Space` or `Ctrl`+`Space`) to switch to it.

Once enabled, you should be able to type Vietnamese Telex in any IBus‑aware application (terminal, editors, browsers, etc.).

### Debug logging and privacy

By default the IBus engine does **not** write any user input to disk.

Debug logging is fully opt‑in and only enabled when you set `GOCHU_DEBUG` in the environment of `ibus-daemon`. When enabled, log messages go to `/tmp/gochu-ibus.log` and are intended for engine debugging only.

To start IBus with debug logging enabled for gochu:

```bash
ibus exit              # or pkill ibus-daemon
GOCHU_DEBUG=1 ibus-daemon --daemonize --xim
```

Then you can inspect the log:

```bash
cat /tmp/gochu-ibus.log
```

Restart IBus normally (without `GOCHU_DEBUG`) when you are finished debugging so that logging is disabled again.

### Troubleshooting

- **Engine does not appear in input method list**
  - Confirm the component file exists:
    ```bash
    ls /usr/share/ibus/component/gochu.xml
    ```
  - Restart the daemon:
    ```bash
    ibus restart
    ```

- **Engine appears but produces no text**
  - Make sure you are actually switched to the gochu engine (not another Telex engine such as m17n).
  - Check the log:
    ```bash
    cat /tmp/gochu-ibus.log
    ```
    If you see `CreateEngine("gochu-telex")` and `commit: ...` lines but nothing shows in applications, there is likely an IBus configuration or environment issue.

- **Firefox behaves differently from terminals/other apps**
  - Some apps are more strict about IBus D‑Bus methods such as `SetSurroundingText`. This engine implements the required interfaces; if Firefox still misbehaves, ensure it is actually using IBus (on some distros it may use another input framework).

### Releasing

From the repo root, with a clean working tree and push access to the remote:

```bash
cargo run -p xtask -- release 0.4.0
```

This bumps all crate versions, runs tests, rebuilds the web demo, then commits, tags `v0.4.0`, and pushes. The GitHub Actions release workflow builds the Linux binary when the tag is pushed.
