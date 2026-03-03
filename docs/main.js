import init, { Gochu } from './pkg/gochu_wasm.js';

async function main() {
  await init();

  const gochu = new Gochu();
  const editor = document.getElementById('editor');
  const preedit = document.getElementById('preedit');
  const modeToggle = document.getElementById('mode');

  let enabled = true;
  let committed = '';
  let composing = false;

  modeToggle.addEventListener('click', () => {
    enabled = !enabled;
    modeToggle.textContent = enabled ? 'Telex ON' : 'Telex OFF';
    modeToggle.classList.toggle('off', !enabled);
    modeToggle.setAttribute('aria-pressed', String(enabled));
    if (!enabled) {
      flushComposing();
    }
    editor.focus();
  });

  function flushComposing() {
    const display = gochu.get_display();
    if (display) {
      committed += display;
    }
    gochu.reset();
    updatePreedit();
    syncEditor();
  }

  function updatePreedit() {
    const display = gochu.get_display();
    if (display) {
      preedit.textContent = display;
      preedit.classList.add('active');
    } else {
      preedit.textContent = '\u2014';
      preedit.classList.remove('active');
    }
  }

  function syncEditor() {
    const display = gochu.get_display();
    editor.value = committed + display;
    editor.selectionStart = editor.selectionEnd = editor.value.length;
  }

  function feedChar(ch) {
    const result = gochu.process_key(ch);
    if (result.type === 'commit') {
      committed += result.text;
      gochu.reset();
    }
  }

  function feedString(s) {
    for (const ch of s) {
      feedChar(ch);
    }
    updatePreedit();
    syncEditor();
  }

  function handleBackspace() {
    if (gochu.is_composing()) {
      gochu.process_key('\x08');
    } else if (committed.length > 0) {
      const chars = [...committed];
      chars.pop();
      committed = chars.join('');
    }
    updatePreedit();
    syncEditor();
  }

  // Mobile keyboards (Samsung, GBoard, etc.) fire keydown with
  // key:'Unidentified' or key:'Process', so keydown alone misses all input.
  // beforeinput always carries the real data in e.data, on both mobile and
  // desktop. On desktop, keydown fires first and calls preventDefault(),
  // which suppresses the subsequent beforeinput — no double processing.
  editor.addEventListener('beforeinput', (e) => {
    if (!enabled) return;
    if (composing) return;

    switch (e.inputType) {
      case 'insertText':
        if (e.data) {
          e.preventDefault();
          feedString(e.data);
        }
        return;

      case 'deleteContentBackward':
        e.preventDefault();
        handleBackspace();
        return;

      case 'insertLineBreak':
      case 'insertParagraph':
        e.preventDefault();
        flushComposing();
        committed += '\n';
        syncEditor();
        return;
    }
  });

  // Some mobile keyboards use IME composition even for Latin text
  // (predictive/swipe input). Let the browser handle composition natively,
  // then process the final result.
  editor.addEventListener('compositionstart', () => {
    composing = true;
  });

  editor.addEventListener('compositionend', (e) => {
    composing = false;
    if (!enabled || !e.data) return;
    e.preventDefault();
    feedString(e.data);
  });

  // keydown: Tab (no beforeinput equivalent) and desktop fallback for
  // Backspace/Enter where some browsers don't fire beforeinput.
  editor.addEventListener('keydown', (e) => {
    if (!enabled) return;
    if (e.ctrlKey || e.metaKey) return;

    if (e.key === 'Backspace') {
      e.preventDefault();
      handleBackspace();
      return;
    }

    if (e.key === 'Enter') {
      e.preventDefault();
      flushComposing();
      committed += '\n';
      syncEditor();
      return;
    }

    if (e.key === 'Tab') {
      e.preventDefault();
      flushComposing();
      committed += '\t';
      syncEditor();
      return;
    }

    if (e.key.length !== 1) return;

    e.preventDefault();
    feedString(e.key);
  });

  editor.addEventListener('cut', (e) => {
    if (!enabled) return;
    e.preventDefault();
    flushComposing();
    const start = editor.selectionStart;
    const end = editor.selectionEnd;
    const selected = committed.substring(start, end);
    if (selected) {
      e.clipboardData.setData('text/plain', selected);
      committed = committed.substring(0, start) + committed.substring(end);
    }
    syncEditor();
  });

  editor.addEventListener('paste', (e) => {
    if (!enabled) return;
    e.preventDefault();
    flushComposing();
    const text = e.clipboardData.getData('text');
    committed += text;
    syncEditor();
  });

  // Safety net: if the browser modifies the textarea through a path we don't
  // handle (autocomplete, autofill, etc.), resync to our known state.
  // Skip during composition so the browser can manage the composing text.
  editor.addEventListener('input', () => {
    if (!enabled || composing) return;
    syncEditor();
  });
}

main();
