/** Node / CLI 默认面：只导出 native。Wasm 请走 `@game-gpt/red-alert2/wasm`。 */
export type { LaunchOptions, NativeBinaryIdentity, NativeBinding } from './native.js';
export { launch, loadNative, nativeBinaryIdentity, version } from './native.js';
