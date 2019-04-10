(function() {
    const __exports = {};
    let wasm;

    let cachedTextDecoder = new TextDecoder('utf-8');

    let cachegetUint8Memory = null;
    function getUint8Memory() {
        if (cachegetUint8Memory === null || cachegetUint8Memory.buffer !== wasm.memory.buffer) {
            cachegetUint8Memory = new Uint8Array(wasm.memory.buffer);
        }
        return cachegetUint8Memory;
    }

    function getStringFromWasm(ptr, len) {
        return cachedTextDecoder.decode(getUint8Memory().subarray(ptr, ptr + len));
    }

    __exports.__wbg_log_ee5ef086d9ee97e1 = function(arg0, arg1) {
        let varg0 = getStringFromWasm(arg0, arg1);
        console.log(varg0);
    };

    __exports.__wbg_alert_91462df3c2071dfb = function(arg0, arg1) {
        let varg0 = getStringFromWasm(arg0, arg1);
        alert(varg0);
    };

    function getArrayU8FromWasm(ptr, len) {
        return getUint8Memory().subarray(ptr / 1, ptr / 1 + len);
    }

    __exports.__wbg_putImageData_b10a4ab0f43ddaa1 = function(arg0, arg1, arg2, arg3) {
        let varg2 = getArrayU8FromWasm(arg2, arg3);
        exports.putImageData(arg0, arg1, varg2);
    };

    let cachegetFloat32Memory = null;
    function getFloat32Memory() {
        if (cachegetFloat32Memory === null || cachegetFloat32Memory.buffer !== wasm.memory.buffer) {
            cachegetFloat32Memory = new Float32Array(wasm.memory.buffer);
        }
        return cachegetFloat32Memory;
    }

    function getArrayF32FromWasm(ptr, len) {
        return getFloat32Memory().subarray(ptr / 4, ptr / 4 + len);
    }

    __exports.__wbg_putSoundData_8bb1346d8a1c3815 = function(arg0, arg1) {
        let varg0 = getArrayF32FromWasm(arg0, arg1);
        exports.putSoundData(varg0);
    };

    __exports.__wbg_onTapeBlock_edf70fa958be9ca2 = function(arg0) {
        exports.onTapeBlock(arg0);
    };
    /**
    * @param {boolean} is128k
    * @returns {number}
    */
    __exports.wasm_main = function(is128k) {
        return wasm.wasm_main(is128k);
    };

    /**
    * @param {number} game
    * @returns {void}
    */
    __exports.wasm_drop = function(game) {
        return wasm.wasm_drop(game);
    };

    /**
    * @param {number} size
    * @returns {number}
    */
    __exports.wasm_alloc = function(size) {
        return wasm.wasm_alloc(size);
    };

    /**
    * @param {number} game
    * @param {boolean} turbo
    * @returns {void}
    */
    __exports.wasm_draw_frame = function(game, turbo) {
        return wasm.wasm_draw_frame(game, turbo);
    };

    let WASM_VECTOR_LEN = 0;

    function passArray8ToWasm(arg) {
        const ptr = wasm.__wbindgen_malloc(arg.length * 1);
        getUint8Memory().set(arg, ptr / 1);
        WASM_VECTOR_LEN = arg.length;
        return ptr;
    }
    /**
    * @param {number} game
    * @param {Uint8Array} data
    * @returns {number}
    */
    __exports.wasm_load_tape = function(game, data) {
        const ptr1 = passArray8ToWasm(data);
        const len1 = WASM_VECTOR_LEN;
        return wasm.wasm_load_tape(game, ptr1, len1);
    };

    let cachedGlobalArgumentPtr = null;
    function globalArgumentPtr() {
        if (cachedGlobalArgumentPtr === null) {
            cachedGlobalArgumentPtr = wasm.__wbindgen_global_argument_ptr();
        }
        return cachedGlobalArgumentPtr;
    }

    let cachegetUint32Memory = null;
    function getUint32Memory() {
        if (cachegetUint32Memory === null || cachegetUint32Memory.buffer !== wasm.memory.buffer) {
            cachegetUint32Memory = new Uint32Array(wasm.memory.buffer);
        }
        return cachegetUint32Memory;
    }
    /**
    * @param {number} game
    * @param {number} index
    * @returns {string}
    */
    __exports.wasm_tape_name = function(game, index) {
        const retptr = globalArgumentPtr();
        wasm.wasm_tape_name(retptr, game, index);
        const mem = getUint32Memory();
        const rustptr = mem[retptr / 4];
        const rustlen = mem[retptr / 4 + 1];

        const realRet = getStringFromWasm(rustptr, rustlen).slice();
        wasm.__wbindgen_free(rustptr, rustlen * 1);
        return realRet;

    };

    /**
    * @param {number} game
    * @param {number} index
    * @returns {boolean}
    */
    __exports.wasm_tape_selectable = function(game, index) {
        return (wasm.wasm_tape_selectable(game, index)) !== 0;
    };

    /**
    * @param {number} game
    * @param {number} index
    * @returns {void}
    */
    __exports.wasm_tape_seek = function(game, index) {
        return wasm.wasm_tape_seek(game, index);
    };

    /**
    * @param {number} game
    * @returns {void}
    */
    __exports.wasm_tape_stop = function(game) {
        return wasm.wasm_tape_stop(game);
    };

    /**
    * @param {number} game
    * @param {Uint8Array} data
    * @returns {boolean}
    */
    __exports.wasm_load_snapshot = function(game, data) {
        const ptr1 = passArray8ToWasm(data);
        const len1 = WASM_VECTOR_LEN;
        try {
            return (wasm.wasm_load_snapshot(game, ptr1, len1)) !== 0;

        } finally {
            wasm.__wbindgen_free(ptr1, len1 * 1);

        }

    };

    /**
    * @param {number} game
    * @returns {Uint8Array}
    */
    __exports.wasm_snapshot = function(game) {
        const retptr = globalArgumentPtr();
        wasm.wasm_snapshot(retptr, game);
        const mem = getUint32Memory();
        const rustptr = mem[retptr / 4];
        const rustlen = mem[retptr / 4 + 1];

        const realRet = getArrayU8FromWasm(rustptr, rustlen).slice();
        wasm.__wbindgen_free(rustptr, rustlen * 1);
        return realRet;

    };

    /**
    * @param {number} game
    * @returns {void}
    */
    __exports.wasm_reset_input = function(game) {
        return wasm.wasm_reset_input(game);
    };

    /**
    * @param {number} game
    * @param {number} key
    * @returns {void}
    */
    __exports.wasm_key_up = function(game, key) {
        return wasm.wasm_key_up(game, key);
    };

    /**
    * @param {number} game
    * @param {number} key
    * @returns {void}
    */
    __exports.wasm_key_down = function(game, key) {
        return wasm.wasm_key_down(game, key);
    };

    const heap = new Array(32);

    heap.fill(undefined);

    heap.push(undefined, null, true, false);

    let heap_next = heap.length;

    function dropObject(idx) {
        if (idx < 36) return;
        heap[idx] = heap_next;
        heap_next = idx;
    }

    __exports.__wbindgen_object_drop_ref = function(i) { dropObject(i); };

    function init(module_or_path, maybe_memory) {
        let result;
        const imports = { './raze': __exports };
        if (module_or_path instanceof URL || typeof module_or_path === 'string' || module_or_path instanceof Request) {

            const response = fetch(module_or_path);
            if (typeof WebAssembly.instantiateStreaming === 'function') {
                result = WebAssembly.instantiateStreaming(response, imports)
                .catch(e => {
                    console.warn("`WebAssembly.instantiateStreaming` failed. Assuming this is because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);
                    return response
                    .then(r => r.arrayBuffer())
                    .then(bytes => WebAssembly.instantiate(bytes, imports));
                });
            } else {
                result = response
                .then(r => r.arrayBuffer())
                .then(bytes => WebAssembly.instantiate(bytes, imports));
            }
        } else {

            result = WebAssembly.instantiate(module_or_path, imports)
            .then(instance => {
                return { instance, module: module_or_path };
            });
        }
        return result.then(({instance, module}) => {
            wasm = instance.exports;
            init.__wbindgen_wasm_module = module;

            return wasm;
        });
    }

    self.wasm_bindgen = Object.assign(init, __exports);

})();
