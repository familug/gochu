# Gochu - Vietnamese Telex Input Engine

Vietnamese Telex input method engine. Core written in Rust (no_std compatible), compiled to WASM for a web frontend, designed for future reuse as a native Linux IBus/Fcitx input method.

## Project Structure

```
gochu/
├── Cargo.toml              # Workspace root (members: gochu-core, gochu-wasm, gochu-ibus), resolver = "2"
├── gochu-core/             # Pure Rust Telex engine — ZERO external dependencies
│   ├── Cargo.toml          # Features: default=["std"], std=[]
│   └── src/
│       ├── lib.rs           # Exports: Action, TelexEngine. #![cfg_attr(not(feature = "std"), no_std)]
│       ├── transform.rs     # FUNCTIONAL CORE: pure functions (classify_key, apply_effect, replay)
│       ├── engine.rs        # IMPERATIVE SHELL: TelexEngine stateful wrapper over transform
│       ├── tone.rs          # Tone enum, TONE_TABLE (24 rows x 6 cols), apply/strip/get tone
│       └── vowel.rs         # Vowel detection, modification (aa→â etc), tone placement rules
├── gochu-wasm/             # WASM bindings via wasm-bindgen
│   ├── Cargo.toml          # crate-type = ["cdylib", "rlib"], deps: gochu-core (no default-features), wasm-bindgen, js-sys
│   ├── src/lib.rs           # Gochu struct wrapping TelexEngine, exposed to JS
│   └── tests/web.rs         # wasm-bindgen-test integration tests (run via Node.js)
├── gochu-ibus/             # Native IBus input method engine (Linux + OpenBSD)
│   ├── Cargo.toml          # deps: gochu-core, zbus 4, tokio
│   ├── src/
│   │   ├── main.rs          # Entry point: IBus address lookup, D-Bus connection, factory registration
│   │   ├── engine.rs        # IBus Engine + Factory D-Bus interfaces (wraps gochu-core TelexEngine)
│   │   └── text.rs          # IBusText/IBusAttrList GVariant construction
│   ├── data/gochu.xml       # IBus component descriptor (engine name, language, icon)
│   └── install.sh           # Build + install binary and component XML
└── docs/                   # Static web frontend (no bundler), served by GitHub Pages from /docs
    ├── index.html           # Semantic HTML, accessibility attributes, meta tags
    ├── style.css            # All styles; CSS custom properties; light/dark via prefers-color-scheme; mobile breakpoint at 480px
    ├── main.js              # Loads WASM, intercepts keydown, manages committed/composing text
    └── pkg/                 # wasm-pack output (auto-generated, do not edit)
```

## Architecture: Functional Core / Imperative Shell

The codebase follows the **functional core / imperative shell** pattern:

- **Functional core** (`transform.rs`, `tone.rs`, `vowel.rs`): Pure functions with no mutable state. Every function takes input and returns output. All Telex decision-making lives here. Independently testable.
- **Imperative shell** (`engine.rs`): `TelexEngine` is a thin stateful wrapper that manages `buf`/`raw` vectors and delegates all decisions to `transform::classify_key()` and `transform::apply_effect()`.
- **WASM shell** (`gochu-wasm`): Thin wasm-bindgen layer wrapping `TelexEngine` into a JS-friendly `Gochu` class.
- **Web frontend** (`web/`): Loads WASM, intercepts keyboard events, manages committed/composing text buffers.

### Dependencies

- `gochu-core`: **zero** external dependencies. Uses only `core` + `alloc`. `no_std` compatible.
- `gochu-wasm`: `wasm-bindgen` 0.2, `js-sys` 0.3 (dev: `wasm-bindgen-test` 0.3)
- All dependencies audited clean via `cargo audit` (zero advisories).

## Agent Maintenance Rule

**Always keep this document up to date.**

Whenever you (an automated agent) make a non-trivial change to behavior, protocols, D-Bus signatures, logging, Telex rules, or installation/usage flows, you must:

- Update `AGENTS.md` with the new knowledge in the appropriate section (or add a new section if needed).
- Update `README.md` if the change affects how users build, install, configure, or safely use the project (especially around privacy, logging, or input behavior).
- Prefer concise, factual notes that future agents can rely on without re-deriving past discoveries.

In addition, for every behavior change, feature, or bugfix:

- Add or update at least one **unit test** in the relevant crate (`gochu-core`, `gochu-ibus`, `gochu-wasm`, or the web JS) that captures the expected behavior (e.g. specific key sequences, D-Bus signatures, logging behavior).
- Do not remove existing tests that describe real-world behaviors users depend on unless you also update this document to explain why the behavior changed.

**WASM / GitHub Pages:** After any code change that affects the web demo (e.g. `gochu-core`, `gochu-wasm`, or `docs/`), run **`./build.sh`** from the repo root so `docs/pkg/` is rebuilt and stays in sync. Commit the updated `docs/pkg/` (and any changed `docs/*.js` / `docs/*.html` / `docs/*.css`) so that GitHub Pages serves the latest WASM and assets. Do not forget this step — the site does not build on GitHub; it only serves what you push.

