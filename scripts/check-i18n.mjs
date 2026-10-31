#!/usr/bin/env node
/**
 * i18n consistency checker for Pokko.
 *
 * Checks:
 * 1. Parity between en.json (reference) and all other locale files.
 * 2. Placeholder consistency (e.g. {{count}}, {{page}}, {{url}}).
 * 3. Usage of translation keys in src/ against en.json.
 * 4. Whitespace and invisible character hygiene.
 */

import { readFileSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const localesDir = join(root, "src", "lib", "i18n", "locales");
const srcDir = join(root, "src");

const locales = readdirSync(localesDir).filter((f) => f.endsWith(".json"));
const refPath = join(localesDir, "en.json");
const reference = JSON.parse(readFileSync(refPath, "utf8"));
const refKeys = new Set(Object.keys(reference));

let errors = 0;
let warnings = 0;
const err = (msg) => { console.error(`  ✗ ${msg}`); errors++; };
const warn = (msg) => { console.warn(`  ⚠ ${msg}`); warnings++; };

const placeholders = (s) => [...String(s).matchAll(/\{\{(\w+)\}\}/g)].map((m) => m[1]).sort().join(",");

console.log(`Reference (en.json): ${refKeys.size} keys\n`);

// 1. Check all locale files
for (const file of locales) {
  const lang = file.replace(".json", "");
  const data = JSON.parse(readFileSync(join(localesDir, file), "utf8"));
  const keys = new Set(Object.keys(data));

  console.log(`Checking locale: ${lang} (${keys.size} keys)`);
  for (const k of refKeys) {
    if (!keys.has(k)) err(`${lang}: missing key "${k}"`);
  }
  for (const k of keys) {
    if (!refKeys.has(k)) err(`${lang}: extra key "${k}" (not in en.json)`);
  }
  for (const k of keys) {
    if (refKeys.has(k) && placeholders(data[k]) !== placeholders(reference[k])) {
      err(`${lang}: placeholder mismatch in "${k}" (en: [${placeholders(reference[k])}] vs ${lang}: [${placeholders(data[k])}])`);
    }
  }

  // Hygiene checks
  const INVISIBLE = /[​‌‍﻿­⁠]/;
  for (const [k, v] of Object.entries(data)) {
    if (typeof v !== "string") continue;
    if (INVISIBLE.test(v)) err(`${lang}: invisible character in "${k}"`);
    // Allow single trailing space for badge prefix labels ending with ": "
    if (v.trimStart() !== v) err(`${lang}: leading whitespace in "${k}"`);
    if (v.trimEnd() !== v && !v.endsWith(": ")) err(`${lang}: trailing whitespace in "${k}"`);
  }
}

// 2. Scan source code for t("...") usages
function* walk(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (!["node_modules", "locales"].includes(entry.name)) yield* walk(p);
    } else if (/\.(svelte|ts|js)$/.test(entry.name)) {
      yield p;
    }
  }
}

console.log("\nScanning source code for translation keys…");
const usedKeys = new Map();
const keyRegex = /\bt\(\s*["']([a-zA-Z0-9_.-]+)["']/g;

for (const file of walk(srcDir)) {
  const text = readFileSync(file, "utf8");
  let match;
  while ((match = keyRegex.exec(text)) !== null) {
    const key = match[1];
    // Exclude common CSS / DOM false positives
    if (["a", "input", "style", "meta", "keydown", "-", "."].includes(key)) continue;
    if (!usedKeys.has(key)) usedKeys.set(key, file);
  }
}

for (const [key, file] of usedKeys) {
  if (!refKeys.has(key)) {
    warn(`key "${key}" used in ${file.replace(root + "/", "")} but missing from en.json`);
  }
}

console.log(`\ni18n check completed: ${errors} error(s), ${warnings} warning(s).`);
process.exit(errors > 0 ? 1 : 0);
