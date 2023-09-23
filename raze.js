'use strict';
import raze_init, * as wasm_bindgen from "./pkg/raze.js";
import * as base64 from "./base64.js";

let g_game;
let g_is128k;
let g_actx = new (window.AudioContext || window.webkitAudioContext)();
let g_audio_next = 0;
let g_turbo = false;
let g_turboPersistent = false;
let g_realCanvas = null;
let g_ctx = null, g_gl = null;
let g_lastSnapshot = null;
let g_delayed_funcs = null;
let g_joyTouchIdentifier = null;
let g_interval = null;
let g_gamepad = null;
let g_gamepadStatus = { fire: false, x: 0, y: 0 };
let g_cursorKeys = null;

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
            resp = await fetch('https://rodrigorivas.no-ip.org/cors/?url=' + url);
        }
        if (resp.ok)
            callback(await resp.arrayBuffer());
        else
            error();
    } catch (e) {
        error();
    }
}

function call_with_delay(first, other, args) {
    g_delayed_funcs = [first, other, args];
}

if (window.localStorage) {
    let last = window.localStorage.getItem("lastSnapshot");
    if (last) {
        g_lastSnapshot = base64.decode(last);
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

export function onTapeBlock(index) {
    console.log("Block", index);
    let xTape = document.getElementById("tape");
    for (let i = 0; i < xTape.children.length; ++i) {
        let btn = xTape.children[i];
        if (btn.dataset.index == index)
            btn.classList.add('selected');
        else
            btn.classList.remove('selected');
    }
    if (!g_turboPersistent)
        setTurbo(false);
}

export function onRZXRunning(isRunning, percent) {
    //console.log("RZX running", isRunning);
    let btn = document.getElementById('rzx_replay');
    if (isRunning) {
        btn.style.display = 'block';
    } else {
        btn.style.display = 'none';
    }
    btn.innerText = "Stop replay (" + percent + "%)";
}

export function putSoundData(slice) {
    let asrc = g_actx.createBufferSource();
    //Safari cannot use random frequencies so go with a standard 22.05 kHz
    let freq = 22050;
    let abuf = g_actx.createBuffer(1, slice.length, freq);
    if (abuf.copyToChannel) {
        abuf.copyToChannel(slice, 0);
    } else {
        let data = abuf.getChannelData(0);
        for (let i = 0; i < slice.length; ++i)
            data[i] = slice[i];
    }
    asrc.buffer = abuf;
    asrc.connect(g_actx.destination);

    asrc.start(g_audio_next);
    g_audio_next = Math.max(g_audio_next, g_actx.currentTime) + abuf.duration;
}

export function putImageData(w, h, data) {
    if (g_gl) {
        g_gl.texImage2D(g_gl.TEXTURE_2D, 0, g_gl.RGBA, w, h, 0, g_gl.RGBA, g_gl.UNSIGNED_BYTE, data);
        g_gl.drawArrays(g_gl.TRIANGLE_STRIP, 0, 4);
        g_gl.flush();
    } else {
        //data is a Uint8Array, but some browsers need a Uint8ClampedArray
        data = new Uint8ClampedArray(data.buffer, data.byteOffset, data.length);
        let img = new ImageData(data, w, h);
        g_ctx.putImageData(img, 0, 0);
    }
}

async function onDocumentLoad() {

    let urlParams = new URLSearchParams(window.location.search);
    let webgl = boolURLParamDef(urlParams, 'webgl', true)

    let canvas3d = document.getElementById('game-layer-3d');
    let canvas = document.getElementById('game-layer');

    if (webgl) {
        g_gl = canvas3d.getContext('webgl');
    }

    if (g_gl && initMyGL(g_gl)) {
        console.log("using webgl rendering");
        g_realCanvas = canvas3d;
    } else {
        if (webgl)
            console.log("webgl initialization failed, falling back to canvas");
        else
            console.log("webgl initialization skipped, falling back to canvas");
        g_gl = null;
        canvas3d.style.display = 'none';
        canvas.style.display = '';

        g_ctx = canvas.getContext('2d');
        g_ctx.imageSmoothingEnabled = false;
        g_realCanvas = canvas;
    }

    await raze_init();

    g_is128k = !boolURLParamDef(urlParams, '48k', false)
    g_game = wasm_bindgen.wasm_main(g_is128k);

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
                    if (g_is128k) {
                        call_with_delay(1500, 100, [
                            () => wasm_bindgen.wasm_key_down(g_game, 0x60), //ENTER
                            () => wasm_bindgen.wasm_key_up(g_game, 0x60), //ENTER
                            () => onLoadTape(bytes),
                        ]);
                    } else {
                        call_with_delay(2000, 100, [
                            () => wasm_bindgen.wasm_key_down(g_game, 0x63), //J (LOAD)
                            () => wasm_bindgen.wasm_key_up(g_game, 0x63),
                            () => wasm_bindgen.wasm_key_down(g_game, 0x71), //SS
                            () => wasm_bindgen.wasm_key_down(g_game, 0x50), //P (")
                            () => wasm_bindgen.wasm_key_up(g_game, 0x50), //P (")
                            () => wasm_bindgen.wasm_key_down(g_game, 0x50), //P (")
                            () => wasm_bindgen.wasm_key_up(g_game, 0x50), //P (")
                            () => wasm_bindgen.wasm_key_up(g_game, 0x71), //SS
                            () => wasm_bindgen.wasm_key_down(g_game, 0x60), //ENTER
                            () => wasm_bindgen.wasm_key_up(g_game, 0x60), //ENTER
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
    document.getElementById('reset_48k').addEventListener('click', handleReset48k, false);
    document.getElementById('reset_128k').addEventListener('click', handleReset128k, false);
    document.getElementById('load_tape').addEventListener('click', handleLoadTape, false);
    document.getElementById('stop_tape').addEventListener('click', handleStopTape, false);
    document.getElementById('snapshot').addEventListener('click', handleSnapshot, false);
    document.getElementById('load_snapshot').addEventListener('click', handleLoadSnapshot, false);
    document.getElementById('load_last_snapshot').addEventListener('click', handleLoadLastSnapshot, false);
    document.getElementById('fullscreen').addEventListener('click', handleFullscreen, false);
    document.getElementById('rzx_replay').addEventListener('click', handleRZXReplay, false);
    document.getElementById('turbo').addEventListener('click', e => handleTurbo(e, false), false);
    document.getElementById('turbo').addEventListener('dblclick', e => handleTurbo(e, true), false);
    document.getElementById('poke').addEventListener('click', handlePoke, false);
    document.getElementById('peek').addEventListener('click', handlePeek, false);
    document.getElementById('toggle_kbd').addEventListener('click', handleToggleKbd, false);
    document.getElementById('dither').addEventListener('click', handleDither, false);
    setDither(false, g_gl);

    let cursorKeys = document.getElementById('cursor_keys');
    cursorKeys.addEventListener('change', handleCursorKeys, false);
    if (window.localStorage) {
        let cursorSel = parseInt(window.localStorage.getItem("cursorKeys"));
        if (!isNaN(cursorSel))
            cursorKeys.selectedIndex = cursorSel;
    }
    handleCursorKeys.call(cursorKeys, null);

    let keyboard = document.getElementById('keyboard');
    if ('ontouchstart' in keyboard) {
        let joyBtns = document.getElementById('joy-btns');
        let joyBtnsCtx = joyBtns.getContext('2d');
        drawJoystickBtns(joyBtnsCtx, false, false, false, false);
        let joyFire = document.getElementById('joy-fire');
        let joyFireCtx = joyFire.getContext('2d');
        drawJoystickFire(joyFireCtx, false);

        //keyboard
        keyboard.querySelectorAll('.key').forEach(key => {
            key.addEventListener('touchstart', onOSKeyDown, false);
            key.addEventListener('touchend', onOSKeyUp, false);
        });

        //joystick
        let joystick = document.getElementById('joystick')
        joystick.style.display = 'grid';
        joyBtns.addEventListener('touchstart', onOSJoyDown.bind(joyBtnsCtx), false);
        joyBtns.addEventListener('touchmove', onOSJoyDown.bind(joyBtnsCtx), false);
        joyBtns.addEventListener('touchend', onOSJoyUp.bind(joyBtnsCtx), false);
        //joystick fire
        joyFire.addEventListener('touchstart', ev => {
            ev.preventDefault();
            drawJoystickFire(joyFireCtx, true);
            wasm_bindgen.wasm_key_down(g_game, g_cursorKeys[4]);
        }, false);
        joyFire.addEventListener('touchend', ev => {
            ev.preventDefault();
            drawJoystickFire(joyFireCtx, false);
            wasm_bindgen.wasm_key_up(g_game, g_cursorKeys[4]);
        }, false);
        //disable scroll/zoom
        keyboard.addEventListener('touchstart', ev => {
            ev.preventDefault();
        }, false);
        keyboard.addEventListener('touchend', ev => {
            ev.preventDefault();
        }, false);
    } else {
        let keys = document.getElementsByClassName('key');
        for (let i = 0; i < keys.length; ++i) {
            let key = keys[i];
            key.addEventListener('mousedown', onOSKeyDown, false);
            key.addEventListener('mouseup', onOSKeyUp, false);
        }
    }
}

function drawJoystickBtns(ctx, t, l, r, b) {
    let w = ctx.canvas.width;
    let h = ctx.canvas.height;
    let rad = 0.45 * Math.min(w, h);
    ctx.lineWidth = 5;
    let grd = ctx.createRadialGradient(w/2, h/2, 0, w/2, h/2, rad);
    grd.addColorStop(0, 'red');
    grd.addColorStop(1, 'black');

    for (let i = 0; i < 4; ++i) {
        ctx.beginPath();
        ctx.moveTo(w/2, h/2);
        ctx.arc(w/2, h/2, rad, i/2 * Math.PI + Math.PI/4, (i+1)/2 * Math.PI + Math.PI/4);
        let x;
        switch (i) {
        case 0: x = b; break;
        case 1: x = l; break;
        case 2: x = t; break;
        case 3: x = r; break;
        }
        ctx.fillStyle = x ? grd : 'white';
        ctx.fill();
    }
    ctx.beginPath();
    ctx.arc(w/2, h/2, rad, 0, 2 * Math.PI);
    ctx.stroke();
}
function drawJoystickFire(ctx, f) {
    let w = ctx.canvas.width;
    let h = ctx.canvas.height;
    let rad = 0.45 * Math.min(w, h);
    ctx.lineWidth = 5;

    if (f) {
        let grd = ctx.createRadialGradient(w/2, h/2, 0, w/2, h/2, rad);
        grd.addColorStop(0, 'red');
        grd.addColorStop(1, 'black');
        ctx.fillStyle = grd;
    } else {
        ctx.fillStyle = 'white';
    }

    ctx.beginPath();
    ctx.arc(w/2, h/2, rad, 0, 2*Math.PI);
    ctx.fill();
    ctx.stroke();
}


function onOSJoyDown(ev) {
    ev.preventDefault();
    let t = null;
    for (let i = 0; i < ev.changedTouches.length; ++i)
        if (g_joyTouchIdentifier == null || g_joyTouchIdentifier == ev.changedTouches[i].identifier) {
            t = ev.changedTouches[i];
            break;
        }
    if (t === null)
        return;
    g_joyTouchIdentifier = t.identifier;

    let rect = this.canvas.getBoundingClientRect();
    let x = t.clientX - rect.left - rect.width / 2;
    let y = t.clientY - rect.top - rect.height / 2;
    let rad = 0.45 * Math.min(rect.width, rect.height);
    let ang = Math.atan2(y, x);
    let hyp = Math.hypot(x, y);

    let up, down, left, right;
    if (hyp < rad * 0.3) {
        up = down = left = right = false;
    } else {
        let piece = ang / (Math.PI / 8);
/* Piece is more or less like this (negative on the top):
    8         0
    7         1
     6       2
       5 4 3
*/
        right = -2.5 < piece && piece < 2.5;
        left = piece > 5.5 || piece < -5.5;
        down = 1.5 < piece  && piece < 6.5;
        up = -6.5 < piece && piece < -1.5;
    }

    drawJoystickBtns(this, up, left, right, down);

    //first do the key_up, then the key_down, in case "cursor" mode is used
    //so that the shift key is properly pressed
    if (!left)
        wasm_bindgen.wasm_key_up(g_game, g_cursorKeys[0]);
    if (!right)
        wasm_bindgen.wasm_key_up(g_game, g_cursorKeys[1]);
    if (!down)
        wasm_bindgen.wasm_key_up(g_game, g_cursorKeys[2]);
    if (!up)
        wasm_bindgen.wasm_key_up(g_game, g_cursorKeys[3]);
    if (left)
        wasm_bindgen.wasm_key_down(g_game, g_cursorKeys[0]);
    if (right)
        wasm_bindgen.wasm_key_down(g_game, g_cursorKeys[1]);
    if (down)
        wasm_bindgen.wasm_key_down(g_game, g_cursorKeys[2]);
    if (up)
        wasm_bindgen.wasm_key_down(g_game, g_cursorKeys[3]);
}

function onOSJoyUp(ev) {
    ev.preventDefault();
    let t = null;
    for (let i = 0; i < ev.changedTouches.length; ++i)
        if (g_joyTouchIdentifier == ev.changedTouches[i].identifier) {
            t = ev.changedTouches[i];
            break;
        }
    if (t === null)
        return;
    g_joyTouchIdentifier = null;
    drawJoystickBtns(this, false, false, false, false);
    wasm_bindgen.wasm_key_up(g_game, g_cursorKeys[0]);
    wasm_bindgen.wasm_key_up(g_game, g_cursorKeys[1]);
    wasm_bindgen.wasm_key_up(g_game, g_cursorKeys[2]);
    wasm_bindgen.wasm_key_up(g_game, g_cursorKeys[3]);
}

function onOSKeyDown(ev) {
    //mouse events obey sticky keys, touch events do not
    let key = parseInt(this.dataset.code);
    ev.preventDefault();
    if (!this.classList.contains('pressed2') && !this.classList.contains('pressed')) {
        this.classList.add('pressed');
        wasm_bindgen.wasm_key_down(g_game, key);
        if (ev.type == 'mousedown' && this.classList.contains('sticky')) {
            this.classList.add('pressed2');
        }
    }
}

function onOSKeyUp(ev) {
    let key = parseInt(this.dataset.code);
    ev.preventDefault();
    if (ev.type == 'mouseup' && this.classList.contains('sticky') && this.classList.contains('pressed2')) {
        this.classList.remove('pressed2');
        //if symbolshift is pressed, caps-shift is not sticky
        if (key == 0x08 && ev.type == 'mouseup') {
            let ss = document.getElementById('ss');
            if (ss.classList.contains('pressed')) {
                this.classList.remove('pressed');
                wasm_bindgen.wasm_key_up(g_game, key);
            }
        }
        else if (key == 0x71 && ev.type == 'mouseup') {
            let caps = document.getElementById('caps');
            if (caps.classList.contains('pressed')) {
                this.classList.remove('pressed');
                wasm_bindgen.wasm_key_up(g_game, key);
            }
        }
    } else {
        this.classList.remove('pressed');
        wasm_bindgen.wasm_key_up(g_game, key);
    }
}

function onKeyDown(ev) {
    //console.log(ev.code);
    ensureAudioRunning();
    switch (ev.code) {
    case "F6":
        handleSnapshot(ev);
        ev.preventDefault();
        return;
    case "F7":
        document.getElementById('toggle_kbd').click();
        ev.preventDefault();
        return;
    case "F8":
        document.getElementById('dither').click();
        ev.preventDefault();
        return;
    case "F9":
        handleLoadLastSnapshot(ev);
        ev.preventDefault();
        return;
    case "F10":
        setTurbo(true, false);
        ev.preventDefault();
        return;
    case "F11":
        handleFullscreen(ev);
        ev.preventDefault();
        return;
    }

    let focus = document.activeElement.id;
    if (focus == 'addr' || focus == 'byte') {
        return;
    }

    let key = getKeyCode(ev);
    if (key == undefined)
        return;
    wasm_bindgen.wasm_key_down(g_game, key);
    ev.preventDefault();
}
function onKeyUp(ev) {
    switch (ev.code) {
    case "F10":
        setTurbo(false);
        ev.preventDefault();
        return;
    }

    let key = getKeyCode(ev);
    if (key == undefined)
        return;
    wasm_bindgen.wasm_key_up(g_game, key);
    ev.preventDefault();
}

function onFocus(ev) {
    if (!g_delayed_funcs)
        wasm_bindgen.wasm_reset_input(g_game);
    if (g_interval === null) {
        g_interval = setInterval(function(){
            inputGamepad();
            if (g_turbo) {
                wasm_bindgen.wasm_draw_frame(g_game, true);
            } else while (g_audio_next - g_actx.currentTime < 0.05) {
                wasm_bindgen.wasm_draw_frame(g_game, false);
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
        wasm_bindgen.wasm_reset_input(g_game);
    if (g_interval !== null) {
        clearInterval(g_interval);
        g_interval = null;
    }
}


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
        (x < -0.3? wasm_bindgen.wasm_key_down : wasm_bindgen.wasm_key_up)(g_game, 0x81);
        (x > 0.3? wasm_bindgen.wasm_key_down : wasm_bindgen.wasm_key_up)(g_game, 0x80);
    }
    if (y != g_gamepadStatus.y) {
        (y > 0.3? wasm_bindgen.wasm_key_down : wasm_bindgen.wasm_key_up)(g_game, 0x82);
        (y < -0.3? wasm_bindgen.wasm_key_down : wasm_bindgen.wasm_key_up)(g_game, 0x83);
    }
    if (fire != g_gamepadStatus.fire)
        (fire? wasm_bindgen.wasm_key_down : wasm_bindgen.wasm_key_up)(g_game, 0x84);
    g_gamepadStatus = { fire: fire, x: x, y: y };
}


function handleCursorKeys(evt) {
    let sel = this.selectedIndex;
    if (window.localStorage)
        window.localStorage.setItem("cursorKeys", sel);
    g_cursorKeys = CURSOR_KEYS[sel];
    this.blur();
    if (g_game)
        wasm_bindgen.wasm_reset_input(g_game);
}

const CURSOR_KEYS = [
    //cursorkeys
    [0x0834, 0x0842, 0x0844, 0x0843, 0x71], //Shift+{5,8,6,7}, SymbolShift
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
        return 0x08; //just like 0x00, but 0x00 is ignored by game code
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
    case "AltLeft":
    case "AltRight":
        return 0x71;
    case "KeyM":
        return 0x72;
    case "KeyN":
        return 0x73;
    case "KeyB":
        return 0x74;
    case "Backspace":
        return 0x0840; //Shift+0
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

function onLoadTape(data) {
    let tape_len = wasm_bindgen.wasm_load_tape(g_game, new Uint8Array(data));
    let xTape = resetTape();

    for (let i = 0; i < tape_len; ++i) {
        let selectable = wasm_bindgen.wasm_tape_selectable(g_game, i);
        let tape_name = wasm_bindgen.wasm_tape_name(g_game, i);
        console.log("Tape ", i, tape_name);
        if (selectable) {
            let btn = document.createElement("button");
            btn.textContent = tape_name;
            xTape.appendChild(btn);
            btn.addEventListener('click', handleTapeBlock, false);
            btn.dataset.index = i;
        }
    }
    if (xTape.firstChild)
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
    let index = btn.dataset.index;
    //evt.target.classList.add('playing');
    wasm_bindgen.wasm_tape_seek(g_game, index);
}

function handleReset48k(evt) {
    resetTape();
    wasm_bindgen.wasm_drop(g_game);
    g_is128k = false;
    g_game = wasm_bindgen.wasm_main(g_is128k);
}

function handleReset128k(evt) {
    resetTape();
    wasm_bindgen.wasm_drop(g_game);
    g_is128k = true;
    g_game = wasm_bindgen.wasm_main(g_is128k);
}

function handleLoadTape(evt) {
    let x = document.createElement("input");
    x.type = "file";
    x.accept = [".tap", ".tzx", ".zip"];
    x.addEventListener('change', handleTapeSelect, false);
    x.click();
}

function handleStopTape(evt) {
    wasm_bindgen.wasm_tape_stop(g_game);
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
    x.accept = [".z80", ".rzx", ".zip"];
    x.addEventListener('change', handleLoadSnapshotSelect, false);
    x.click();
}

function saveLastSnapshot(data) {
    g_lastSnapshot = data;
    if (g_lastSnapshot && window.localStorage) {
        window.localStorage.setItem("lastSnapshot", base64.encode(g_lastSnapshot));
    }
}

function handleLoadLastSnapshot(evt) {
    if (!g_lastSnapshot)
        return;
    g_is128k = wasm_bindgen.wasm_load_snapshot(g_game, g_lastSnapshot);
}

function handleSnapshot(evt) {
    console.log("snapshot");
    let data = wasm_bindgen.wasm_snapshot(g_game);
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
    if (document.fullscreenElement) {
        document.exitFullscreen();
    } else {
        let fs = g_realCanvas.requestFullscreen ||
            g_realCanvas.mozRequestFullScreen ||
            g_realCanvas.webkitRequestFullScreen ||
            g_realCanvas.msRequestFullscreen;
        if (fs)
            fs.call(g_realCanvas);
    }
}

function handleRZXReplay(evt) {
    wasm_bindgen.wasm_stop_rzx_replay(g_game);
}

function handleTurbo(evt, persistent) {
    setTurbo(!g_turbo, persistent);
}

function setTurbo(mode, persistent) {
    console.log(mode, persistent);
    g_turbo = mode;
    g_turboPersistent = g_turbo && persistent;
    let turbo = document.getElementById('turbo');
    if (g_turbo) {
        turbo.classList.add('active');
    } else {
        turbo.classList.remove('active');
    }
    if (g_turboPersistent) {
        turbo.classList.add('persist');
    } else {
        turbo.classList.remove('persist');
    }
}

function handlePoke(evt) {
    let addr = parseInt(document.getElementById('addr').value);
    if (isNaN(addr))
        return;
    let value = parseInt(document.getElementById('byte').value);
    if (isNaN(value))
        return;
    wasm_bindgen.wasm_poke(g_game, addr, value);
}

function handlePeek(evt) {
    let addr = parseInt(document.getElementById('addr').value);
    if (isNaN(addr))
        return;
    let value = wasm_bindgen.wasm_peek(g_game, addr);
    document.getElementById('byte').value = value;
}

function handleToggleKbd(evt) {
    let keyboard = document.getElementById('keyboard');
    if (this.classList.contains('active')) {
        this.classList.remove('active');
        keyboard.style.display = 'none'
    } else {
        this.classList.add('active');
        keyboard.style.display = 'block'
    }
}

function handleDither(evt) {
    if (this.classList.contains('active')) {
        this.classList.remove('active');
        setDither(false, g_gl);
    } else {
        this.classList.add('active');
        setDither(true, g_gl);
    }
}

function setDither(dither, gl) {
    if (dither) {
        if (gl) {
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
        } else {
            document.getElementById('game-layer').classList.remove('pixelated');
        }
    } else {
        if (gl) {
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
        } else {
            document.getElementById('game-layer').classList.add('pixelated');
        }
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
    gl.depthMask(false);
    gl.stencilMask(0);

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

