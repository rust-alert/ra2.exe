/**
 * GitHub Actions：发布真实 npm 包（非占位 stub）。
 *
 * - tag vX.Y.Z → version X.Y.Z
 * - 幂等：registry 已有同版本则跳过
 * - 不用 NPM_TOKEN；OIDC Trusted Publisher（permissions.id-token: write）
 * - 契约：file=publish-npm.yml env=NPM_PUBLISH repo=rust-alert/ra2.exe
 *
 * 前置：全部 native artifact 已在 RA2_NATIVE_ARTIFACTS（见 publish-npm.yml）。
 * 顺序：先平台包，再 wasm，最后 `@game-gpt/red-alert2`。
 */

import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

const NATIVE_PLATFORMS = [
    { short: 'win32-x64', triple: 'win32-x64-msvc', os: ['win32'], cpu: ['x64'] },
    { short: 'darwin-x64', triple: 'darwin-x64', os: ['darwin'], cpu: ['x64'] },
    { short: 'darwin-arm64', triple: 'darwin-arm64', os: ['darwin'], cpu: ['arm64'] },
    { short: 'linux-x64', triple: 'linux-x64-gnu', os: ['linux'], cpu: ['x64'] },
];

/** @type {{ dir: string, publishName: string }[]} */
const JS_PACKAGES = [
    { dir: 'projects/platforms/wasm/red-alert2-unknown-wasm32', publishName: '@game-gpt/red-alert2-unknown-wasm32' },
    { dir: 'projects/hosts/red-alert2', publishName: '@game-gpt/red-alert2' },
];

function fail(msg) {
    console.error(`ci-publish-npm: ${msg}`);
    process.exit(1);
}

function run(cmd, args, opts = {}) {
    const r = spawnSync(cmd, args, {
        cwd: opts.cwd ?? ROOT,
        encoding: 'utf8',
        shell: process.platform === 'win32',
        env: opts.env ?? process.env,
        stdio: opts.stdio ?? 'pipe',
    });
    return {
        status: r.status ?? 1,
        stdout: String(r.stdout ?? '').trim(),
        stderr: String(r.stderr ?? '').trim(),
    };
}

function resolveVersion() {
    const fromArg = process.argv.find((a) => a.startsWith('--version='))?.slice('--version='.length);
    if (fromArg) return fromArg.replace(/^v/, '');
    const ref = process.env.GITHUB_REF ?? '';
    const m = ref.match(/^refs\/tags\/v?(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)$/);
    if (m) return m[1];
    fail('need --version=X.Y.Z or GITHUB_REF=refs/tags/vX.Y.Z');
}

function readJson(p) {
    return JSON.parse(fs.readFileSync(p, 'utf8'));
}

function writeJson(p, obj) {
    fs.writeFileSync(p, `${JSON.stringify(obj, null, 4)}\n`);
}

function copyTree(src, dest, filter) {
    fs.mkdirSync(dest, { recursive: true });
    for (const name of fs.readdirSync(src)) {
        if (name === 'node_modules' || name === '.git') continue;
        const from = path.join(src, name);
        const to = path.join(dest, name);
        const st = fs.statSync(from);
        if (st.isDirectory()) {
            if (filter && !filter(from, true)) continue;
            copyTree(from, to, filter);
        } else {
            if (filter && !filter(from, false)) continue;
            fs.copyFileSync(from, to);
        }
    }
}

function rewriteWorkspaceDeps(deps, version) {
    if (!deps) return deps;
    /** @type {Record<string, string>} */
    const out = {};
    for (const [k, v] of Object.entries(deps)) {
        if (typeof v === 'string' && (v.startsWith('workspace:') || v === '*')) {
            out[k] = version;
        } else {
            out[k] = v;
        }
    }
    return out;
}

function rewriteDepsField(pkg, version) {
    for (const field of ['dependencies', 'optionalDependencies', 'peerDependencies']) {
        if (pkg[field]) pkg[field] = rewriteWorkspaceDeps(pkg[field], version);
    }
    return pkg;
}

function isAlreadyPublished(blob) {
    return /cannot publish over existing|EPUBLISHCONFLICT|previously published versions|version already exists|cannot publish.*same version|you cannot publish over/i.test(
        blob,
    );
}

function isAuthFailure(blob) {
    return /ENEEDAUTH|Unable to authenticate|not authorized|OIDC|trusted publisher|two-factor|need to be logged|login|identity token|do not have permission to access it|Access token expired or revoked/i.test(
        blob,
    );
}

function isMissingPackage(blob) {
    if (isAuthFailure(blob)) return false;
    return /Package not found|does not exist on the registry|cannot publish.*before creating|This package has not been created|is not in this registry/i.test(
        blob,
    );
}

