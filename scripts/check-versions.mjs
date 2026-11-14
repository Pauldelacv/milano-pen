#!/usr/bin/env node
// Fails unless package.json, src-tauri/tauri.conf.json and the Cargo
// workspace agree on the app version (and, with --tag vX.Y.Z, match the tag).
import { readFileSync } from 'node:fs';

const read = (p) => readFileSync(new URL(`../${p}`, import.meta.url), 'utf8');
const pkg = JSON.parse(read('package.json')).version;
const tauri = JSON.parse(read('src-tauri/tauri.conf.json')).version;
const cargo = /\[workspace\.package\][^[]*?\nversion\s*=\s*"([^"]+)"/s.exec(read('Cargo.toml'))?.[1];

const versions = { 'package.json': pkg, 'tauri.conf.json': tauri, 'Cargo.toml': cargo };
const tagIndex = process.argv.indexOf('--tag');
if (tagIndex !== -1) versions['git tag'] = (process.argv[tagIndex + 1] ?? '').replace(/^v/, '');

const distinct = new Set(Object.values(versions));
if (distinct.size !== 1 || distinct.has(undefined) || distinct.has('')) {
  console.error('Version mismatch:');
  for (const [k, v] of Object.entries(versions)) console.error(`  ${k}: ${v}`);
  process.exit(1);
}
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(pkg)) {
  console.error(`Not a semantic version: ${pkg}`);
  process.exit(1);
}
console.log(`All versions are ${pkg}`);
