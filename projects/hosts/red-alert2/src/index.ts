/** Node / CLI 默认面：只导出 native。Wasm 请走 `@game-gpt/red-alert2/wasm`。 */
export type {
    DiagnoseMapsOptions,
    DiagnoseMapsReport,
    ExtractedFile,
    ExtractOptions,
    ExtractResult,
    EmulateOptions,
    MapDiagnoseRow,
    NativeBinaryIdentity,
    NativeBinding,
    UnpackOptions,
    UnpackResult,
} from './native.js';
export { diagnoseMaps, emulate, extract, loadNative, nativeBinaryIdentity, unpack, version } from './native.js';
