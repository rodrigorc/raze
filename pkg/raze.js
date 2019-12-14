(function() {
    const __exports = {};
    let wasm;

    /**
    * @param {boolean} is128k
    * @returns {number}
    */
    __exports.wasm_main = function(is128k) {
        const ret = wasm.wasm_main(is128k);
        return ret;
    };

    /**
    * @param {number} game
    */
    __exports.wasm_drop = function(game) {
        wasm.wasm_drop(game);
    };

    /**
    * @param {number} size
    * @returns {number}
    */
    __exports.wasm_alloc = function(size) {
        const ret = wasm.wasm_alloc(size);
        return ret;
    };

    /**
    * @param {number} game
    * @param {boolean} turbo
    */
    __exports.wasm_draw_frame = function(game, turbo) {
        wasm.wasm_draw_frame(game, turbo);
    };

    let cachegetUint8Memory = null;
    function getUint8Memory() {
        if (cachegetUint8Memory === null || cachegetUint8Memory.buffer !== wasm.memory.buffer) {
            cachegetUint8Memory = new Uint8Array(wasm.memory.buffer);
        }
        return cachegetUint8Memory;
    }

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
        const ret = wasm.wasm_load_tape(game, passArray8ToWasm(data), WASM_VECTOR_LEN);
        return ret >>> 0;
    };

    let cachegetInt32Memory = null;
    function getInt32Memory() {
        if (cachegetInt32Memory === null || cachegetInt32Memory.buffer !== wasm.memory.buffer) {
            cachegetInt32Memory = new Int32Array(wasm.memory.buffer);
        }
        return cachegetInt32Memory;
    }

    let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });

    cachedTextDecoder.decode();

    function getStringFromWasm(ptr, len) {
        return cachedTextDecoder.decode(getUint8Memory().subarray(ptr, ptr + len));
    }
    /**
    * @param {number} game
    * @param {number} index
    * @returns {string}
    */
    __exports.wasm_tape_name = function(game, index) {
        const retptr = 8;
        const ret = wasm.wasm_tape_name(retptr, game, index);
        const memi32 = getInt32Memory();
        const v0 = getStringFromWasm(memi32[retptr / 4 + 0], memi32[retptr / 4 + 1]).slice();
        wasm.__wbindgen_free(memi32[retptr / 4 + 0], memi32[retptr / 4 + 1] * 1);
        return v0;
    };

    /**
    * @param {number} game
    * @param {number} index
    * @returns {boolean}
    */
    __exports.wasm_tape_selectable = function(game, index) {
        const ret = wasm.wasm_tape_selectable(game, index);
        return ret !== 0;
    };

    /**
    * @param {number} game
    * @param {number} index
    */
    __exports.wasm_tape_seek = function(game, index) {
        wasm.wasm_tape_seek(game, index);
    };

    /**
    * @param {number} game
    */
    __exports.wasm_tape_stop = function(game) {
        wasm.wasm_tape_stop(game);
    };

    /**
    * @param {number} game
    * @param {Uint8Array} data
    * @returns {boolean}
    */
    __exports.wasm_load_snapshot = function(game, data) {
        const ret = wasm.wasm_load_snapshot(game, passArray8ToWasm(data), WASM_VECTOR_LEN);
        return ret !== 0;
    };

    function getArrayU8FromWasm(ptr, len) {
        return getUint8Memory().subarray(ptr / 1, ptr / 1 + len);
    }
    /**
    * @param {number} game
    * @returns {Uint8Array}
    */
    __exports.wasm_snapshot = function(game) {
        const retptr = 8;
        const ret = wasm.wasm_snapshot(retptr, game);
        const memi32 = getInt32Memory();
        const v0 = getArrayU8FromWasm(memi32[retptr / 4 + 0], memi32[retptr / 4 + 1]).slice();
        wasm.__wbindgen_free(memi32[retptr / 4 + 0], memi32[retptr / 4 + 1] * 1);
        return v0;
    };

    /**
    * @param {number} game
    */
    __exports.wasm_reset_input = function(game) {
        wasm.wasm_reset_input(game);
    };

    /**
    * @param {number} game
    * @param {number} key
    */
    __exports.wasm_key_up = function(game, key) {
        wasm.wasm_key_up(game, key);
    };

    /**
    * @param {number} game
    * @param {number} key
    */
    __exports.wasm_key_down = function(game, key) {
        wasm.wasm_key_down(game, key);
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

    function init(module) {

        let result;
        const imports = {};
        imports.wbg = {};
        imports.wbg.__wbg_log_ee5ef086d9ee97e1 = function(arg0, arg1) {
            console.log(getStringFromWasm(arg0, arg1));
        };
        imports.wbg.__wbg_onTapeBlock_edf70fa958be9ca2 = function(arg0) {
            exports.onTapeBlock(arg0 >>> 0);
        };
        imports.wbg.__wbg_putSoundData_8bb1346d8a1c3815 = function(arg0, arg1) {
            exports.putSoundData(getArrayF32FromWasm(arg0, arg1));
        };
        imports.wbg.__wbg_putImageData_b10a4ab0f43ddaa1 = function(arg0, arg1, arg2, arg3) {
            exports.putImageData(arg0, arg1, getArrayU8FromWasm(arg2, arg3));
        };
        imports.wbg.__wbg_alert_91462df3c2071dfb = function(arg0, arg1) {
            alert(getStringFromWasm(arg0, arg1));
        };

        if ((typeof URL === 'function' && module instanceof URL) || typeof module === 'string' || (typeof Request === 'function' && module instanceof Request)) {

            const response = fetch(module);
            if (typeof WebAssembly.instantiateStreaming === 'function') {
                result = WebAssembly.instantiateStreaming(response, imports)
                .catch(e => {
                    return response
                    .then(r => {
                        if (r.headers.get('Content-Type') != 'application/wasm') {
                            console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);
                            return r.arrayBuffer();
                        } else {
                            throw e;
                        }
                    })
                    .then(bytes => WebAssembly.instantiate(bytes, imports));
                });
            } else {
                result = response
                .then(r => r.arrayBuffer())
                .then(bytes => WebAssembly.instantiate(bytes, imports));
            }
        } else {

            result = WebAssembly.instantiate(module, imports)
            .then(result => {
                if (result instanceof WebAssembly.Instance) {
                    return { instance: result, module };
                } else {
                    return result;
                }
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
