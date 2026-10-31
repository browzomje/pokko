import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const it = JSON.parse(readFileSync(new URL('../src/lib/i18n/locales/it.json', import.meta.url), 'utf8'));
const en = JSON.parse(readFileSync(new URL('../src/lib/i18n/locales/en.json', import.meta.url), 'utf8'));

test('it.json and en.json have identical key sets', () => {
  const itKeys = Object.keys(it).sort();
  const enKeys = Object.keys(en).sort();
  assert.deepEqual(itKeys, enKeys, 'Localization keys in it.json and en.json must match exactly');
});

test('en.json does not contain untranslated Italian words', () => {
  const commonItalianWords = [
    'scarica', 'capitoli', 'capitolo', 'ricerca', 'fonti', 'impostazioni',
    'lingua', 'salva', 'bozza', 'verifica', 'elementi', 'precedente',
    'successiva', 'trovati', 'attendere', 'connessione', 'pagina', 'pagine',
    'selezionati', 'chiudi', 'annulla', 'riprendi', 'pausa', 'destinazione',
    'intervallo', 'secondo', 'secondi', 'apri', 'esplora'
  ];

  for (const [key, value] of Object.entries(en)) {
    if (typeof value !== 'string') continue;
    // Tokenize into words
    const words = value.toLowerCase().replace(/[^a-zàèéìòù]/g, ' ').split(/\s+/).filter(Boolean);
    for (const itWord of commonItalianWords) {
      assert.ok(
        !words.includes(itWord),
        `Untranslated Italian word "${itWord}" found in en.json under key "${key}": "${value}"`
      );
    }
  }
});
