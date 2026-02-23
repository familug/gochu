## gochu

Rust Vietnamese Telex engine with two frontends:

- **Web**: WASM demo under `docs/` (for GitHub Pages).
- **IBus**: native input method engine for Linux desktops.

This guide focuses on setting up the IBus engine.

### Build and install the IBus engine

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

### Verifying that it works

The engine logs to `/tmp/gochu-ibus.log`:

```bash
cat /tmp/gochu-ibus.log
```

A typical session shows lines like:

- `connecting to ...`
- `Hello: :1.x`
- `RequestName(org.freedesktop.IBus.Gochu): 1`
- `CreateEngine("gochu-telex") -> /org/freedesktop/IBus/Engine/0`
- `preedit: "vie"`
- `commit: "việt "`

If you see `preedit:` lines as you type and `commit:` lines when a word finishes, the engine is working and IBus is receiving both composing and committed text.

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

