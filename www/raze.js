'use strict';

let g_module = {};
let g_actx = new AudioContext();
let g_audio_next = 0;
let g_turbo = false;
let g_realCanvas = null;

function ensureAudioRunning() {
    //autoplay policy in chrome requires this
    if (g_actx.state == "suspended") {
        console.log("Resume AutoPlay");
        g_actx.resume();
    }
}

async function fetch_with_cors_if_needed(url, callback, error) {
    try {
        let resp;
        try {
            resp = await fetch(url);
        } catch (_) {
            resp = await fetch('https://cors-anywhere.herokuapp.com/' + url);
        }
        if (resp.ok)
            callback(await resp.arrayBuffer());
        else
            error();
    } catch (e) {
        error();
    }
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

async function onDocumentLoad() {

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

    Object.assign(exports, {
        putImageData: function(w, h, data) {
            if (gl) {
                gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, data);
                gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
                gl.flush();
            } else {
                let img = new ImageData(data, w, h);
                ctx.putImageData(img, 0, 0);
            }
        },
        putSoundData: function(slice) {
            let asrc = g_actx.createBufferSource();
            let abuf = g_actx.createBuffer(1, slice.length, g_module.is128k? 21112 : 20833); // cpufreq / AUDIO_SAMPLE / RATE_MULTIPLIER
            let data = abuf.getChannelData(0);
            for (let i = 0; i < slice.length; ++i)
                data[i] = slice[i];
            asrc.buffer = abuf;
            asrc.connect(g_actx.destination);

            asrc.start(g_audio_next);
            g_audio_next = Math.max(g_audio_next, g_actx.currentTime) + abuf.duration;
        },
    });

    let wasm = await wasm_bindgen('/pkg/raze_bg.wasm');

    let is128k = !boolURLParamDef(urlParams, '48k', false)
    g_module.is128k = is128k;
    g_module.game = wasm_bindgen.wasm_main(is128k);

    let snapshot = urlParams.get("snapshot");
    if (snapshot) {
        console.log("SNAPSHOT=", snapshot);
        await fetch_with_cors_if_needed(snapshot,
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
        await fetch_with_cors_if_needed(tape,
            bytes => {
                console.log(bytes);
                if (bytes) {
                    if (is128k) {
                        call_with_delay(1500, 100, [
                            () => wasm_bindgen.wasm_key_down(g_module.game, 0x60), //ENTER
                            () => wasm_bindgen.wasm_key_up(g_module.game, 0x60), //ENTER
                            () => onLoadTape(bytes),
                        ]);
                    } else {
                        call_with_delay(2000, 100, [
                            () => wasm_bindgen.wasm_key_down(g_module.game, 0x63), //J (LOAD)
                            () => wasm_bindgen.wasm_key_up(g_module.game, 0x63),
                            () => wasm_bindgen.wasm_key_down(g_module.game, 0x71), //SS
                            () => wasm_bindgen.wasm_key_down(g_module.game, 0x50), //P (")
                            () => wasm_bindgen.wasm_key_up(g_module.game, 0x50), //P (")
                            () => wasm_bindgen.wasm_key_down(g_module.game, 0x50), //P (")
                            () => wasm_bindgen.wasm_key_up(g_module.game, 0x50), //P (")
                            () => wasm_bindgen.wasm_key_up(g_module.game, 0x71), //SS
                            () => wasm_bindgen.wasm_key_down(g_module.game, 0x60), //ENTER
                            () => wasm_bindgen.wasm_key_up(g_module.game, 0x60), //ENTER
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
    window.addEventListener("gamepadconnected", onGamepadConnected);
    window.addEventListener("gamepaddisconnected", onGamepadDisconnected);
    g_audio_next = g_actx.currentTime;
    if (document.hasFocus())
        onFocus();

    document.querySelector('body').addEventListener('mousedown', ensureAudioRunning, false);
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
    ensureAudioRunning();
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
    wasm_bindgen.wasm_key_down(g_module.game, key);
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
    wasm_bindgen.wasm_key_up(g_module.game, key);
    ev.preventDefault();
}

let g_interval = null;
function onFocus(ev) {
    if (!g_delayed_funcs)
        wasm_bindgen.wasm_reset_input(g_module.game);
    if (g_interval === null) {
        g_interval = setInterval(function(){
            inputGamepad();
            if (g_turbo) {
                wasm_bindgen.wasm_draw_frame(g_module.game, true);
            } else while (g_audio_next - g_actx.currentTime < 0.05) {
                wasm_bindgen.wasm_draw_frame(g_module.game, false);
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
        wasm_bindgen.wasm_reset_input(g_module.game);
    if (g_interval !== null) {
        clearInterval(g_interval);
        g_interval = null;
    }
}

let g_gamepad = null;
let g_gamepadStatus = { fire: false, x: 0, y: 0 };

function onGamepadConnected(ev, connecting) {
    if (!g_gamepad) {
        g_gamepad = ev.gamepad.index;
        console.log("Using gamepad " + ev.gamepad.id);
    }
}
function onGamepadDisconnected(ev) {
    if (g_gamepad == ev.gamepad.index) {
        console.log("Removing gamepad");
        g_gamepad = null;
    }
}

function inputGamepad() {
    if (g_gamepad === null)
        return;
    let gamepad = navigator.getGamepads()[g_gamepad];
    let fire = false;
    for (let i = 0; i < 3 && i < gamepad.buttons.length && !fire; ++i)
        fire |= gamepad.buttons[i].pressed;
    let x = gamepad.axes[0];
    let y = gamepad.axes[1];
    if (x != g_gamepadStatus.x) {
        (x < -0.3? wasm_bindgen.wasm_key_down : wasm_bindgen.wasm_key_up)(g_module.game, 0x81);
        (x > 0.3? wasm_bindgen.wasm_key_down : wasm_bindgen.wasm_key_up)(g_module.game, 0x80);
    }
    if (y != g_gamepadStatus.y) {
        (y > 0.3? wasm_bindgen.wasm_key_down : wasm_bindgen.wasm_key_up)(g_module.game, 0x82);
        (y < -0.3? wasm_bindgen.wasm_key_down : wasm_bindgen.wasm_key_up)(g_module.game, 0x83);
    }
    if (fire != g_gamepadStatus.fire)
        (fire? wasm_bindgen.wasm_key_down : wasm_bindgen.wasm_key_up)(g_module.game, 0x84);
    g_gamepadStatus = { fire: fire, x: x, y: y };
}

let g_cursorKeys = null;

function handleCursorKeys(evt) {
    let sel = this.selectedIndex;
    if (window.localStorage)
        window.localStorage.setItem("cursorKeys", sel);
    g_cursorKeys = CURSOR_KEYS[sel];
    this.blur();
    if (g_module.game)
        wasm_bindgen.wasm_reset_input(g_module.game);
}

const CURSOR_KEYS = [
    //cursorkeys
    [0xf034, 0xf042, 0xf044, 0xf043, 0x71], //Shift+{5,8,6,7}, SymbolShift
    //kempston
    [0x81, 0x80, 0x82, 0x83, 0x84],
    //sinclair
    [0x44, 0x43, 0x42, 0x41, 0x40], //6, 7, 8, 9, 0
    //protek
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

//export to Rust
let exports = {
    onTapeBlock: function(index) {
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
}

function onLoadTape(data) {
    let tape_len = wasm_bindgen.wasm_load_tape(g_module.game, new Uint8Array(data));
    let xTape = resetTape();

    for (let i = 0; i < tape_len; ++i) {
        let selectable = wasm_bindgen.wasm_tape_selectable(g_module.game, i);
        let tape_name = wasm_bindgen.wasm_tape_name(g_module.game, i);
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
    wasm_bindgen.wasm_tape_seek(g_module.game, index);
}

function handleReset48k(evt) {
    resetTape();
    wasm_bindgen.wasm_drop(g_module.game);
    g_module.is128k = false;
    g_module.game = wasm_bindgen.wasm_main(g_module.is128k);
}

function handleReset128k(evt) {
    resetTape();
    wasm_bindgen.wasm_drop(g_module.game);
    g_module.is128k = true;
    g_module.game = wasm_bindgen.wasm_main(g_module.is128k);
}

function handleLoadTape(evt) {
    let x = document.createElement("input");
    x.type = "file";
    x.accept = [".tap", ".tzx", ".zip"];
    x.addEventListener('change', handleTapeSelect, false);
    x.click();
}

function handleStopTape(evt) {
    wasm_bindgen.wasm_tape_stop(g_module.game);
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
    g_module.is128k = wasm_bindgen.wasm_load_snapshot(g_module.game, g_lastSnapshot);
}

function handleSnapshot(evt) {
    console.log("snapshot");
    let data = wasm_bindgen.wasm_snapshot(g_module.game);
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

