/* tslint:disable */
/* eslint-disable */

/**
 * Compresses multiple entries into a 7z archive in WebAssembly environment.
 *
 * This function creates a compressed archive from multiple file entries,
 * designed specifically for WASM targets.
 *
 * # Arguments
 * * `entries` - Vector of JavaScript strings representing file names/paths
 * * `datas` - Vector of Uint8Arrays containing the file data corresponding to entries
 */
export function compress(entries: string[], datas: Uint8Array[]): Uint8Array;

/**
 * Decompresses a 7z archive in WebAssembly environment.
 *
 * This function is specifically designed for WASM targets and uses JavaScript interop
 * to handle the decompression process with a callback function.
 *
 * # Arguments
 * * `src` - Uint8Array containing the compressed archive data
 * * `pwd` - Password string for encrypted archives (use empty string for unencrypted)
 * * `f` - JavaScript callback function to handle extracted entries
 */
export function decompress(src: Uint8Array, pwd: string, f: Function): void;

/**
 * Fetches a URL through the browser and returns its response bytes.
 */
export function fetch_file_from_server(url: string): Promise<Uint8Array>;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly main: (a: number, b: number) => number;
    readonly compress: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly decompress: (a: any, b: number, c: number, d: any) => [number, number];
    readonly fetch_file_from_server: (a: number, b: number) => any;
    readonly rust_zstd_wasm_shim_calloc: (a: number, b: number) => number;
    readonly rust_zstd_wasm_shim_free: (a: number) => void;
    readonly rust_zstd_wasm_shim_malloc: (a: number) => number;
    readonly rust_zstd_wasm_shim_memcmp: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_memcpy: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_memmove: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_memset: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_qsort: (a: number, b: number, c: number, d: number) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___bool__core_ed718c3d60ebd546___result__Result_____wasm_bindgen_527d7e65d48076c5___JsError___true_: (a: number, b: number, c: number) => [number, number];
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___js_sys_c5449e363cdae367___Array__web_sys_3ff2519d89fb937___features__gen_ResizeObserver__ResizeObserver______true_: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___js_sys_c5449e363cdae367___Function_fn_wasm_bindgen_527d7e65d48076c5___JsValue_____wasm_bindgen_527d7e65d48076c5___sys__Undefined___js_sys_c5449e363cdae367___Function_fn_wasm_bindgen_527d7e65d48076c5___JsValue_____wasm_bindgen_527d7e65d48076c5___sys__Undefined_______true_: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___wasm_bindgen_527d7e65d48076c5___JsValue__core_ed718c3d60ebd546___result__Result_____wasm_bindgen_527d7e65d48076c5___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___core_ed718c3d60ebd546___option__Option_web_sys_3ff2519d89fb937___features__gen_Blob__Blob_______true_: (a: number, b: number, c: number) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___wasm_bindgen_527d7e65d48076c5___JsValue______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___wasm_bindgen_527d7e65d48076c5___JsValue______true__1_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___wasm_bindgen_527d7e65d48076c5___JsValue______true__1__12: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___wasm_bindgen_527d7e65d48076c5___JsValue______true__1__13: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___wasm_bindgen_527d7e65d48076c5___JsValue______true__1__15: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___wasm_bindgen_527d7e65d48076c5___JsValue______true__1__16: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___wasm_bindgen_527d7e65d48076c5___JsValue______true__1__17: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___wasm_bindgen_527d7e65d48076c5___JsValue______true__1__19: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___wasm_bindgen_527d7e65d48076c5___JsValue______true__1__9: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___web_sys_3ff2519d89fb937___features__gen_InputEvent__InputEvent______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___web_sys_3ff2519d89fb937___features__gen_InputEvent__InputEvent______true__11: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___web_sys_3ff2519d89fb937___features__gen_InputEvent__InputEvent______true__14: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke___web_sys_3ff2519d89fb937___features__gen_InputEvent__InputEvent______true__18: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_527d7e65d48076c5___convert__closures_____invoke_______true_: (a: number, b: number) => void;
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
