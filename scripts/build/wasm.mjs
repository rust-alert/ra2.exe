/**
 * 构建 `ra-wasm` → `@game-gpt/red-alert2-unknown-wasm32`（与 `napi.mjs` 对等的 Wasm 管线）：
 *   pkg/  — wasm-pack 产出（`.wasm` + glue）
 *   dist/ — 手写 `src/` 的 tsc 输出
 *
 * Usage: node scripts/build/wasm.mjs [--release]
 *
 * `wasm-pack` 的 `--out-dir` 相对 crate 目录解析。
 */

import { spawnSync } from 'node:child_process';
import {
    existsSync,
    mkdirSync,
    readdirSync,
    readFileSync,
    rmSync,
    unlinkSync,
    writeFileSync,
} from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const release = process.argv.includes('--release');
const crateDir = path.join(root, 'projects', 'bindings', 'ra-wasm');
const outPkg = path.join(root, 'projects', 'platforms', 'wasm', 'red-alert2-unknown-wasm32');
const pkgDir = path.join(outPkg, 'pkg');
const distDir = path.join(outPkg, 'dist');
const outRelFromCrate = path.relative(crateDir, pkgDir);

function run(cmd, args, cwd = root) {
    const r = spawnSync(cmd, args, { cwd, stdio: 'inherit', shell: false, env: process.env });
    if (r.error) {
        console.error(r.error);
        process.exit(1);
    }
    if (r.status !== 0) process.exit(r.status ?? 1);
}

mkdirSync(path.dirname(pkgDir), { recursive: true });
rmSync(pkgDir, { recursive: true, force: true });

const wasmPackArgs = ['build', '.', '--target', 'web', '--out-dir', outRelFromCrate, '--out-name', 'ra_wasm'];
if (release) wasmPackArgs.push('--release');

console.log(`wasm-pack ${wasmPackArgs.join(' ')} (cwd=${crateDir})`);
run('wasm-pack', wasmPackArgs, crateDir);

for (const name of ['package.json', '.gitignore']) {
    const p = path.join(pkgDir, name);
    if (existsSync(p)) unlinkSync(p);
}

const pkgJsonPath = path.join(outPkg, 'package.json');
const pkgName = '@game-gpt/red-alert2-unknown-wasm32';
const repoDir = 'projects/platforms/wasm/red-alert2-unknown-wasm32';
if (!existsSync(pkgJsonPath)) {
    writeFileSync(
        pkgJsonPath,
        `${JSON.stringify(
            {
                name: pkgName,
                version: '0.0.0',
                private: true,
                type: 'module',
                description:
                    'Wasm-pack artifact from `ra-wasm` (WebGL2-oriented). Consumed via `@game-gpt/red-alert2`. Not a standalone product API.',
                repository: {
                    type: 'git',
                    url: 'https://github.com/rust-alert/ra2.exe.git',
                    directory: repoDir,
                },
                homepage: `https://github.com/rust-alert/ra2.exe/tree/master/${repoDir}`,
                bugs: { url: 'https://github.com/rust-alert/ra2.exe/issues' },
                keywords: ['red-alert-2', 'ra2', 'wasm', 'wasm-pack', 'webgl2'],
                license: 'Apache-2.0',
                main: './dist/index.js',
                types: './dist/index.d.ts',
                exports: {
                    '.': {
                        types: './dist/index.d.ts',
                        default: './dist/index.js',
                    },
                },
                files: ['dist', 'pkg', 'README.md'],
                engines: { node: '>=20' },
                scripts: { build: 'tsc -p tsconfig.json' },
                devDependencies: { typescript: '^5.8.3' },
            },
            null,
            4,
        )}\n`,
    );
} else {
    try {
        // 只保证发布入口字段，保留 description / repository 等元数据。
        const pkg = JSON.parse(readFileSync(pkgJsonPath, 'utf8'));
        pkg.name = pkgName;
        pkg.private = true;
        pkg.type = 'module';
        pkg.main = './dist/index.js';
        pkg.types = './dist/index.d.ts';
        pkg.files = ['dist', 'pkg', 'README.md'];
        if (!pkg.license) pkg.license = 'Apache-2.0';
        if (!pkg.engines) pkg.engines = { node: '>=20' };
        writeFileSync(pkgJsonPath, `${JSON.stringify(pkg, null, 4)}\n`);
    } catch {
        /* ignore */
    }
}

console.log(`wasm pkg ← ${pkgDir}`);
if (existsSync(pkgDir)) {
    for (const name of readdirSync(pkgDir)) console.log(`  pkg/${name}`);
}

rmSync(distDir, { recursive: true, force: true });
const tscJs = path.join(root, 'node_modules', 'typescript', 'bin', 'tsc');
const tscArgs = existsSync(tscJs)
    ? [tscJs, '-p', 'tsconfig.json']
    : ['--package=typescript', 'tsc', '-p', 'tsconfig.json'];
const tscCmd = existsSync(tscJs) ? process.execPath : process.platform === 'win32' ? 'npx.cmd' : 'npx';
console.log(`tsc -p tsconfig.json (cwd=${outPkg})`);
run(tscCmd, tscArgs, outPkg);

console.log(`wasm dist ← ${distDir}`);
if (existsSync(distDir)) {
    for (const name of readdirSync(distDir)) console.log(`  dist/${name}`);
}
