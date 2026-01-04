import { onRZXRunning, onTapeBlock, putImageData, putSoundData } from '../raze.js';

let wasm;

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

let WASM_VECTOR_LEN = 0;

/**
 * @param {number} size
 * @returns {number}
 */
export function wasm_alloc(size) {
    const ret = wasm.wasm_alloc(size);
    return ret >>> 0;
}

/**
 * @param {number} game
 * @param {boolean} turbo
 */
export function wasm_draw_frame(game, turbo) {
    wasm.wasm_draw_frame(game, turbo);
}

/**
 * @param {number} game
 */
export function wasm_drop(game) {
    wasm.wasm_drop(game);
}

/**
 * @param {number} game
 * @param {number} key
 */
export function wasm_key_down(game, key) {
    wasm.wasm_key_down(game, key);
}

/**
 * @param {number} game
 * @param {number} key
 */
export function wasm_key_up(game, key) {
    wasm.wasm_key_up(game, key);
}

/**
 * @param {number} game
 * @param {Uint8Array} data
 * @returns {boolean}
 */
export function wasm_load_snapshot(game, data) {
    const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.wasm_load_snapshot(game, ptr0, len0);
    return ret !== 0;
}

/**
 * @param {number} game
 * @param {Uint8Array} data
 * @returns {number}
 */
export function wasm_load_tape(game, data) {
    const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.wasm_load_tape(game, ptr0, len0);
    return ret >>> 0;
}

/**
 * @param {boolean} is128k
 * @returns {number}
 */
export function wasm_main(is128k) {
    const ret = wasm.wasm_main(is128k);
    return ret >>> 0;
}

/**
 * @param {number} game
 * @param {number} addr
 * @returns {number}
 */
export function wasm_peek(game, addr) {
    const ret = wasm.wasm_peek(game, addr);
    return ret;
}

/**
 * @param {number} game
 * @param {number} addr
 * @param {number} value
 */
export function wasm_poke(game, addr, value) {
    wasm.wasm_poke(game, addr, value);
}

/**
 * @param {number} game
 */
export function wasm_reset_input(game) {
    wasm.wasm_reset_input(game);
}

/**
 * @param {number} game
 * @returns {Uint8Array}
 */
export function wasm_snapshot(game) {
    const ret = wasm.wasm_snapshot(game);
    var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v1;
}

/**
 * @param {number} game
 */
export function wasm_stop_rzx_replay(game) {
    wasm.wasm_stop_rzx_replay(game);
}

/**
 * @param {number} game
 * @param {number} index
 * @returns {string}
 */
export function wasm_tape_name(game, index) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.wasm_tape_name(game, index);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * @param {number} game
 * @param {number} index
 */
export function wasm_tape_seek(game, index) {
    wasm.wasm_tape_seek(game, index);
}

/**
 * @param {number} game
 * @param {number} index
 * @returns {boolean}
 */
export function wasm_tape_selectable(game, index) {
    const ret = wasm.wasm_tape_selectable(game, index);
    return ret !== 0;
}

/**
 * @param {number} game
 */
export function wasm_tape_stop(game) {
    wasm.wasm_tape_stop(game);
}

const EXPECTED_RESPONSE_TYPES = new Set(['basic', 'cors', 'default']);

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && EXPECTED_RESPONSE_TYPES.has(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }
}

function __wbg_get_imports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbg___wbindgen_throw_dd24417ed36fc46e = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbg_alert_f07c379ec5f7f730 = function(arg0, arg1) {
        alert(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbg_debug_9ad80675faf0c9cf = function(arg0, arg1, arg2, arg3) {
        console.debug(arg0, arg1, arg2, arg3);
    };
    imports.wbg.__wbg_error_ad1ecdacd1bb600d = function(arg0, arg1, arg2, arg3) {
        console.error(arg0, arg1, arg2, arg3);
    };
    imports.wbg.__wbg_info_b7fa8ce2e59d29c6 = function(arg0, arg1, arg2, arg3) {
        console.info(arg0, arg1, arg2, arg3);
    };
    imports.wbg.__wbg_log_f614673762e98966 = function(arg0, arg1, arg2, arg3) {
        console.log(arg0, arg1, arg2, arg3);
    };
    imports.wbg.__wbg_onRZXRunning_bbc9dbc112820c89 = function(arg0, arg1) {
        onRZXRunning(arg0 !== 0, arg1 >>> 0);
    };
    imports.wbg.__wbg_onTapeBlock_eb2197e5ff0c42ae = function(arg0) {
        onTapeBlock(arg0 >>> 0);
    };
    imports.wbg.__wbg_putImageData_a17392c7701a1b6e = function(arg0, arg1, arg2, arg3) {
        putImageData(arg0, arg1, getArrayU8FromWasm0(arg2, arg3));
    };
    imports.wbg.__wbg_putSoundData_cf0a760dc0626ad4 = function(arg0, arg1) {
        putSoundData(getArrayF32FromWasm0(arg0, arg1));
    };
    imports.wbg.__wbg_warn_165ef4f6bcfc05e7 = function(arg0, arg1, arg2, arg3) {
        console.warn(arg0, arg1, arg2, arg3);
    };
    imports.wbg.__wbindgen_cast_2241b6af4c4b2941 = function(arg0, arg1) {
        // Cast intrinsic for `Ref(String) -> Externref`.
        const ret = getStringFromWasm0(arg0, arg1);
        return ret;
    };
    imports.wbg.__wbindgen_init_externref_table = function() {
        const table = wasm.__wbindgen_externrefs;
        const offset = table.grow(4);
        table.set(0, undefined);
        table.set(offset + 0, undefined);
        table.set(offset + 1, null);
        table.set(offset + 2, true);
        table.set(offset + 3, false);
    };

    return imports;
}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedFloat32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;


    wasm.__wbindgen_start();
    return wasm;
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (typeof module !== 'undefined') {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (typeof module_or_path !== 'undefined') {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (typeof module_or_path === 'undefined') {
        module_or_path = new URL('raze_web_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync };
export default __wbg_init;
