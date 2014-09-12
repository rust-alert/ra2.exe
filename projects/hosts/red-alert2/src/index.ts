/** Node / CLI 默认面：只导出 native。Wasm 请走 `@game-gpt/red-alert2/wasm`。 */
export type {
    ExtractedFile,
    ExtractOptions,
    ExtractResult,
    LaunchOptions,
    NativeBinaryIdentity,
    NativeBinding,
    UnpackOptions,
    UnpackResult,
} from './native.js';
export { extract, launch, loadNative, nativeBinaryIdentity, unpack, version } from './native.js';
