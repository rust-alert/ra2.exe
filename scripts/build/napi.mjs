/**
 * 构建 `ra-napi` → 当前平台 `@game-gpt/red-alert2-<short>`（与 `wasm.mjs` 对等的 native 管线）：
 *   编译 `ra-napi`，把 `.node` 装进 `projects/platforms/native/red-alert2-<short>/`。
 *
 * Usage: node scripts/build/napi.mjs [--release]
 */

import { spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, unlinkSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const release = process.argv.includes('--release');
const profile = release ? 'release' : 'debug';

/** @returns {{ triple: string, short: string, os: string[], cpu: string[] }} */
function platformInfo() {
    const { platform, arch } = process;
    if (platform === 'win32' && arch === 'x64') {
        return { triple: 'win32-x64-msvc', short: 'win32-x64', os: ['win32'], cpu: ['x64'] };
    }
    if (platform === 'darwin' && arch === 'arm64') {
        return { triple: 'darwin-arm64', short: 'darwin-arm64', os: ['darwin'], cpu: ['arm64'] };
    }
    if (platform === 'darwin' && arch === 'x64') {
        return { triple: 'darwin-x64', short: 'darwin-x64', os: ['darwin'], cpu: ['x64'] };
    }
    if (platform === 'linux' && arch === 'x64') {
        return { triple: 'linux-x64-gnu', short: 'linux-x64', os: ['linux'], cpu: ['x64'] };
    }
    const triple = `${platform}-${arch}`;
    return { triple, short: triple, os: [platform], cpu: [arch] };
}

const cargoArgs = ['build', '--manifest-path', path.join(root, 'Cargo.toml'), '-p', 'ra-napi'];
if (release) cargoArgs.push('--release');

console.log(`cargo ${cargoArgs.join(' ')}`);
const build = spawnSync('cargo', cargoArgs, { cwd: root, stdio: 'inherit' });
if (build.status !== 0) {
    process.exit(build.status ?? 1);
}

const targetDir = path.join(root, 'target', profile);
const stem = 'ra_napi';
const candidates = [];
if (process.platform === 'win32') {
    candidates.push(`${stem}.dll`, `${stem}.node`);
} else if (process.platform === 'darwin') {
    candidates.push(`lib${stem}.dylib`, `${stem}.dylib`, `lib${stem}.so`, `${stem}.so`, `${stem}.node`);
} else {
    candidates.push(`lib${stem}.so`, `${stem}.so`, `${stem}.node`);
}

let artifact = null;
for (const name of candidates) {
    const p = path.join(targetDir, name);
    if (existsSync(p)) {
        artifact = p;
        break;
    }
}

if (!artifact) {
    const deps = path.join(targetDir, 'deps');
    if (existsSync(deps)) {
        for (const name of readdirSync(deps)) {
            const base = name.replace(/^lib/, '');
            if (
                (name === stem || name.startsWith(`${stem}.`) || base.startsWith(`${stem}.`) || name.startsWith(`lib${stem}.`)) &&
                (name.endsWith('.dll') || name.endsWith('.so') || name.endsWith('.dylib') || name.endsWith('.node'))
            ) {
                artifact = path.join(deps, name);
                break;
            }
        }
    }
}

if (!artifact) {
    console.error(`Could not find lib${stem} / ${stem} .{dll,so,dylib,node} under ${targetDir}`);
    process.exit(1);
}

const plat = platformInfo();
const outDir = path.join(root, 'projects', 'platforms', 'native', `red-alert2-${plat.short}`);
mkdirSync(outDir, { recursive: true });

const pkgJsonPath = path.join(outDir, 'package.json');
const binaryName = `red-alert2.${plat.triple}.node`;
const pkgName = `@game-gpt/red-alert2-${plat.short}`;
const repoDir = `projects/platforms/native/red-alert2-${plat.short}`;
if (!existsSync(pkgJsonPath)) {
    writeFileSync(
        pkgJsonPath,
        `${JSON.stringify(
            {
                name: pkgName,
                version: '0.0.0',
                private: true,
                description: `Prebuilt Node-API addon for \`@game-gpt/red-alert2\` (${plat.short}). Platform artifact only.`,
                repository: {
                    type: 'git',
                    url: 'https://github.com/rust-alert/ra2.exe.git',
                    directory: repoDir,
                },
                homepage: `https://github.com/rust-alert/ra2.exe/tree/master/${repoDir}`,
                bugs: {
                    url: 'https://github.com/rust-alert/ra2.exe/issues',
                },
                keywords: ['red-alert-2', 'ra2', 'napi', 'native-addon', ...plat.os, ...plat.cpu],
                license: 'Apache-2.0',
                os: plat.os,
                cpu: plat.cpu,
                main: binaryName,
                files: [binaryName, 'README.md'],
                engines: { node: '>=20' },
            },
            null,
            4,
        )}\n`,
    );
} else {
    try {
        // 只同步构建相关字段，保留 description / repository / homepage 等发布元数据。
        const pkg = JSON.parse(readFileSync(pkgJsonPath, 'utf8'));
        pkg.name = pkgName;
        pkg.private = true;
        pkg.os = plat.os;
        pkg.cpu = plat.cpu;
        pkg.main = binaryName;
        pkg.files = [binaryName, 'README.md'];
        if (!pkg.license) pkg.license = 'Apache-2.0';
        if (!pkg.engines) pkg.engines = { node: '>=20' };
        writeFileSync(pkgJsonPath, `${JSON.stringify(pkg, null, 4)}\n`);
    } catch {
        /* ignore */
    }
}

const destNamed = path.join(outDir, binaryName);
copyFileSync(artifact, destNamed);
const destPlain = path.join(outDir, 'red-alert2.node');
if (existsSync(destPlain)) {
    try {
        unlinkSync(destPlain);
    } catch {
        /* ignore */
    }
}
console.log(`Copied ${artifact}`);
console.log(` → ${destNamed}`);
console.log(`Platform package: @game-gpt/red-alert2-${plat.short} (${outDir})`);
