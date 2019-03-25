'use strict';

let g_module = {};

let g_utfDecoder = new TextDecoder('utf-8');
function getStr(ptr, len) {
    let slice = new Uint8Array(g_module.memory.buffer, ptr, len);
    return g_utfDecoder.decode(slice);
};

let g_actx = new AudioContext();
let g_audio_next = 0;
let g_turbo = false;
let g_realCanvas = null;

function fetch_with_cors_if_needed(url, callback, error) {
    let on_ok = resp => {
        if (resp.ok)
            resp.arrayBuffer().then(callback);
        else
            error();
    };
    fetch(url).
        then(on_ok).
        catch(_ => {
            fetch('https://cors-anywhere.herokuapp.com/' + url).
                then(on_ok).
                catch (e => {
                    error(e);
                });
        });
}

let g_delayed_funcs = null;
function call_with_delay(first, other, args) {
    g_delayed_funcs = [first, other, args];
}

let g_lastSnapshot = null;
if (window.localStorage) {
    let last = window.localStorage.getItem("lastSnapshot");
    if (last) {
        g_lastSnapshot = base64decode(last);
    }
}

function boolURLParamDef(urlParams, key, def) {
    let res = urlParams.get(key);
    if (res === null)
        return def;
    res = res.toLowerCase();
    if (res == "")
        return true;
    let c = res[0];
    if (c == '0' || c == 'n' || c == 'f')
        return false;
    return true;
}

