import { onRZXRunning, onTapeBlock, putImageData, putSoundData } from '../raze.js';


/**
 * @param {number} bld
 * @returns {number}
 */
export function wasm_builder_build(bld) {
    const ret = wasm.wasm_builder_build(bld);
    return ret >>> 0;
}

/**
 * @returns {number}
 */
export function wasm_builder_new() {
    const ret = wasm.wasm_builder_new();
    return ret >>> 0;
}

/**
 * @param {number} bld
 * @param {number} border_x
 * @param {number} border_y
 */
export function wasm_builder_set_border(bld, border_x, border_y) {
    wasm.wasm_builder_set_border(bld, border_x, border_y);
}

/**
 * @param {number} bld
 * @param {number} model
 */
export function wasm_builder_set_model(bld, model) {
    wasm.wasm_builder_set_model(bld, model);
}

/**
 * @param {number} bld
 * @param {Uint8Array} data
 */
export function wasm_builder_set_snapshot(bld, data) {
    const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    wasm.wasm_builder_set_snapshot(bld, ptr0, len0);
}

/**
 * @param {number} game
 */
export function wasm_disk_eject(game) {
    wasm.wasm_disk_eject(game);
}

/**
 * @param {number} game
 * @param {Uint8Array} data
 * @returns {boolean}
 */
export function wasm_disk_load(game, data) {
    const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.wasm_disk_load(game, ptr0, len0);
    return ret !== 0;
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
export function wasm_game_drop(game) {
    wasm.wasm_game_drop(game);
}

/**
 * @param {number} game
 * @returns {number}
 */
export function wasm_game_model(game) {
    const ret = wasm.wasm_game_model(game);
    return ret;
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

export function wasm_main() {
    wasm.wasm_main();
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
 * @param {Uint8Array} data
 * @returns {number}
 */
export function wasm_tape_load(game, data) {
    const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.wasm_tape_load(game, ptr0, len0);
    return ret >>> 0;
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
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_throw_bb96b2010945f0bc: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_alert_8b030ddbc89dc95d: function(arg0, arg1) {
            alert(getStringFromWasm0(arg0, arg1));
        },
        __wbg_debug_a680c6306b002d2f: function(arg0, arg1, arg2, arg3) {
            console.debug(arg0, arg1, arg2, arg3);
        },
        __wbg_error_d02b59e42e8c9cd8: function(arg0, arg1, arg2, arg3) {
            console.error(arg0, arg1, arg2, arg3);
        },
        __wbg_info_000cb4eb27951897: function(arg0, arg1, arg2, arg3) {
            console.info(arg0, arg1, arg2, arg3);
        },
        __wbg_log_403c270908e48f02: function(arg0, arg1, arg2, arg3) {
            console.log(arg0, arg1, arg2, arg3);
        },
        __wbg_onRZXRunning_0984a72d4aea42a2: function(arg0, arg1) {
            onRZXRunning(arg0 !== 0, arg1 >>> 0);
        },
        __wbg_onTapeBlock_a72a692719349e9e: function(arg0) {
            onTapeBlock(arg0 >>> 0);
        },
        __wbg_putImageData_5f3be119303929a5: function(arg0, arg1, arg2, arg3) {
            putImageData(arg0, arg1, getArrayU8FromWasm0(arg2, arg3));
        },
        __wbg_putSoundData_53218e515a22dd14: function(arg0, arg1) {
            putSoundData(getArrayF32FromWasm0(arg0, arg1));
        },
        __wbg_warn_ee28149ca3d208d8: function(arg0, arg1, arg2, arg3) {
            console.warn(arg0, arg1, arg2, arg3);
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./raze_web_bg.js": import0,
    };
}

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
    return decodeText(ptr >>> 0, len);
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

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedFloat32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
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

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
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


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('raze_web_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
