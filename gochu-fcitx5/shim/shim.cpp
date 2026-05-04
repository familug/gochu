// C++ shim: implements the fcitx5 addon/IM-engine C++ interface and delegates
// all Telex logic to `extern "C"` functions implemented in Rust (lib.rs).
//
// This file is compiled by build.rs via the `cc` crate and linked into the
// cdylib.  Only the minimal fcitx5 headers needed to satisfy the vtable are
// used; no other fcitx5 C++ API surfaces are accessed from Rust directly.

#include <cstring>

#include <fcitx-utils/key.h>
#include <fcitx/addonfactory.h>
#include <fcitx/addonmanager.h>
#include <fcitx/event.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/inputpanel.h>
#include <fcitx/text.h>

// ---------------------------------------------------------------------------
// Functions implemented in Rust (src/lib.rs)
// ---------------------------------------------------------------------------
extern "C" {
    void *gochu_create();
    void  gochu_destroy(void *state);

    // Returns 1 if the key was consumed, 0 if it should be forwarded.
    int   gochu_key_event(void *state, uint32_t keyval, bool is_release,
                          void *ic);
    void  gochu_flush(void *state, void *ic);
    void  gochu_escape(void *state, void *ic);
}

// ---------------------------------------------------------------------------
// Callbacks invoked by Rust to interact with the active InputContext
// ---------------------------------------------------------------------------
extern "C" void gochu_ic_commit(void *ic_ptr, const char *text) {
    auto *ic = static_cast<fcitx::InputContext *>(ic_ptr);
    ic->commitString(text);
}

extern "C" void gochu_ic_update_preedit(void *ic_ptr, const char *text) {
    auto *ic = static_cast<fcitx::InputContext *>(ic_ptr);
    auto &ip  = ic->inputPanel();
    ip.reset();
    fcitx::Text preedit(text);
    // setCursor takes a byte offset; using strlen gives cursor-at-end.
    preedit.setCursor(static_cast<int>(strlen(text)));
    ip.setPreedit(preedit);
    ic->updatePreedit();
    ic->updateUserInterface(fcitx::UserInterfaceComponent::InputPanel);
}

extern "C" void gochu_ic_clear_preedit(void *ic_ptr) {
    auto *ic = static_cast<fcitx::InputContext *>(ic_ptr);
    ic->inputPanel().reset();
    ic->updatePreedit();
    ic->updateUserInterface(fcitx::UserInterfaceComponent::InputPanel);
}

// ---------------------------------------------------------------------------
// fcitx5 InputMethodEngine implementation
// ---------------------------------------------------------------------------
class GochuEngine : public fcitx::InputMethodEngine {
public:
    GochuEngine() : state_(gochu_create()) {}
    ~GochuEngine() override { gochu_destroy(state_); }

    void activate(const fcitx::InputMethodEntry &,
                  fcitx::InputContextEvent &) override {}

    void deactivate(const fcitx::InputMethodEntry &,
                    fcitx::InputContextEvent &event) override {
        gochu_flush(state_, event.inputContext());
    }

    void keyEvent(const fcitx::InputMethodEntry &,
                  fcitx::KeyEvent &event) override {
        // Pass through keys with Ctrl / Alt / Super modifiers.
        if (event.key().states() &
            (fcitx::KeyState::Ctrl | fcitx::KeyState::Alt |
             fcitx::KeyState::Super)) {
            gochu_flush(state_, event.inputContext());
            return;
        }

        auto sym = static_cast<uint32_t>(event.key().sym());

        // Enter / KP_Enter: flush composing buffer and let the app handle it.
        if (sym == FcitxKey_Return || sym == FcitxKey_KP_Enter) {
            gochu_flush(state_, event.inputContext());
            return;
        }

        // Escape: discard composing buffer without committing.
        if (sym == FcitxKey_Escape) {
            gochu_escape(state_, event.inputContext());
            return;
        }

        if (gochu_key_event(state_, sym, event.isRelease(),
                             event.inputContext())) {
            event.filterAndAccept();
        }
    }

    void reset(const fcitx::InputMethodEntry &,
               fcitx::InputContextEvent &event) override {
        gochu_escape(state_, event.inputContext());
    }

    void focusOut(const fcitx::InputMethodEntry &,
                  fcitx::InputContextEvent &event) override {
        gochu_flush(state_, event.inputContext());
    }

private:
    void *state_;
};

// ---------------------------------------------------------------------------
// fcitx5 AddonFactory — creates one GochuEngine per input context group
// ---------------------------------------------------------------------------
class GochuFactory : public fcitx::AddonFactory {
public:
    fcitx::AddonInstance *create(fcitx::AddonManager *) override {
        return new GochuEngine();
    }
};

FCITX_ADDON_FACTORY(GochuFactory)