function versionExists(name, version) {
    const r = run('npm', ['view', `${name}@${version}`, 'version']);
    return r.status === 0 && r.stdout === version;
}

function npmPublish(stagingDir, name, version) {
    const args = ['publish', '--access', 'public'];
    console.log(`\n=== ${name}@${version} npm ${args.join(' ')} ===`);
    const r = run('npm', args, { cwd: stagingDir });
    if (r.stdout) process.stdout.write(`${r.stdout}\n`);
    if (r.stderr) process.stderr.write(`${r.stderr}\n`);
    const blob = `${r.stdout}\n${r.stderr}`;
    if (r.status === 0) return 'published';
    if (isAlreadyPublished(blob) || versionExists(name, version)) return 'exists';
    if (isAuthFailure(blob)) return 'auth';
    if (isMissingPackage(blob)) return 'missing';
    if (versionExists(name, version)) return 'exists';
    console.error(blob.slice(0, 1200));
    return 'other';
}

function trustedPublisherHint() {
    return 'Add Trusted Publisher: file=publish-npm.yml env=NPM_PUBLISH repo=rust-alert/ra2.exe';
}

function publishNative(version, artifactsRoot) {
    let published = 0;
    let skipped = 0;
    for (const plat of NATIVE_PLATFORMS) {
        const name = `@game-gpt/red-alert2-${plat.short}`;
        const pkgDir = `projects/platforms/native/red-alert2-${plat.short}`;
        const pkgPath = path.join(ROOT, pkgDir, 'package.json');
        if (!fs.existsSync(pkgPath)) fail(`${name}: missing ${pkgDir}/package.json`);
        const raw = readJson(pkgPath);

        const artDir = path.join(artifactsRoot, plat.short);
        if (!fs.existsSync(artDir) || fs.readdirSync(artDir).length === 0) {
            fail(`${name}: missing artifact under ${artDir} — all natives required before publish`);
        }
        if (versionExists(name, version)) {
            console.log(` ✓ ${name}@${version} already on registry — skip`);
            skipped += 1;
            continue;
        }

        const stage = path.join(os.tmpdir(), `ra2-pub-native-${plat.short}-${version}`);
        fs.rmSync(stage, { recursive: true, force: true });
        fs.mkdirSync(stage, { recursive: true });
        for (const f of fs.readdirSync(artDir)) {
            fs.copyFileSync(path.join(artDir, f), path.join(stage, f));
        }

        const want = typeof raw.main === 'string' ? raw.main : `red-alert2.${plat.triple}.node`;
        const pkg = { ...raw };
        pkg.version = version;
        delete pkg.private;
        pkg.publishConfig = { ...(pkg.publishConfig ?? {}), access: 'public' };
        writeJson(path.join(stage, 'package.json'), pkg);

        for (const f of fs.readdirSync(stage)) {
            if (f.endsWith('.node') && f !== want) fs.unlinkSync(path.join(stage, f));
        }
        const plain = path.join(stage, 'red-alert2.node');
        if (!fs.existsSync(path.join(stage, want)) && fs.existsSync(plain)) {
            fs.renameSync(plain, path.join(stage, want));
        }
        if (!fs.existsSync(path.join(stage, want))) {
            fail(`${name}: staged artifact missing ${want}`);
        }

        const readmeSrc = path.join(ROOT, pkgDir, 'README.md');
        if (fs.existsSync(readmeSrc)) {
            fs.copyFileSync(readmeSrc, path.join(stage, 'README.md'));
        } else {
            fs.writeFileSync(
                path.join(stage, 'README.md'),
                `# ${name}\n\nOptional native binary for \`@game-gpt/red-alert2\` (${plat.short} / ${plat.triple}).\n`,
            );
        }

        const outcome = npmPublish(stage, name, version);
        if (outcome === 'published') published += 1;
        else if (outcome === 'exists') {
            console.log(` ✓ ${name}@${version} already on registry — skip`);
            skipped += 1;
        } else if (outcome === 'auth') {
            fail(`OIDC/auth failed for ${name}. ${trustedPublisherHint()}`);
        } else fail(`publish failed for ${name}`);
    }
    return { published, skipped };
}