function onDocumentLoad() {

    let urlParams = new URLSearchParams(window.location.search);
    let webgl = boolURLParamDef(urlParams, 'webgl', true)

    let ctx = null, gl = null;

    let canvas3d = document.querySelector('#game-layer-3d');
    let canvas = document.querySelector('#game-layer');

    if (webgl) {
        gl = canvas3d.getContext('webgl');
    }

    if (gl && initMyGL(gl)) {
        console.log("using webgl rendering");
        g_realCanvas = canvas3d;
    } else {
        if (webgl)
            console.log("webgl initialization failed, falling back to canvas");
        else
            console.log("webgl initialization skipped, falling back to canvas");
        gl = null;
        canvas3d.style.display = 'none';
        canvas.style.display = '';

        ctx = canvas.getContext('2d');
        ctx.imageSmoothingEnabled = false;
        g_realCanvas = canvas;
    }

    let imports = {
        env: {
            consolelog: (ptr, len) => console.log(getStr(ptr, len)),
            alert: (ptr, len) => alert(getStr(ptr, len)),
            putImageData: (w, h, ptr, len) => {
                if (gl) {
                    let data = new Uint8Array(g_module.memory.buffer, ptr, len);
                    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, data);
                    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
                    gl.flush();
                } else {
                    let data = new Uint8ClampedArray(g_module.memory.buffer, ptr, len);
                    let img = new ImageData(data, w, h);
                    ctx.putImageData(img, 0, 0);
                }
            },
            putSoundData: (ptr, len) => {
                let asrc = g_actx.createBufferSource();
                let abuf = g_actx.createBuffer(1, len, g_module.is128k? 21112 : 20833); // cpufreq / AUDIO_SAMPLE / RATE_MULTIPLIER
                let data = abuf.getChannelData(0);
                let slice = new Float32Array(g_module.memory.buffer, ptr, len);
                for (let i = 0; i < len; ++i)
                    data[i] = slice[i];
                asrc.buffer = abuf;
                asrc.connect(g_actx.destination);

                asrc.start(g_audio_next);
                g_audio_next = Math.max(g_audio_next, g_actx.currentTime) + abuf.duration;
            },
            onTapeBlock: (index) => {
                onTapeBlock(index);
            },
        }
    };
    let wasm = '/target/wasm32-unknown-unknown/release/raze.wasm';
    if (WebAssembly.instantiateStreaming) {
        wasm = WebAssembly.instantiateStreaming(fetch(wasm), imports);
    } else {
        wasm = fetch(wasm).
                then(resp => resp.arrayBuffer()).
                then(bytes => WebAssembly.instantiate(bytes, imports));
    }
    wasm.
        then(wasm => {
            let exports = wasm.instance.exports;
            Object.assign(g_module, {
                wasm: wasm,
                exports: exports,
                memory: exports.memory,
            });
            let is128k = !boolURLParamDef(urlParams, '48k', false)
            g_module.is128k = is128k;
            g_module.game = exports.wasm_main(is128k);

            let snapshot = urlParams.get("snapshot");
            if (snapshot) {
                console.log("SNAPSHOT=", snapshot);
                fetch_with_cors_if_needed(snapshot,
                    bytes => {
                        saveLastSnapshot(new Uint8Array(bytes));
                        handleLoadLastSnapshot();
                    },
                    error => {
                        alert("Cannot download file " + snapshot);
                    }
                );
            }

            let tape = urlParams.get("tape");
            if (tape) {
                console.log("TAPE=", tape);
                fetch_with_cors_if_needed(tape,
                    bytes => {
                        if (bytes) {
                            if (is128k) {
                                call_with_delay(1000, 100, [
                                    () => g_module.exports.wasm_key_down(g_module.game, 0x60), //ENTER
                                    () => g_module.exports.wasm_key_up(g_module.game, 0x60), //ENTER
                                    () => onLoadTape(bytes),
                                ]);
                            } else {
                                call_with_delay(2000, 100, [
                                    () => g_module.exports.wasm_key_down(g_module.game, 0x63), //J (LOAD)
                                    () => g_module.exports.wasm_key_up(g_module.game, 0x63),
                                    () => g_module.exports.wasm_key_down(g_module.game, 0x71), //SS
                                    () => g_module.exports.wasm_key_down(g_module.game, 0x50), //P (")
                                    () => g_module.exports.wasm_key_up(g_module.game, 0x50), //P (")
                                    () => g_module.exports.wasm_key_down(g_module.game, 0x50), //P (")
                                    () => g_module.exports.wasm_key_up(g_module.game, 0x50), //P (")
                                    () => g_module.exports.wasm_key_up(g_module.game, 0x71), //SS
                                    () => g_module.exports.wasm_key_down(g_module.game, 0x60), //ENTER
                                    () => g_module.exports.wasm_key_up(g_module.game, 0x60), //ENTER
                                    () => onLoadTape(bytes),
                                ]);
                            }
                        }
                    },
                    error => {
                        alert("Cannot download file " + tape);
                    }
                );
            }
            window.addEventListener('keydown', onKeyDown)
            window.addEventListener('keyup', onKeyUp)
            window.addEventListener('focus', onFocus)
            window.addEventListener('blur', onBlur)
            g_audio_next = g_actx.currentTime;
            if (document.hasFocus())
                onFocus();
        });

    document.querySelector('#reset_48k').addEventListener('click', handleReset48k, false);
    document.querySelector('#reset_128k').addEventListener('click', handleReset128k, false);
    document.querySelector('#load_tape').addEventListener('click', handleLoadTape, false);
    document.querySelector('#stop_tape').addEventListener('click', handleStopTape, false);
    document.querySelector('#snapshot').addEventListener('click', handleSnapshot, false);
    document.querySelector('#load_snapshot').addEventListener('click', handleLoadSnapshot, false);
    document.querySelector('#load_last_snapshot').addEventListener('click', handleLoadLastSnapshot, false);
    document.querySelector('#fullscreen').addEventListener('click', handleFullscreen, false);
    document.querySelector('#turbo').addEventListener('click', handleTurbo, false);
    let dither = document.querySelector('#dither');
    dither.addEventListener('click', function(evt) { handleDither.call(this, evt, gl) }, false);
    handleDither.call(dither, null, gl)

    let cursorKeys = document.querySelector('#cursor_keys');
    cursorKeys.addEventListener('change', handleCursorKeys, false);
    if (window.localStorage) {
        let cursorSel = parseInt(window.localStorage.getItem("cursorKeys"));
        if (!isNaN(cursorSel))
            cursorKeys.selectedIndex = cursorSel;
    }
    handleCursorKeys.call(cursorKeys, null);
}

function onKeyDown(ev) {
    //console.log(ev.code);
    switch (ev.code) {
    case "F6":
        handleSnapshot(ev);
        ev.preventDefault();
        return;
    case "F9":
        handleLoadLastSnapshot(ev);
        ev.preventDefault();
        return;
    case "F10":
        document.querySelector('#turbo').checked = g_turbo = true;
        ev.preventDefault();
        return;
    case "F11":
        handleFullscreen(ev);
        ev.preventDefault();
        return;
    }

    let key = getKeyCode(ev);
    if (key == undefined)
        return;
    g_module.exports.wasm_key_down(g_module.game, key);
    ev.preventDefault();
}
function onKeyUp(ev) {
    switch (ev.code) {
    case "F10":
        document.querySelector('#turbo').checked = g_turbo = false;
        ev.preventDefault();
        return;
    }

    let key = getKeyCode(ev);
    if (key == undefined)
        return;
    g_module.exports.wasm_key_up(g_module.game, key);
    ev.preventDefault();
}

