/**
 * Build `ra-wasm` with wasm-pack into `platforms/wasm/red-alert2-unknown-wasm32/pkg`.
 *
 * Usage: node scripts/build/wasm.mjs [--release]
 *
 * `wasm-pack` resolves `--out-dir` relative to the crate directory.
 */

import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const release = process.argv.includes('--release');
const crateDir = path.join(root, 'projects', 'bindings', 'ra-wasm');
const outAbs = path.join(root, 'projects', 'platforms', 'wasm', 'red-alert2-unknown-wasm32', 'pkg');
const outRelFromCrate = path.relative(crateDir, outAbs);

fs.mkdirSync(path.dirname(outAbs), { recursive: true });
fs.rmSync(outAbs, { recursive: true, force: true });

const args = ['build', '.', '--target', 'web', '--out-dir', outRelFromCrate, '--out-name', 'ra_wasm'];
if (release) args.push('--release');

console.log(`wasm-pack ${args.join(' ')} (cwd=${crateDir})`);
const r = spawnSync('wasm-pack', args, {
    cwd: crateDir,
    stdio: 'inherit',
    shell: false,
    env: process.env,
});
if (r.status !== 0) process.exit(r.status ?? 1);

for (const name of ['package.json', '.gitignore']) {
    const p = path.join(outAbs, name);
    if (fs.existsSync(p)) fs.unlinkSync(p);
}
console.log(`wasm pkg → ${outAbs}`);
