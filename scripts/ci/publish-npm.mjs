/**
 * Publish npm packages (requires NPM_TOKEN or NODE_AUTH_TOKEN).
 *
 * Builds Wasm pkg first. Usage: node scripts/ci/publish-npm.mjs
 */

import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

const packages = [
    'projects/hosts/red-alert2',
    'projects/platforms/native/red-alert2-win32-x64',
    'projects/platforms/native/red-alert2-linux-x64',
    'projects/platforms/native/red-alert2-darwin-arm64',
    'projects/platforms/native/red-alert2-darwin-x64',
    'projects/platforms/wasm/red-alert2-unknown-wasm32',
];

if (!process.env.NPM_TOKEN && !process.env.NODE_AUTH_TOKEN) {
    console.error('publish requires NPM_TOKEN or NODE_AUTH_TOKEN');
    process.exit(1);
}

const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const pnpm = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm';
const node = process.execPath;

function run(cmd, args, cwd = root) {
    const r = spawnSync(cmd, args, { cwd, stdio: 'inherit', shell: false, env: process.env });
    if (r.status !== 0) process.exit(r.status ?? 1);
}

run(node, ['scripts/build/wasm.mjs', '--release']);
run(pnpm, ['run', 'build:ts']);

for (const rel of packages) {
    const abs = path.join(root, rel);
    if (!fs.existsSync(path.join(abs, 'package.json'))) continue;
    console.log(`publish ${rel}`);
    run(npm, ['publish', '--access', 'public'], abs);
}