let g_interval = null;
function onFocus(ev) {
    if (!g_delayed_funcs)
        g_module.exports.wasm_reset_input(g_module.game);
    if (g_interval === null) {
        g_interval = setInterval(function(){
            if (g_turbo) {
                g_module.exports.wasm_draw_frame(g_module.game, true);
            } else while (g_audio_next - g_actx.currentTime < 0.05) {
                g_module.exports.wasm_draw_frame(g_module.game, false);
                if (g_delayed_funcs !== null) {
                    if ((g_delayed_funcs[0] -= 20) <= 0) {
                        let f = g_delayed_funcs[2].shift();
                        if (f) {
                            f();
                            g_delayed_funcs[0] = g_delayed_funcs[1];
                        } else {
                            g_delayed_funcs = null;
                        }
                    }
                }
            }
        }, 0);
    }
}
function onBlur(ev) {
    if (!g_delayed_funcs)
        g_module.exports.wasm_reset_input(g_module.game);
    if (g_interval !== null) {
        clearInterval(g_interval);
        g_interval = null;
    }
}

let g_cursorKeys = null;

function handleCursorKeys(evt) {
    let sel = this.selectedIndex;
    if (window.localStorage)
        window.localStorage.setItem("cursorKeys", sel);
    g_cursorKeys = CURSOR_KEYS[sel];
    this.blur();
    if (g_module.exports)
        g_module.exports.wasm_reset_input(g_module.game);
}

const CURSOR_KEYS = [
    //cursorkeys
    [0xf034, 0xf042, 0xf044, 0xf043, 0x71], //Shift+{5,8,6,7}, SymbolShift
    //kempston
    [0x81, 0x80, 0x82, 0x83, 0x84],
    //sinclair
    [0x44, 0x43, 0x42, 0x41, 0x40], //6, 7, 8, 9, 0
    //cursorjoy
    [0x34, 0x42, 0x44, 0x43, 0x40], //5, 8, 6, 7, 0
];

function getKeyCode(ev) {
    switch (ev.code) {
    case "ArrowLeft":
        return g_cursorKeys[0];
    case "ArrowRight":
        return g_cursorKeys[1];
    case "ArrowDown":
        return g_cursorKeys[2];
    case "ArrowUp":
        return g_cursorKeys[3];
    case "ControlLeft":
        return g_cursorKeys[4];

    case "ShiftLeft":
    case "ShiftRight":
        return 0xf0; //just like 0x00, but 0x00 is ignored by game code
    case "KeyZ":
        return 0x01;
    case "KeyX":
        return 0x02;
    case "KeyC":
        return 0x03;
    case "KeyV":
        return 0x04;
    case "KeyA":
        return 0x10;
    case "KeyS":
        return 0x11;
    case "KeyD":
        return 0x12;
    case "KeyF":
        return 0x13;
    case "KeyG":
        return 0x14;
    case "KeyQ":
        return 0x20;
    case "KeyW":
        return 0x21;
    case "KeyE":
        return 0x22;
    case "KeyR":
        return 0x23;
    case "KeyT":
        return 0x24;
    case "Digit1":
        return 0x30;
    case "Digit2":
        return 0x31;
    case "Digit3":
        return 0x32;
    case "Digit4":
        return 0x33;
    case "Digit5":
        return 0x34;
    case "Digit0":
        return 0x40;
    case "Digit9":
        return 0x41;
    case "Digit8":
        return 0x42;
    case "Digit7":
        return 0x43;
    case "Digit6":
        return 0x44;
    case "KeyP":
        return 0x50;
    case "KeyO":
        return 0x51;
    case "KeyI":
        return 0x52;
    case "KeyU":
        return 0x53;
    case "KeyY":
        return 0x54;
    case "Enter":
        return 0x60;
    case "KeyL":
        return 0x61;
    case "KeyK":
        return 0x62;
    case "KeyJ":
        return 0x63;
    case "KeyH":
        return 0x64;
    case "Space":
        return 0x70;
    case "ControlRight":
        return 0x71;
    case "KeyM":
        return 0x72;
    case "KeyN":
        return 0x73;
    case "KeyB":
        return 0x74;
    case "Backspace":
        return 0xf040; //Shift+0
    default:
        return null;
    }
}

function resetTape() {
    let xTape = document.getElementById("tape");
    while (xTape.firstChild)
        xTape.removeChild(xTape.firstChild);
    return xTape;
}

