/** Node / CLI 默认面：只导出 native。Wasm 请走 `@game-gpt/red-alert2/wasm`。 */
export type {
    DiagnoseMapsOptions,
    DiagnoseMapsReport,
    DiagnoseMobileVxlOptions,
    DiagnoseMobileVxlResult,
    ExtractedFile,
    ExtractOptions,
    ExtractResult,
    EmulateOptions,
    MapDiagnoseRow,
    MobileVxlDiagReport,
    MobileVxlLayerDiag,
    NativeBinaryIdentity,
    NativeBinding,
    UnpackOptions,
    UnpackResult,
} from './native.js';
export { diagnoseMaps, diagnoseMobileVxl, emulate, extract, loadNative, nativeBinaryIdentity, unpack, version } from './native.js';
