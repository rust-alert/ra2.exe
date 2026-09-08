import { existsSync, statSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

const require = createRequire(import.meta.url);

export type LaunchOptions = {
    path: string;
    edition?: string;
};

export type NativeBinding = {
    version(): string;
    launch(options: LaunchOptions): void;
};

export type NativeBinaryIdentity = {
    path: string;
    packageName: string;
    triple: string;
    version: string;
    size: number;
    mtimeMs: number;
};

function platformPackage(): { name: string; triple: string } {
    const { platform, arch } = process;
    if (platform === 'win32' && arch === 'x64') {
        return { name: '@game-gpt/red-alert2-win32-x64', triple: 'win32-x64-msvc' };
    }
    if (platform === 'darwin' && arch === 'arm64') {
        return { name: '@game-gpt/red-alert2-darwin-arm64', triple: 'darwin-arm64' };
    }
    if (platform === 'darwin' && arch === 'x64') {
        return { name: '@game-gpt/red-alert2-darwin-x64', triple: 'darwin-x64' };
    }
    if (platform === 'linux' && arch === 'x64') {
        return { name: '@game-gpt/red-alert2-linux-x64', triple: 'linux-x64-gnu' };
    }
    throw new Error(`unsupported platform ${platform}-${arch}`);
}

let cached: NativeBinding | null = null;
let cachedIdentity: NativeBinaryIdentity | null = null;

function resolveBinaryPath(): { name: string; triple: string; binary: string } {
    const { name, triple } = platformPackage();
    const pkgJson = require.resolve(`${name}/package.json`);
    const dir = path.dirname(pkgJson);
    const binary = path.join(dir, `red-alert2.${triple}.node`);
    if (!existsSync(binary)) {
        throw new Error(`native addon missing: ${binary} (run pnpm run build:native)`);
    }
    return { name, triple, binary };
}

/** 加载当前平台 N-API 插件（缓存）。 */
export function loadNative(): NativeBinding {
    if (cached) return cached;
    const { binary } = resolveBinaryPath();
    cached = require(binary) as NativeBinding;
    return cached;
}

/** 已加载 addon 的路径与指纹。 */
export function nativeBinaryIdentity(): NativeBinaryIdentity {
    if (cachedIdentity) return cachedIdentity;
    const { name, triple, binary } = resolveBinaryPath();
    const binding = loadNative();
    const st = statSync(binary);
    cachedIdentity = {
        path: binary,
        packageName: name,
        triple,
        version: binding.version(),
        size: st.size,
        mtimeMs: st.mtimeMs,
    };
    return cachedIdentity;
}

export function launch(options: LaunchOptions): void {
    loadNative().launch(options);
}

export function version(): string {
    return loadNative().version();
}
