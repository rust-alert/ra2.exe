/** Browser / Wasm 面：经 `@game-gpt/red-alert2/wasm` 进入，不拖住 Node CLI。 */

export type { InitInput } from '@game-gpt/red-alert2-unknown-wasm32';
export {
    InstallSession,
    LoadProgress,
    PrepareReport,
    attachCanvas,
    engineName,
    init,
    loadJobBusy,
    loadJobProgress,
    presentBackend,
    presentFrame,
    resizePresent,
    supportsPresent,
    version,
} from '@game-gpt/red-alert2-unknown-wasm32';