function onTapeBlock(index) {
    console.log("Block", index);
    let xTape = document.getElementById("tape");
    for (let i = 0; i < xTape.children.length; ++i) {
        let btn = xTape.children[i];
        if (btn['data-index'] == index)
            btn.classList.add('selected');
        else
            btn.classList.remove('selected');
    }
}

function onLoadTape(data) {
    console.log("data " + data.byteLength);
    console.log(data);
    let ptr = g_module.exports.wasm_alloc(data.byteLength);
    let d = new Uint8Array(g_module.memory.buffer, ptr, data.byteLength);
    d.set(new Uint8Array(data));
    let tape_len = g_module.exports.wasm_load_tape(g_module.game, ptr, data.byteLength);
    let xTape = resetTape();

    for (let i = 0; i < tape_len; ++i) {
        let tape_ptr = g_module.exports.wasm_tape_name(g_module.game, i);
        let tape_ptr_len = g_module.exports.wasm_tape_name_len(g_module.game, i);
        let selectable = g_module.exports.wasm_tape_selectable(g_module.game, i);
        let tape_name = getStr(tape_ptr, tape_ptr_len);
        console.log("Tape ", i, tape_name);
        if (selectable) {
            let btn = document.createElement("button");
            btn.textContent = tape_name;
            xTape.appendChild(btn);
            btn.addEventListener('click', handleTapeBlock, false);
            btn['data-index'] = i;
        }
    }
    xTape.firstChild.classList.add('selected');
}

function handleTapeSelect(evt) {
    let f = evt.target.files[0];
    console.log("reading " + f.name);
    let reader = new FileReader();
    reader.onload = function(e) { onLoadTape(this.result); };
    reader.readAsArrayBuffer(f);
}

function handleTapeBlock(evt) {
    let btn = evt.target;
    let index = btn['data-index'];
    //evt.target.classList.add('playing');
    g_module.exports.wasm_tape_seek(g_module.game, index);
}

function handleReset48k(evt) {
    resetTape();
    g_module.exports.wasm_drop(g_module.game);
    g_module.is128k = false;
    g_module.game = g_module.exports.wasm_main(g_module.is128k);
}

function handleReset128k(evt) {
    resetTape();
    g_module.exports.wasm_drop(g_module.game);
    g_module.is128k = true;
    g_module.game = g_module.exports.wasm_main(g_module.is128k);
}

function handleLoadTape(evt) {
    let x = document.createElement("input");
    x.type = "file";
    x.accept = [".tap", ".tzx", ".zip"];
    x.addEventListener('change', handleTapeSelect, false);
    x.click();
}

function handleStopTape(evt) {
    g_module.exports.wasm_tape_stop(g_module.game);
}

function handleLoadSnapshotSelect(evt) {
    let f = evt.target.files[0];
    console.log("reading " + f.name);
    let reader = new FileReader();
    reader.onload = function(e) {
        saveLastSnapshot(new Uint8Array(this.result));
        handleLoadLastSnapshot();
    }
    reader.readAsArrayBuffer(f);
}

function handleLoadSnapshot(evt) {
    let x = document.createElement("input");
    x.type = "file";
    x.accept = [".z80", ".zip"];
    x.addEventListener('change', handleLoadSnapshotSelect, false);
    x.click();
}

function saveLastSnapshot(data) {
    g_lastSnapshot = data;
    if (g_lastSnapshot && window.localStorage) {
        window.localStorage.setItem("lastSnapshot", base64encode(g_lastSnapshot));
    }
}

function handleLoadLastSnapshot(evt) {
    if (!g_lastSnapshot)
        return;

    let ptr = g_module.exports.wasm_alloc(g_lastSnapshot.byteLength);
    let d = new Uint8Array(g_module.memory.buffer, ptr, g_lastSnapshot.byteLength);
    d.set(g_lastSnapshot);
    g_module.is128k = g_module.exports.wasm_load_snapshot(g_module.game, ptr, g_lastSnapshot.byteLength);
}

