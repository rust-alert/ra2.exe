/** Browser face over `ra-wasm`（`pkg/` 由 `scripts/build/wasm.mjs` 生成）。 */

import initWasm, {
    InstallSession,
    LoadProgress,
    PrepareReport,
    attachCanvas,
    engine_name,
    loadJobBusy,
    loadJobProgress,
    presentBackend,
    resizePresent,
    supports_present,
    version as wasmVersion,
} from '../pkg/ra_wasm.js';

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export {
    InstallSession,
    LoadProgress,
    PrepareReport,
    attachCanvas,
    loadJobBusy,
    loadJobProgress,
    presentBackend,
    resizePresent,
};

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

/** wgpu 呈现路径是否已接线（须先 `init` 且成功 `attachCanvas`）。 */
export function supportsPresent(): boolean {
    return supports_present();
}
