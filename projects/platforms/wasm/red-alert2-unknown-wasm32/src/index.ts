/** Loader face for `ra-wasm` artifacts. Full wasm-bindgen pkg lands under `pkg/` later. */
export function engineName(): string {
    return 'ra2';
}

export function supportsWebgl2(): boolean {
    return false;
}
