import { readFileSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
const version = process.argv[2];
if (!/^\d+\.\d+\.\d+(?:-[\w.-]+)?$/.test(version ?? '')) throw new Error('Invalid release version');
for (const path of ['package.json', 'package-lock.json', 'src-tauri/tauri.conf.json']) {
  const data = JSON.parse(readFileSync(path, 'utf8'));
  data.version = version;
  if (data.packages?.['']) data.packages[''].version = version;
  writeFileSync(path, JSON.stringify(data, null, 2) + '\n');
}
const path = 'src-tauri/Cargo.toml';
const manifest = readFileSync(path, 'utf8');
writeFileSync(path, manifest.replace(/(\[package\][\s\S]*?\nversion\s*=\s*)"[^"]+"/, `$1"${version}"`));
const lockPath = 'src-tauri/Cargo.lock';
const lock = readFileSync(lockPath, 'utf8');
const updated = lock.replace(/(\[\[package\]\]\nname = "pokko"\nversion = )"[^"]+"/, `$1"${version}"`);
if (updated === lock && !lock.includes(`name = "pokko"\nversion = "${version}"`)) throw new Error('Root package missing from Cargo.lock');
writeFileSync(lockPath, updated);
execFileSync('cargo', ['metadata', '--manifest-path', path, '--locked', '--offline', '--no-deps', '--format-version', '1'], { stdio: 'ignore' });
