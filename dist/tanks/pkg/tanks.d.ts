/* tslint:disable */
/* eslint-disable */

export function add_tanks(): void;

export function load_glb_bytes(data: Uint8Array): void;

export function start(): void;

export function unlock_audio(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly add_tanks: () => void;
    readonly load_glb_bytes: (a: number, b: number) => void;
    readonly start: () => void;
    readonly unlock_audio: () => void;
    readonly __wasm_bindgen_func_elem_2279: (a: number, b: number, c: number, d: number) => void;
    readonly __wasm_bindgen_func_elem_3380: (a: number, b: number, c: number, d: number) => void;
    readonly __wasm_bindgen_func_elem_705: (a: number, b: number, c: number, d: number) => void;
    readonly __wasm_bindgen_func_elem_705_15: (a: number, b: number, c: number, d: number) => void;
    readonly __wasm_bindgen_func_elem_705_16: (a: number, b: number, c: number, d: number) => void;
    readonly __wasm_bindgen_func_elem_2275: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_2275_11: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_2275_12: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_2275_13: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_2275_14: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_2275_7: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_2275_8: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_2275_9: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_2287: (a: number, b: number) => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number) => void;
    readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export5: (a: number, b: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
