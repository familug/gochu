import init, { Gochu } from './pkg/gochu_wasm.js';

async function main() {
  await init();

  const gochu = new Gochu();
  const editor = document.getElementById('editor');
  const preedit = document.getElementById('preedit');
  const modeToggle = document.getElementById('mode');

  let enabled = true;
  let committed = '';

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

  editor.addEventListener('keydown', (e) => {
    if (!enabled) return;
    if (e.ctrlKey || e.metaKey) return;

    if (e.key === 'Backspace') {
      e.preventDefault();
      if (gochu.is_composing()) {
        gochu.process_key('\x08');
        updatePreedit();
        syncEditor();
      } else if (committed.length > 0) {
        const chars = [...committed];
        chars.pop();
        committed = chars.join('');
        syncEditor();
      }
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
    const result = gochu.process_key(e.key);

    if (result.type === 'commit') {
      committed += result.text;
      gochu.reset();
    }

    updatePreedit();
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

  editor.addEventListener('input', (e) => {
    if (!enabled) return;
    e.preventDefault();
    editor.value = committed + gochu.get_display();
  });
}

main();