## Core Engine Design (gochu-core)

### Pure Transform Layer (transform.rs)

`classify_key(key: char, buf: &[char]) -> KeyEffect` — classifies a keystroke given current buffer state, returning one of:
- `ToneApplied { position, replacement }` — a tone was resolved
- `DdApplied { position, replacement }` — dd → đ
- `VowelModified { position, replacement }` — vowel modification (e.g. a→â)
- `WAsVowel(char)` — standalone w → ư
- `Append(char)` — regular character
- `Commit(char)` — word separator or non-alpha triggers commit
- `Backspace`

`apply_effect(buf: &[char], effect: &KeyEffect) -> Vec<char>` — pure: applies an effect to a buffer, returning a new buffer.

`replay(raw_keys: &[char]) -> Vec<char>` — pure: replays a full sequence of raw keys to reconstruct the buffer (used for backspace).

### TelexEngine (engine.rs) — Imperative Shell

State: `buf: Vec<char>` (composed output), `raw: Vec<char>` (telex keystrokes), `composing: bool`.

`process_key(char) -> Action` where Action is `Composing(String)` or `Commit(String)`.

The engine calls `classify_key` then pattern-matches the effect:
- `Backspace` → pops from `raw`, calls `transform::replay()` to reconstruct `buf`
- `Commit` → flushes display + commit char, resets state
- All others → pushes to `raw`, calls `apply_effect` to update `buf`

### Tone Placement (vowel.rs: tone_position)

Rules in priority:
1. Single vowel → tone on it
2. Exactly one modified vowel (â, ê, ô, ơ, ư, ă) → tone on it
3. Multiple modified vowels (e.g. ươ) → fall through to cluster rules
4. 3+ vowel cluster → tone on second vowel
5. 2-vowel cluster: closed syllable (final consonant) → second vowel, open → first vowel

### Tone Table (tone.rs)

24 rows (12 vowels × 2 cases), 6 columns: [base, sắc, huyền, hỏi, ngã, nặng]. Covers a/ă/â/e/ê/i/o/ô/ơ/u/ư/y and uppercase variants.

`is_vowel()` strips tones before checking, so toned characters like `á`, `ề` are correctly recognized as vowels.

### Vowel Modification (vowel.rs: modify_vowel)

Maps: (a,a)→â, (a,w)→ă, (e,e)→ê, (o,o)→ô, (o,w)→ơ, (u,w)→ư. Preserves existing tone and case.

## WASM Bindings (gochu-wasm)

The `Gochu` JS class exposes:
- `constructor()` / `new Gochu()`
- `process_key(key: char) -> { type: "composing"|"commit", text: string }`
- `get_display() -> string`
- `is_composing() -> boolean`
- `reset()`

## Web Frontend (web/)

Minimal static site, deployable to GitHub Pages as-is (no build step needed beyond wasm-pack).

**Files:** `docs/index.html` (semantic HTML), `docs/style.css` (all styles), `docs/main.js` (WASM loader + input handling), `docs/pkg/` (wasm-pack output, committed so GitHub Pages can serve it directly).

**Theming:** CSS custom properties on `:root` with `@media (prefers-color-scheme: dark)` override. 20+ variables covering backgrounds, borders, text, toggles, and indicators. No JS needed for theme switching — follows OS preference automatically.

**Responsive:** Fluid layout with `max-width: 640px`. Mobile breakpoint at `480px` adjusts padding, font sizes, and textarea height. Uses `100dvh` for correct mobile viewport.

**Accessibility:** Semantic `<header>`, `<main>`, `<footer>`. Toggle is a `<button>` with `aria-pressed`. Preedit has `aria-live="polite"`. Textarea has `aria-label`. `<meta name="color-scheme" content="light dark">` for native form control theming.

**Agent rule for web/GitHub Pages:**

- After any change that affects the web demo (e.g. `gochu-core`, `gochu-wasm`, or `docs/`), **run `./build.sh`** so `docs/pkg/` is rebuilt. Commit updated `docs/pkg/` and any changed `docs/*.js` / `docs/*.html` / `docs/*.css`. GitHub Pages does not build; it only serves what you push.
- When you change behavior (keyboard handling, Telex semantics, UI), update `docs/main.js` / `docs/index.html` / `docs/style.css` as needed so the live site reflects the latest behavior.

**Input handling (`main.js`):** Two-layer text model — `committed` (finalized string) and the engine's composing buffer. On each keydown, feeds key to `gochu.process_key()`. On "commit" action, appends to `committed` and resets engine. Textarea always shows `committed + composing`.

## Tests

68 unit tests in `gochu-core` across 4 modules:
- `tone::tests` — roundtrip apply/strip, all tone variants, passthrough for non-vowels
- `vowel::tests` — is_vowel (including toned chars), modify_vowel, tone_position for all cluster types
- `transform::tests` — classify_key for each KeyEffect variant, apply_effect, replay
- `engine::tests` — integration: full words, backspace, commit, passthrough, multi-word, uppercase

7 integration tests in `gochu-wasm` via `wasm-bindgen-test` (Node.js).

