import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const ALL_LOCALES = [
  "ar", "zh", "nl", "en", "fr", "de", "hi", "it", "ja", "ko", "pl", "pt", "ru", "es", "tr"
];

const localesDir = new URL('../src/lib/i18n/locales/', import.meta.url);
const it = JSON.parse(readFileSync(new URL('it.json', localesDir), 'utf8'));
const en = JSON.parse(readFileSync(new URL('en.json', localesDir), 'utf8'));

test('it.json and en.json have identical key sets', () => {
  const itKeys = Object.keys(it).sort();
  const enKeys = Object.keys(en).sort();
  assert.deepEqual(itKeys, enKeys, 'Localization keys in it.json and en.json must match exactly');
});

test('all 15 locales exist and have identical key sets matching en.json', () => {
  const enKeys = Object.keys(en).sort();
  for (const lang of ALL_LOCALES) {
    const data = JSON.parse(readFileSync(new URL(`${lang}.json`, localesDir), 'utf8'));
    const keys = Object.keys(data).sort();
    assert.deepEqual(keys, enKeys, `Localization keys in ${lang}.json must match en.json exactly`);
  }
});

test('placeholders match en.json across all 15 locales', () => {
  const phRegex = /\{\{?(\w+)\}?\}/g;
  const getPh = (s) => [...String(s).matchAll(phRegex)].map((m) => m[1]).sort().join(',');

  for (const lang of ALL_LOCALES) {
    const data = JSON.parse(readFileSync(new URL(`${lang}.json`, localesDir), 'utf8'));
    for (const [key, refVal] of Object.entries(en)) {
      const targetVal = data[key];
      assert.equal(
        getPh(targetVal),
        getPh(refVal),
        `Placeholder mismatch in ${lang}.json for key "${key}" (en: "${refVal}" vs ${lang}: "${targetVal}")`
      );
    }
  }
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
