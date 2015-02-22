/** Browser face over `ra-wasm`（`pkg/` 由 `scripts/build/wasm.mjs` 生成）。 */

import initWasm, {
    InstallSession,
    PrepareReport,
    engine_name,
    supports_webgl2,
    version as wasmVersion,
} from '../pkg/ra_wasm.js';

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export { InstallSession, PrepareReport };

/** 加载 `.wasm`（页面入口先 `await init()`）。 */
export async function init(moduleOrPath?: InitInput): Promise<void> {
    await initWasm(moduleOrPath);
}

/** 绑定版本（须先 `init`；与 native `version` 对齐）。 */
export function version(): string {
    return wasmVersion();
}

/** 引擎显示名（须先 `init`）。 */
export function engineName(): string {
    return engine_name();
}

/**
 * wgpu 浏览器呈现路径是否已接线（须先 `init`）。
 * 名称保留 WebGL2：当前 wgpu web 后端多为 WebGL2，不是手写 GL 绑定。
 */
export function supportsWebgl2(): boolean {
    return supports_webgl2();
}
