# Gochu - Vietnamese Telex Input Engine

Vietnamese Telex input method engine. Core written in Rust (no_std compatible), compiled to WASM for a web frontend, designed for future reuse as a native Linux IBus/Fcitx input method.

## Project Structure

```
gochu/
├── Cargo.toml              # Workspace root (members: gochu-core, gochu-wasm), resolver = "2"
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

**Files:** `docs/index.html` (semantic HTML), `docs/style.css` (all styles), `docs/main.js` (WASM loader + input handling), `docs/pkg/` (wasm-pack output).

**Theming:** CSS custom properties on `:root` with `@media (prefers-color-scheme: dark)` override. 20+ variables covering backgrounds, borders, text, toggles, and indicators. No JS needed for theme switching — follows OS preference automatically.

**Responsive:** Fluid layout with `max-width: 640px`. Mobile breakpoint at `480px` adjusts padding, font sizes, and textarea height. Uses `100dvh` for correct mobile viewport.

**Accessibility:** Semantic `<header>`, `<main>`, `<footer>`. Toggle is a `<button>` with `aria-pressed`. Preedit has `aria-live="polite"`. Textarea has `aria-label`. `<meta name="color-scheme" content="light dark">` for native form control theming.

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

# Build WASM (output goes to docs/pkg/)
wasm-pack build gochu-wasm --target web --out-dir ../docs/pkg

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

## Future Plans

The `gochu-core` crate is designed to be reused in a native Linux IBus or Fcitx5 input method plugin, sharing 100% of the Telex logic with the WASM version.
