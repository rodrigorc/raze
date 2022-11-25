import { putImageData, putSoundData, onTapeBlock, onRZXRunning } from '../raze.js';

let wasm;

const cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

let cachedUint8Memory0 = new Uint8Array();

function getUint8Memory0() {
    if (cachedUint8Memory0.byteLength === 0) {
        cachedUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8Memory0;
}

function getStringFromWasm0(ptr, len) {
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

const heap = new Array(32).fill(undefined);

heap.push(undefined, null, true, false);

let heap_next = heap.length;

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

function getObject(idx) { return heap[idx]; }

function dropObject(idx) {
    if (idx < 36) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}
/**
* @param {boolean} is128k
* @returns {number}
*/
export function wasm_main(is128k) {
    const ret = wasm.wasm_main(is128k);
    return ret;
}

/**
* @param {number} game
*/
export function wasm_drop(game) {
    wasm.wasm_drop(game);
}

/**
* @param {number} size
* @returns {number}
*/
export function wasm_alloc(size) {
    const ret = wasm.wasm_alloc(size);
    return ret;
}

/**
* @param {number} game
* @param {boolean} turbo
*/
export function wasm_draw_frame(game, turbo) {
    wasm.wasm_draw_frame(game, turbo);
}

let WASM_VECTOR_LEN = 0;

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1);
    getUint8Memory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
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

let cachedInt32Memory0 = new Int32Array();

function getInt32Memory0() {
    if (cachedInt32Memory0.byteLength === 0) {
        cachedInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32Memory0;
}
/**
* @param {number} game
* @param {number} index
* @returns {string}
*/
export function wasm_tape_name(game, index) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.wasm_tape_name(retptr, game, index);
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        return getStringFromWasm0(r0, r1);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_free(r0, r1);
    }
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
* @param {number} index
*/
export function wasm_tape_seek(game, index) {
    wasm.wasm_tape_seek(game, index);
}

/**
* @param {number} game
*/
export function wasm_tape_stop(game) {
    wasm.wasm_tape_stop(game);
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

function getArrayU8FromWasm0(ptr, len) {
    return getUint8Memory0().subarray(ptr / 1, ptr / 1 + len);
}
/**
* @param {number} game
* @returns {Uint8Array}
*/
export function wasm_snapshot(game) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.wasm_snapshot(retptr, game);
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var v0 = getArrayU8FromWasm0(r0, r1).slice();
        wasm.__wbindgen_free(r0, r1 * 1);
        return v0;
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
* @param {number} game
*/
export function wasm_reset_input(game) {
    wasm.wasm_reset_input(game);
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
* @param {number} key
*/
export function wasm_key_down(game, key) {
    wasm.wasm_key_down(game, key);
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
export function wasm_stop_rzx_replay(game) {
    wasm.wasm_stop_rzx_replay(game);
}

let cachedFloat32Memory0 = new Float32Array();

function getFloat32Memory0() {
    if (cachedFloat32Memory0.byteLength === 0) {
        cachedFloat32Memory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32Memory0;
}

function getArrayF32FromWasm0(ptr, len) {
    return getFloat32Memory0().subarray(ptr / 4, ptr / 4 + len);
}

async function load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

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

function getImports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        const ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbg_onRZXRunning_7a22aafb2ab573e6 = function(arg0, arg1) {
        onRZXRunning(arg0 !== 0, arg1 >>> 0);
    };
    imports.wbg.__wbg_putSoundData_8773188cf303e0e9 = function(arg0, arg1) {
        putSoundData(getArrayF32FromWasm0(arg0, arg1));
    };
    imports.wbg.__wbg_putImageData_72e3e076dd01cf61 = function(arg0, arg1, arg2, arg3) {
        putImageData(arg0, arg1, getArrayU8FromWasm0(arg2, arg3));
    };
    imports.wbg.__wbg_onTapeBlock_db302dfd3cf6cc28 = function(arg0) {
        onTapeBlock(arg0 >>> 0);
    };
    imports.wbg.__wbg_alert_61045fcdf98ae703 = function(arg0, arg1) {
        alert(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbg_debug_64711eb2fc6980ef = function(arg0, arg1, arg2, arg3) {
        console.debug(getObject(arg0), getObject(arg1), getObject(arg2), getObject(arg3));
    };
    imports.wbg.__wbg_error_00c5d571f754f629 = function(arg0, arg1, arg2, arg3) {
        console.error(getObject(arg0), getObject(arg1), getObject(arg2), getObject(arg3));
    };
    imports.wbg.__wbg_info_d60a960a4e955dc2 = function(arg0, arg1, arg2, arg3) {
        console.info(getObject(arg0), getObject(arg1), getObject(arg2), getObject(arg3));
    };
    imports.wbg.__wbg_log_de258f66ad9eb784 = function(arg0, arg1, arg2, arg3) {
        console.log(getObject(arg0), getObject(arg1), getObject(arg2), getObject(arg3));
    };
    imports.wbg.__wbg_warn_be542501a57387a5 = function(arg0, arg1, arg2, arg3) {
        console.warn(getObject(arg0), getObject(arg1), getObject(arg2), getObject(arg3));
    };

    return imports;
}

function initMemory(imports, maybe_memory) {

}

function finalizeInit(instance, module) {
    wasm = instance.exports;
    init.__wbindgen_wasm_module = module;
    cachedFloat32Memory0 = new Float32Array();
    cachedInt32Memory0 = new Int32Array();
    cachedUint8Memory0 = new Uint8Array();


    return wasm;
}

function initSync(module) {
    const imports = getImports();

    initMemory(imports);

    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }

    const instance = new WebAssembly.Instance(module, imports);

    return finalizeInit(instance, module);
}

async function init(input) {
    if (typeof input === 'undefined') {
        input = new URL('raze_bg.wasm', import.meta.url);
    }
    const imports = getImports();

    if (typeof input === 'string' || (typeof Request === 'function' && input instanceof Request) || (typeof URL === 'function' && input instanceof URL)) {
        input = fetch(input);
    }

    initMemory(imports);

    const { instance, module } = await load(await input, imports);

    return finalizeInit(instance, module);
}

export { initSync }
export default init;
