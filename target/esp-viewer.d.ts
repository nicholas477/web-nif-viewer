/* tslint:disable */
/* eslint-disable */

export function fetch_file_from_server(url: string): Promise<Uint8Array>;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly main: (a: number, b: number) => number;
    readonly fetch_file_from_server: (a: number, b: number) => any;
    readonly rust_zstd_wasm_shim_calloc: (a: number, b: number) => number;
    readonly rust_zstd_wasm_shim_free: (a: number) => void;
    readonly rust_zstd_wasm_shim_malloc: (a: number) => number;
    readonly rust_zstd_wasm_shim_memcmp: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_memcpy: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_memmove: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_memset: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_qsort: (a: number, b: number, c: number, d: number) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___wasm_bindgen_c05fd28b539a9326___JsValue__core_ed718c3d60ebd546___result__Result_____wasm_bindgen_c05fd28b539a9326___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___js_sys_d0a92db60a2c9c54___Array__web_sys_f7a5b628dd404f20___features__gen_ResizeObserver__ResizeObserver______true_: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___js_sys_d0a92db60a2c9c54___Function_fn_wasm_bindgen_c05fd28b539a9326___JsValue_____wasm_bindgen_c05fd28b539a9326___sys__Undefined___js_sys_d0a92db60a2c9c54___Function_fn_wasm_bindgen_c05fd28b539a9326___JsValue_____wasm_bindgen_c05fd28b539a9326___sys__Undefined_______true_: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___wasm_bindgen_c05fd28b539a9326___JsValue______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___wasm_bindgen_c05fd28b539a9326___JsValue______true__1_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___wasm_bindgen_c05fd28b539a9326___JsValue______true__1__4: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___web_sys_f7a5b628dd404f20___features__gen_InputEvent__InputEvent______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___web_sys_f7a5b628dd404f20___features__gen_InputEvent__InputEvent______true__6: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___wasm_bindgen_c05fd28b539a9326___JsValue______true__1__7: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___wasm_bindgen_c05fd28b539a9326___JsValue______true__1__8: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___web_sys_f7a5b628dd404f20___features__gen_InputEvent__InputEvent______true__9: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___wasm_bindgen_c05fd28b539a9326___JsValue______true__1__10: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___wasm_bindgen_c05fd28b539a9326___JsValue______true__1__11: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___wasm_bindgen_c05fd28b539a9326___JsValue______true__1__12: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___web_sys_f7a5b628dd404f20___features__gen_InputEvent__InputEvent______true__13: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___wasm_bindgen_c05fd28b539a9326___JsValue______true__1__14: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke___core_ed718c3d60ebd546___option__Option_web_sys_f7a5b628dd404f20___features__gen_Blob__Blob_______true_: (a: number, b: number, c: number) => void;
    readonly wasm_bindgen_c05fd28b539a9326___convert__closures_____invoke_______true_: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
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