function publishJs(version) {
    let published = 0;
    let skipped = 0;
    const optionalPlatforms = Object.fromEntries([
        ...NATIVE_PLATFORMS.map((p) => [`@game-gpt/red-alert2-${p.short}`, version]),
        ['@game-gpt/red-alert2-unknown-wasm32', version],
    ]);

    for (const spec of JS_PACKAGES) {
        const abs = path.join(ROOT, spec.dir);
        if (!fs.existsSync(abs)) fail(`missing package dir ${spec.dir}`);
        const raw = readJson(path.join(abs, 'package.json'));
        const name = spec.publishName ?? raw.name;
        if (!name) fail(`no name for ${spec.dir}`);

        if (versionExists(name, version)) {
            console.log(` ✓ ${name}@${version} already on registry — skip`);
            skipped += 1;
            continue;
        }

        const stage = path.join(os.tmpdir(), `ra2-pub-js-${name.replace(/[/@]/g, '-')}-${version}`);
        fs.rmSync(stage, { recursive: true, force: true });
        fs.mkdirSync(stage, { recursive: true });

        const files = Array.isArray(raw.files) && raw.files.length ? raw.files : null;
        if (files) {
            for (const f of files) {
                const from = path.join(abs, f);
                if (!fs.existsSync(from)) continue;
                const st = fs.statSync(from);
                const to = path.join(stage, f);
                if (st.isDirectory()) copyTree(from, to);
                else {
                    fs.mkdirSync(path.dirname(to), { recursive: true });
                    fs.copyFileSync(from, to);
                }
            }
            for (const extra of ['package.json', 'README.md', 'LICENSE', 'bin']) {
                const from = path.join(abs, extra);
                if (!fs.existsSync(from)) continue;
                const to = path.join(stage, extra);
                if (fs.statSync(from).isDirectory()) copyTree(from, to);
                else fs.copyFileSync(from, to);
            }
        } else {
            copyTree(abs, stage, (p) => {
                const rel = path.relative(abs, p);
                if (rel.includes('node_modules') || rel.includes('tests') || rel.endsWith('.node')) return false;
                return true;
            });
        }

        const pkg = rewriteDepsField({ ...raw }, version);
        pkg.name = name;
        pkg.version = version;
        delete pkg.private;
        pkg.publishConfig = { ...(pkg.publishConfig ?? {}), access: 'public' };

        if (name === '@game-gpt/red-alert2-unknown-wasm32') {
            const wasm = path.join(stage, 'pkg/ra_wasm_bg.wasm');
            const entry = path.join(stage, 'dist/index.js');
            if (!fs.existsSync(wasm)) {
                fail(`${name}: missing pkg/ra_wasm_bg.wasm — run pnpm build:wasm before publish`);
            }
            if (!fs.existsSync(entry)) {
                fail(`${name}: missing dist/index.js — run pnpm build:ts before publish`);
            }
        }

        if (name === '@game-gpt/red-alert2') {
            pkg.optionalDependencies = { ...(pkg.optionalDependencies ?? {}), ...optionalPlatforms };
            for (const f of fs.readdirSync(stage)) {
                if (f.endsWith('.node')) fs.unlinkSync(path.join(stage, f));
            }
        }

        if (pkg.scripts) {
            delete pkg.scripts.prepack;
            delete pkg.scripts.prepare;
            if (Object.keys(pkg.scripts).length === 0) delete pkg.scripts;
        }
        delete pkg.devDependencies;
        writeJson(path.join(stage, 'package.json'), pkg);

        if (!fs.existsSync(path.join(stage, 'README.md'))) {
            fs.writeFileSync(path.join(stage, 'README.md'), `# ${name}\n\nRed Alert 2 package ${version}.\n`);
        }

        const outcome = npmPublish(stage, name, version);
        if (outcome === 'published') published += 1;
        else if (outcome === 'exists') {
            console.log(` ✓ ${name}@${version} already on registry — skip`);
            skipped += 1;
        } else if (outcome === 'auth') {
            fail(`OIDC/auth failed for ${name}. ${trustedPublisherHint()}`);
        } else if (outcome === 'missing') {
            fail(`${name} is not on the registry yet. Create the scoped package / Trusted Publisher binding, then retry.`);
        } else fail(`publish failed for ${name}`);
    }
    return { published, skipped };
}

const version = resolveVersion();
console.log(`ci-publish-npm: version=${version}`);
console.log(` GITHUB_REF=${process.env.GITHUB_REF ?? '(none)'}`);
console.log(' Trusted Publisher contract: publish-npm.yml + env NPM_PUBLISH + repo rust-alert/ra2.exe\n');

delete process.env.NODE_AUTH_TOKEN;
delete process.env.NPM_TOKEN;

const artifactsRoot = process.env.RA2_NATIVE_ARTIFACTS || path.join(ROOT, 'dist', 'native-flat');

const native = publishNative(version, artifactsRoot);
const js = publishJs(version);

console.log(
    `\nci-publish-npm: done (native published=${native.published} skipped=${native.skipped}; js published=${js.published} skipped=${js.skipped})`,
);