function handleSnapshot(evt) {
    console.log("snapshot");
    let snapshot = g_module.exports.wasm_snapshot(g_module.game);
    let ptr = g_module.exports.wasm_buffer_ptr(snapshot);
    let len = g_module.exports.wasm_buffer_len(snapshot);
    //copy the data because it will be freed at the end
    let data = new Uint8Array(new Uint8Array(g_module.memory.buffer, ptr, len));
    let blob = new Blob([data], {type: "application/octet-stream"});
    let url = window.URL.createObjectURL(blob);

    saveLastSnapshot(data);

    let a = document.createElement("a");
    a.style = "display: none";
    a.href = url;
    a.download = "snapshot.z80";
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    window.URL.revokeObjectURL(url);
    g_module.exports.wasm_buffer_free(snapshot);
}
function handleFullscreen(evt) {
    console.log("fullscreen");
    let canvas = g_realCanvas;
    let fs = canvas.requestFullscreen || canvas.mozRequestFullScreen || canvas.webkitRequestFullScreen || canvas.msRequestFullscreen;
    if (fs)
        fs.call(canvas);
}

function handleTurbo(evt) {
    g_turbo = this.checked;
}

function handleDither(evt, gl, ctx) {
    if (gl) {
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, this.checked? gl.LINEAR : gl.NEAREST);
    } else {
        let canvas = document.querySelector('#game-layer');
        if (this.checked)
            canvas.classList.remove('pixelated');
        else
            canvas.classList.add('pixelated');
    }
}

document.addEventListener("DOMContentLoaded", onDocumentLoad);

function compileShader(gl, type, source) {
    const shader = gl.createShader(type);

    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
        console.log('Shader compiler error: ' + gl.getShaderInfoLog(shader));
        gl.deleteShader(shader);
        return null;
    }
    return shader;
}
function linkShader(gl, vs, fs) {
    const vertexShader = compileShader(gl, gl.VERTEX_SHADER, vs);
    const fragmentShader = compileShader(gl, gl.FRAGMENT_SHADER, fs);
    if (!vertexShader || !fragmentShader) {
        return null;
    }

    const shaderProgram = gl.createProgram();
    gl.attachShader(shaderProgram, vertexShader);
    gl.attachShader(shaderProgram, fragmentShader);
    gl.linkProgram(shaderProgram);
    if (!gl.getProgramParameter(shaderProgram, gl.LINK_STATUS)) {
        console.log('Shader linker error: ' + gl.getProgramInfoLog(shaderProgram));
        return null;
    }
    return shaderProgram;
}

function initMyGL(gl) {
    if (!gl) {
        return false;
    }
    //Shaders
    const vs = `
    attribute vec2 aPos;
    attribute vec2 aTex;
    varying highp vec2 vTex;

    void main() {
      gl_Position = vec4(aPos, 0, 1);
      vTex = aTex;
    }
    `;

    const fs = `
    uniform sampler2D uSampler;
    varying highp vec2 vTex;

    void main() {
        gl_FragColor = texture2D(uSampler, vTex);
    }
    `;
    const program = linkShader(gl, vs, fs);
    if (!program)
        return false;

    //Buffers
    const bufferV = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, bufferV);

    const positionsV = [
        1.0,  1.0,
        -1.0,  1.0,
        1.0, -1.0,
        -1.0, -1.0,
    ];
    gl.bufferData(gl.ARRAY_BUFFER,
        new Float32Array(positionsV),
        gl.STATIC_DRAW);

    const bufferT = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, bufferT);

    const positionsT = [
        1.0,  0.0,
        0.0,  0.0,
        1.0,  1.0,
        0.0,  1.0,
    ];
    gl.bufferData(gl.ARRAY_BUFFER,
        new Float32Array(positionsT),
        gl.STATIC_DRAW);
    
    //let buffers = { vertex: bufferV, texture: bufferT };

    gl.clearColor(0.0,0.0,0.0,1);
    gl.clear(gl.COLOR_BUFFER_BIT);

    gl.useProgram(program);
    gl.bindBuffer(gl.ARRAY_BUFFER, bufferV);
    let aPos = gl.getAttribLocation(program, 'aPos');
    gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 0, 0);
    gl.enableVertexAttribArray(aPos);

    gl.bindBuffer(gl.ARRAY_BUFFER, bufferT);
    let aTex = gl.getAttribLocation(program, 'aTex');
    gl.vertexAttribPointer(aTex, 2, gl.FLOAT, false, 0, 0);
    gl.enableVertexAttribArray(aTex);
    
    const texture = gl.createTexture();
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, texture);
    const pixel = new Uint8Array([255, 0, 255, 255]); //dummy image
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);

    let uSampler = gl.getUniformLocation(program, 'uSampler');
    gl.uniform1i(uSampler, 0);
    const error = gl.getError();
    if (error != 0) {
        console.log("GL error: ", error);
        return false;
    }
    return true;
}

