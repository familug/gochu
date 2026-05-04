//! Gochu fcitx5 addon — Telex engine logic exposed via `extern "C"` to the
//! C++ shim in `shim/shim.cpp`.
//!
//! The C++ shim owns the vtable / ABI glue; this file owns the state machine.

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

use gochu_core::{Action, TelexEngine};

// ---------------------------------------------------------------------------
// Callbacks into C++ (implemented in shim.cpp)
// ---------------------------------------------------------------------------
extern "C" {
    fn gochu_ic_commit(ic: *mut c_void, text: *const c_char);
    fn gochu_ic_update_preedit(ic: *mut c_void, text: *const c_char);
    fn gochu_ic_clear_preedit(ic: *mut c_void);
}

// ---------------------------------------------------------------------------
// Per-engine state (one instance per fcitx5 input method group / connection)
// ---------------------------------------------------------------------------
struct GochuState {
    telex: TelexEngine,
}

// ---------------------------------------------------------------------------
// Exported C interface called by shim.cpp
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn gochu_create() -> *mut c_void {
    Box::into_raw(Box::new(GochuState {
        telex: TelexEngine::new(),
    })) as *mut c_void
}

/// # Safety
/// `ptr` must be a valid pointer returned by `gochu_create`, called at most once.
#[no_mangle]
pub unsafe extern "C" fn gochu_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut GochuState));
    }
}

/// Returns 1 if the key event was consumed, 0 if it should be forwarded.
///
/// # Safety
/// `ptr` and `ic` must be valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn gochu_key_event(
    ptr: *mut c_void,
    keyval: u32,
    is_release: bool,
    ic: *mut c_void,
) -> c_int {
    if is_release {
        return 0;
    }

    let state = &mut *(ptr as *mut GochuState);

    let ch = match keyval {
        0xff08 => '\x08', // XK_BackSpace
        0x20..=0x7e => keyval as u8 as char,
        _ => {
            // Unknown key: flush any composing text and pass the event through.
            do_flush(state, ic);
            return 0;
        }
    };

    match state.telex.process_key(ch) {
        Action::Composing(text) => {
            let c_text = CString::new(text).unwrap_or_default();
            gochu_ic_update_preedit(ic, c_text.as_ptr());
            1
        }
        Action::Commit(text) => {
            gochu_ic_clear_preedit(ic);
            // An empty Commit on Backspace means "buffer was already empty;
            // let the application handle the deletion".
            if text.is_empty() {
                0
            } else {
                let c_text = CString::new(text).unwrap_or_default();
                gochu_ic_commit(ic, c_text.as_ptr());
                1
            }
        }
    }
}

/// Flush the composing buffer as committed text (used on focus-out, Enter, etc.)
///
/// # Safety
/// `ptr` and `ic` must be valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn gochu_flush(ptr: *mut c_void, ic: *mut c_void) {
    let state = &mut *(ptr as *mut GochuState);
    do_flush(state, ic);
}

/// Discard the composing buffer without committing (used on Escape / Reset).
///
/// # Safety
/// `ptr` and `ic` must be valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn gochu_escape(ptr: *mut c_void, ic: *mut c_void) {
    let state = &mut *(ptr as *mut GochuState);
    if state.telex.is_composing() {
        state.telex.reset();
        gochu_ic_clear_preedit(ic);
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

unsafe fn do_flush(state: &mut GochuState, ic: *mut c_void) {
    if !state.telex.is_composing() {
        return;
    }
    let text = state.telex.get_display();
    state.telex.reset();
    gochu_ic_clear_preedit(ic);
    if !text.is_empty() {
        let c_text = CString::new(text).unwrap_or_default();
        gochu_ic_commit(ic, c_text.as_ptr());
    }
}

// ---------------------------------------------------------------------------
// Tests (no-IC path, exercising just the Telex state machine)
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use gochu_core::{Action, TelexEngine};

    fn type_word(engine: &mut TelexEngine, word: &str) -> String {
        let mut last = String::new();
        for ch in word.chars() {
            match engine.process_key(ch) {
                Action::Composing(s) | Action::Commit(s) => last = s,
            }
        }
        last
    }

    #[test]
    fn engine_basic_telex() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "vieejt"), "việt");
    }

    #[test]
    fn engine_flush_on_space() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "xin "), "xin ");
        assert!(!e.is_composing());
    }

    #[test]
    fn engine_backspace_empty_is_commit_empty() {
        let mut e = TelexEngine::new();
        match e.process_key('\x08') {
            Action::Commit(s) => assert_eq!(s, ""),
            other => panic!("expected Commit(\"\"), got {other:?}"),
        }
    }
}