## Build Commands

```bash
# Run core tests
cargo test -p gochu-core

# Run WASM integration tests
wasm-pack test --node gochu-wasm

# Build WASM (output goes to docs/pkg/, removes generated .gitignore)
./build.sh

# Serve locally
cd docs && python3 -m http.server 8080

# Security audit
cargo audit
```

## Telex Reference

| Input | Output | Input | Output |
|-------|--------|-------|--------|
| aa    | â      | s     | sắc (´)  |
| aw    | ă      | f     | huyền (`) |
| ee    | ê      | r     | hỏi (̉)   |
| oo    | ô      | x     | ngã (~)  |
| ow    | ơ      | j     | nặng (.) |
| uw    | ư      | z     | remove tone |
| dd    | đ      |       |        |

## Native IBus Engine (gochu-ibus)

A native input method engine for Ubuntu Linux and OpenBSD using the IBus framework over D-Bus.

**Architecture:** Pure Rust, no C dependencies. Uses `zbus` (pure Rust D-Bus implementation) to communicate with the IBus daemon. Reuses `gochu-core` for all Telex logic.

**D-Bus interfaces implemented:**
- `org.freedesktop.IBus.Factory` — `CreateEngine(name) -> ObjectPath`. IBus calls this to instantiate engines.
- `org.freedesktop.IBus.Engine` — `ProcessKeyEvent(keyval, keycode, state) -> bool` plus `FocusIn/Out`, `Reset`, `Enable/Disable`. Emits signals: `CommitText`, `UpdatePreeditText`, `HidePreeditText`.

**IBusText serialization** (`text.rs`): Constructs the GVariant `(sa{sv}sv)` format that IBus expects. IBusText wraps the text string and an IBusAttrList `(sa{sv}av)`. The struct is built using a `Field` wrapper so that `StructureBuilder` sees the *contained* type signatures (`s`, `a{sv}`, `v`) instead of the generic `Value` variant type (`v` for everything). There are unit tests that hex‑dump the serialized body and assert the inner signature is exactly `(sa{sv}sv)`.

**Key event handling:** Maps X11 keysyms (0x20–0x7e for printable ASCII, 0xff08 for BackSpace) to chars, ignores release events and Ctrl/Alt/Super modifiers, feeds to `TelexEngine::process_key()`. Backspace behavior:

- When `TelexEngine` returns `Action::Composing(_)`, the engine consumes the key and updates preedit.
- When it returns `Action::Commit(text)` with non‑empty `text`, the engine emits `CommitText` and returns `true`.
- When it returns `Action::Commit("")` (empty string, used by core to mean “no composing text; let the app handle deletion”), the engine hides preedit and returns `false` so IBus forwards Backspace to the application.

**Connection:** Finds the private IBus address by scanning `~/.config/ibus/bus/` for the current `machine_id` and using the newest entry. Connects in **p2p mode** via `zbus::connection::Builder::address(...).p2p().build()`, then manually calls `Hello` and `RequestName(BUS_NAME)` on `org.freedesktop.DBus` so the daemon associates the connection with the component.

**Signals:** Uses `Message::signal(...).build(&body)` instead of `emit_signal`, passing bodies whose `DynamicType` matches the IBus spec:

- `CommitText`: body `v` where the variant contains `IBusText` `(sa{sv}sv)`.
- `UpdatePreeditText`: body `vubu` — `(IBusText, cursor_pos: u32, visible: bool, mode: u32)`.
- `HidePreeditText`: empty body.

There are unit tests in `text.rs` that assert the runtime signature of these bodies and roundtrip them through zvariant serialization/deserialization.

**Logging / privacy:** `gochu-ibus` logs to `/tmp/gochu-ibus.log` **only** when `GOCHU_DEBUG` is set in the environment of `ibus-daemon`. Without `GOCHU_DEBUG`, the `log()` function is a no‑op and does not create the file. All debug logging has been restricted to high‑level engine events; user text (preedit and commit content) is no longer logged. There are unit tests in `main.rs` that verify:

- Without `GOCHU_DEBUG`, calling `log("...")` does not create or write `/tmp/gochu-ibus.log`.
- With `GOCHU_DEBUG=1`, `log("...")` creates the file and writes the given line.

**Install:**

```bash
# Build
cargo build --release -p gochu-ibus

# Install (run from repo root, needs sudo for system dirs)
sudo install -Dm755 target/release/gochu-ibus /usr/local/bin/gochu-ibus
sudo install -Dm644 gochu-ibus/data/gochu.xml /usr/share/ibus/component/gochu.xml

# Or use the script:
sudo gochu-ibus/install.sh

# Activate
ibus restart
# Then add "Gochu Telex" in Settings > Keyboard > Input Sources
```

**Cross-platform notes:**
- Ubuntu: IBus is the default. Component XML goes to `/usr/share/ibus/component/`.
- OpenBSD: IBus available in ports (`pkg_add ibus`). Component XML goes to `/usr/local/share/ibus/component/`. Set `IBUS_COMPONENT_DIR` when running install.sh.
