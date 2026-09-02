'use strict';
import raze_init, * as wasm_bindgen from "./pkg/raze_web.js?v=__VERSION__";
import * as base64 from "./base64.js?v=__VERSION__";


const SPEC48K = 0;
const SPEC128K = 1;
const PLUS3 = 2;

let g_game = null;
let g_border = { x: 5, y: 4 };
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
let g_cursorKeys = null;
let g_gamepadKeys = null;
let g_gamepadStatus = { fire: false, left: false, right: false, up: false, down: false };

// Options:
// * snapshot: Uint8Array
// * model: number
function createGame(options = {}) {
    let builder = wasm_bindgen.wasm_builder_new();

    // fixed options
    wasm_bindgen.wasm_builder_set_border(builder, g_border.x, g_border.y);

    // variable options
    if (options.snapshot !== undefined) {
        wasm_bindgen.wasm_builder_set_snapshot(builder, options.snapshot);
    } else if (options.model !== undefined) {
        wasm_bindgen.wasm_builder_set_model(builder, options.model);
    }

    try {
        let new_game = wasm_bindgen.wasm_builder_build(builder);
        if (g_game)
            wasm_bindgen.wasm_game_drop(g_game);
        g_game = new_game;
    } catch (e) {
        alert(e.message);
        return;
    }

    g_delayed_funcs = null;
    resetTape();
    resetDisk();
    doPlay();
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

// Delays in frames (20ms each)
function call_with_delay(first_delay, delay, funcs) {
    g_delayed_funcs = { first_delay, delay, funcs };
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

let g_lastTapeBlock = null;
function onTapeBlock(index) {
    if (g_lastTapeBlock == index)
        return;
    g_lastTapeBlock = index;
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

let g_rzxPercent = null;
function onRZXRunning(percent) {
    // quick path
    if (g_rzxPercent == percent)
        return;
    g_rzxPercent = percent;

    //console.log("RZX running", isRunning);
    let btn = document.getElementById('rzx_replay');
    let container = document.getElementById('buttons');
    if (percent != null) {
        btn.innerText = "Stop replay (" + percent + "%)";
        container.classList.add("rzx_mode");
    } else {
        container.classList.remove("rzx_mode");
    }
}

function putSoundData(slice) {
    if (g_actx.state == "suspended") {
        g_actx.resume();
        return;
    }

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

function putImageData(w, h, data) {
    if (g_gl) {
        g_gl.texImage2D(g_gl.TEXTURE_2D, 0, g_gl.RGBA, w, h, 0, g_gl.RGBA, g_gl.UNSIGNED_BYTE, data);
        g_gl.drawArrays(g_gl.TRIANGLE_STRIP, 0, 4);
        g_gl.flush();
    } else {
        let img = new ImageData(data, w, h);
        g_ctx.putImageData(img, 0, 0);
    }
}

function putNewFrameInfo(rzx, tape_block) {
    onRZXRunning(rzx);
    onTapeBlock(tape_block);
}

function parse2Ints(str) {
    if (!str)
        return null;
    const [x, y] = str.split(',').map(x => parseInt(x));
    if (isNaN(x))
        return null;
    if (isNaN(y))
        return { x, y: x };
    return { x, y };
}

async function onDocumentLoad() {

    let urlParams = new URLSearchParams(window.location.search);
    let webgl = boolURLParamDef(urlParams, 'webgl', true);
    let dither = boolURLParamDef(urlParams, 'dither', false);
    let border = parse2Ints(urlParams.get("border"));
    if (border)
        g_border = border;

    let screen_width = 2 * g_border.x + 256;
    let screen_height = 2 * g_border.y + 192;

    let canvas3d = document.getElementById('game-layer-3d');
    let canvas = document.getElementById('game-layer');

    canvas3d.width = canvas.width = screen_width;
    canvas3d.height = canvas.height = screen_height;

    if (webgl) {
        g_gl = canvas3d.getContext('webgl');
    }

    if (g_gl && initMyGL(g_gl)) {
        console.log("using webgl rendering");
        canvas.classList.add("hidden");
        g_realCanvas = canvas3d;
    } else {
        if (webgl)
            console.log("webgl initialization failed, falling back to canvas");
        else
            console.log("webgl initialization skipped, falling back to canvas");
        g_gl = null;
        canvas3d.classList.add("hidden");

        g_ctx = canvas.getContext('2d');
        g_ctx.imageSmoothingEnabled = false;
        g_realCanvas = canvas;
    }

    await raze_init({
        module_or_path: './pkg/raze_web_bg.wasm?v=__VERSION__',
    });
    wasm_bindgen.wasm_main();

    let gameOpts = { };
    if (boolURLParamDef(urlParams, '48k', false))
        gameOpts.model = SPEC48K;
    else if (boolURLParamDef(urlParams, 'plus3', false))
        gameOpts.model = PLUS3;
    else if (urlParams.has("disk"))
        // disk drive only in plus3, so if there is disk= but no model=, assume +3
        gameOpts.model = PLUS3;
    else
        gameOpts.model = SPEC128K;

    console.log("Spec model", gameOpts.model);


    let snapshot = urlParams.get("snapshot");
    if (snapshot) {
        console.log("SNAPSHOT=", snapshot);
        await fetch_with_cors_if_needed(snapshot,
            bytes => {
                saveLastSnapshot(new Uint8Array(bytes));
                gameOpts.snapshot = g_lastSnapshot;
                // If there is a snapshot ignore the selected model, this will disable the autotype if a tape/disk is autoloaded.
                // The autotype is only useful if the system is reset.
                delete gameOpts.model;
            },
            error => {
                alert("Cannot download file " + snapshot);
            }
        );
    }

    createGame(gameOpts);

    // Load tape/disk. If there is no snapshot type the initial sequence to start the load procedure
    let tape = urlParams.get("tape");
    let disk = urlParams.get("disk");
    if (tape) {
        console.log("TAPE=", tape);
        await fetch_with_cors_if_needed(tape,
            bytes => {
                if (bytes) {
                    switch (gameOpts.model) {
                    case SPEC48K:
                        // 48K loading sequence: typìng LOAD ""
                        call_with_delay(100, 5, [
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
                        break;
                    case SPEC128K:
                    case PLUS3:
                        // 128K loading sequence: enter in the load menu
                        // +3 loading sequence: same as 128K but a slightly longer delay because of the floppy
                        call_with_delay(gameOpts.model == PLUS3 ? 100 : 75, 5, [
                            () => wasm_bindgen.wasm_key_down(g_game, 0x60), //ENTER
                            () => wasm_bindgen.wasm_key_up(g_game, 0x60), //ENTER
                            () => onLoadTape(bytes),
                        ]);
                        break;
                    case undefined:
                        onLoadTape(bytes);
                        break;

                    }
                }
            },
            error => {
                alert("Cannot download file " + tape);
            }
        );
    } else if (disk) {
        console.log("DISK=", disk);
        await fetch_with_cors_if_needed(disk,
            bytes => {
                // Contrary to tapes, the disk is best loaded first, and then press enter, else
                // the floppy may not be detected and the system will default to loading the tape.
                if (onLoadDisk(bytes)) {
                    if (gameOpts.model == PLUS3) {
                        call_with_delay(100, 5, [
                            () => wasm_bindgen.wasm_key_down(g_game, 0x60), //ENTER
                            () => wasm_bindgen.wasm_key_up(g_game, 0x60), //ENTER
                        ]);
                    }
                }
            },
            error => {
                alert("Cannot download file " + disk);
            }
        );
    }

    g_actx.addEventListener('statechange', onAudioStateChanged, false);
    onAudioStateChanged();
    g_audio_next = g_actx.currentTime;
    doPlay();

    window.addEventListener('keydown', onKeyDown)
    window.addEventListener('keyup', onKeyUp)
    window.addEventListener('blur', onBlur)
    window.addEventListener("gamepadconnected", onGamepadConnected);
    window.addEventListener("gamepaddisconnected", onGamepadDisconnected);

    document.getElementById('reset_48k').addEventListener('click', e => handleReset(e, SPEC48K), false);
    document.getElementById('reset_128k').addEventListener('click', e => handleReset(e, SPEC128K), false);
    document.getElementById('reset_plus3').addEventListener('click', e => handleReset(e, PLUS3), false);
    document.getElementById('load_tape').addEventListener('click', handleLoadTape, false);
    document.getElementById('stop_tape').addEventListener('click', handleStopTape, false);
    document.getElementById('snapshot').addEventListener('click', handleSnapshot, false);
    document.getElementById('load_snapshot').addEventListener('click', handleLoadSnapshot, false);
    document.getElementById('load_last_snapshot').addEventListener('click', handleLoadLastSnapshot, false);
    document.getElementById('load_disk').addEventListener('click', handleLoadDisk, false);
    document.getElementById('fullscreen').addEventListener('click', handleFullscreen, false);
    document.getElementById('rzx_replay').addEventListener('click', handleRZXReplay, false);
    document.getElementById('turbo').addEventListener('click', e => handleTurbo(e, false), false);
    document.getElementById('turbo').addEventListener('dblclick', e => handleTurbo(e, true), false);
    document.getElementById('pause').addEventListener('click', handlePause, false);
    document.getElementById('poke').addEventListener('click', handlePoke, false);
    document.getElementById('peek').addEventListener('click', handlePeek, false);
    document.getElementById('toggle_kbd').addEventListener('click', handleToggleKbd, false);
    let btnDither = document.getElementById('dither');
    btnDither.addEventListener('click', handleDither, false);

    if (dither) {
        btnDither.classList.add('active');
    }
    setDither(dither, g_gl);

    let cursorKeys = document.getElementById('cursor_keys');
    cursorKeys.addEventListener('change', handleCursorKeys, false);
    let gamepadKeys = document.getElementById('gamepad_keys');
    gamepadKeys.addEventListener('change', handleGamepadKeys, false);

    let cursorSel = parseInt(urlParams.get('cursorKeys'));
    let gamepadSel = cursorSel;
    if (isNaN(cursorSel)) {
        if (window.localStorage) {
            cursorSel = parseInt(window.localStorage.getItem("cursorKeys"));
            gamepadSel = parseInt(window.localStorage.getItem("gamepadKeys"));
        }
    }
    if (!isNaN(cursorSel)) {
        cursorKeys.selectedIndex = cursorSel;
    } else {
        // default cursorKeys is "cursor"
        cursorKeys.selectedIndex = 0;
    }
    if (!isNaN(gamepadSel)) {
        gamepadKeys.selectedIndex = gamepadSel;
    } else {
        // default gamepadKeys is "kempston"
        gamepadKeys.selectedIndex = 1;
    }
    handleCursorKeys.call(cursorKeys, null);
    handleGamepadKeys.call(gamepadKeys, null);

    let keyboard = document.getElementById('keyboard');
    if ('ontouchstart' in keyboard) {
        let joyBtns = document.getElementById('joy-btns');
        let joyBtnsCtx = joyBtns.getContext('2d');
        drawJoystickBtns(joyBtnsCtx, false, false, false, false);
        let joyFire = document.getElementById('joy-fire');
        let joyFireCtx = joyFire.getContext('2d');
        drawJoystickFire(joyFireCtx, false);

        //joystick
        let joystick = document.getElementById('joystick')
        joystick.classList.remove("hidden");
        joyBtns.addEventListener('touchstart', onOSJoyDown.bind(joyBtnsCtx), false);
        joyBtns.addEventListener('touchmove', onOSJoyDown.bind(joyBtnsCtx), false);
        joyBtns.addEventListener('touchend', onOSJoyUp.bind(joyBtnsCtx), false);
        //joystick fire
        joyFire.addEventListener('touchstart', ev => {
            ev.preventDefault();
            if (g_delayed_funcs)
                return;
            drawJoystickFire(joyFireCtx, true);
            wasm_bindgen.wasm_key_down(g_game, g_cursorKeys.fire);
        }, false);
        joyFire.addEventListener('touchend', ev => {
            ev.preventDefault();
            if (g_delayed_funcs)
                return;
            drawJoystickFire(joyFireCtx, false);
            wasm_bindgen.wasm_key_up(g_game, g_cursorKeys.fire);
        }, false);
        //disable scroll/zoom
        keyboard.addEventListener('touchstart', ev => {
            ev.preventDefault();
        }, false);
        keyboard.addEventListener('touchend', ev => {
            ev.preventDefault();
        }, false);
    }

    //keyboard
    keyboard.querySelectorAll('.key').forEach(key => {
        key.addEventListener('pointerdown', onOSKeyDown, false);
        key.addEventListener('pointerup', onOSKeyUp, false);
    });


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
    if (g_delayed_funcs)
        return;

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
        wasm_bindgen.wasm_key_up(g_game, g_cursorKeys.left);
    if (!right)
        wasm_bindgen.wasm_key_up(g_game, g_cursorKeys.right);
    if (!down)
        wasm_bindgen.wasm_key_up(g_game, g_cursorKeys.down);
    if (!up)
        wasm_bindgen.wasm_key_up(g_game, g_cursorKeys.up);
    if (left)
        wasm_bindgen.wasm_key_down(g_game, g_cursorKeys.left);
    if (right)
        wasm_bindgen.wasm_key_down(g_game, g_cursorKeys.right);
    if (down)
        wasm_bindgen.wasm_key_down(g_game, g_cursorKeys.down);
    if (up)
        wasm_bindgen.wasm_key_down(g_game, g_cursorKeys.up);
}

function onOSJoyUp(ev) {
    ev.preventDefault();
    if (g_delayed_funcs)
        return;

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
    wasm_bindgen.wasm_key_up(g_game, g_cursorKeys.left);
    wasm_bindgen.wasm_key_up(g_game, g_cursorKeys.right);
    wasm_bindgen.wasm_key_up(g_game, g_cursorKeys.down);
    wasm_bindgen.wasm_key_up(g_game, g_cursorKeys.up);
}

function onOSKeyDown(ev) {
    ev.preventDefault();
    if (g_delayed_funcs)
        return;

    this.setPointerCapture(ev.pointerId);

    //mouse events obey sticky keys, touch events do not
    let key = parseInt(this.dataset.code);
    if (!this.classList.contains('pressed2') && !this.classList.contains('pressed')) {
        this.classList.add('pressed');
        wasm_bindgen.wasm_key_down(g_game, key);
        if (ev.pointerType == 'mouse' && this.classList.contains('sticky')) {
            this.classList.add('pressed2');
        }
    }
}

function onOSKeyUp(ev) {
    ev.preventDefault();

    let key = parseInt(this.dataset.code);
    if (ev.pointerType == 'mouse' && this.classList.contains('sticky') && this.classList.contains('pressed2')) {
        this.classList.remove('pressed2');
        //if symbolshift is pressed, caps-shift is not sticky
        if (key == 0x08 && ev.pointerType == 'mouse') {
            let ss = document.getElementById('ss');
            if (ss.classList.contains('pressed')) {
                this.classList.remove('pressed');
                wasm_bindgen.wasm_key_up(g_game, key);
            }
        }
        else if (key == 0x71 && ev.pointerType == 'mouse') {
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
    case "Escape":
        handlePause(ev);
        ev.preventDefault();
    }

    let focus = document.activeElement.id;
    if (focus == 'addr' || focus == 'byte') {
        return;
    }

    let key = getKeyCode(ev);
    if (key == undefined)
        return;
    ev.preventDefault();
    if (g_delayed_funcs)
        return;

    wasm_bindgen.wasm_key_down(g_game, key);
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

    ev.preventDefault();
    if (g_delayed_funcs)
        return;

    wasm_bindgen.wasm_key_up(g_game, key);
}

let g_frame_next = 0;

function doFrame() {
    inputGamepad();
    if (g_turbo && !g_delayed_funcs) {
        wasm_bindgen.wasm_do_frame(g_game, true, putNewFrameInfo);
        g_animationFrame ??= window.requestAnimationFrame(drawAnimationFrame);
    } else {
        let time = performance.now();
        // In case we are underpowered, do not do more than N emulated frames per real frame
        for (let i = 0; i < 5; ++i) {
            if (!g_turbo) {
                if (g_actx.state == "running") {
                    // in seconds
                    if (g_audio_next - g_actx.currentTime >= 0.05) {
                        g_frame_next = time + 20;
                        break;
                    }
                } else {
                    // in milliseconds
                    if (g_frame_next - time >= 20)
                        break;
                    g_frame_next += 20;
                }
            }

            wasm_bindgen.wasm_do_frame(g_game, false, putNewFrameInfo);
            wasm_bindgen.wasm_get_audio(g_game, putSoundData);
            g_animationFrame ??= window.requestAnimationFrame(drawAnimationFrame);

            run_delayed_funcs();
        }
    }
}

// The actual image is drawn only once, in an animationFrame
let g_animationFrame = null;
function drawAnimationFrame() {
    g_animationFrame = null;
    wasm_bindgen.wasm_get_image(g_game, putImageData);
}

function run_delayed_funcs() {
    if (g_delayed_funcs == null)
        return;

    if ((g_delayed_funcs.first_delay -= 1) <= 0) {
        let f = g_delayed_funcs.funcs.shift();
        if (f) {
            f();
            g_delayed_funcs.first_delay = g_delayed_funcs.delay;
        } else {
            g_delayed_funcs = null;
        }
    }
}

function onBlur(ev) {
    if (!g_delayed_funcs)
        wasm_bindgen.wasm_reset_input(g_game);
}

function onAudioStateChanged(e) {
    let running = g_actx.state == "running";
    let audio_indicator = document.getElementById('muted');
    if (running)
        audio_indicator.classList.add('hidden');
    else
        audio_indicator.classList.remove('hidden');
}



function onGamepadConnected(ev, connecting) {
    if (g_gamepad === null) {
        g_gamepad = ev.gamepad.index;
        console.log("Using gamepad " + ev.gamepad.id);
        let cursorKeys = document.getElementById('cursor_keys_p');
        cursorKeys.classList.add("with_gamepad");
    }
}
function onGamepadDisconnected(ev) {
    if (g_gamepad == ev.gamepad.index) {
        console.log("Removing gamepad");
        g_gamepad = null;
        let cursorKeys = document.getElementById('cursor_keys_p');
        cursorKeys.classList.remove("with_gamepad");
    }
}

function inputGamepad() {
    if (g_delayed_funcs != null)
        return;

    if (g_gamepad === null)
        return;
    let gamepad = navigator.getGamepads()[g_gamepad];
    let fire = false;
    for (let i = 0; i < 3; ++i) {
        if (gamepad.buttons[i]?.pressed)
            fire = true;
    }
    let up = gamepad.buttons[12]?.pressed ?? false;
    let down = gamepad.buttons[13]?.pressed ?? false;
    let left = gamepad.buttons[14]?.pressed ?? false;
    let right = gamepad.buttons[15]?.pressed ?? false;

    let x = gamepad.axes[0];
    let y = gamepad.axes[1];

    if (x < -0.3)
        left = true;
    else if (x > 0.3)
        right = true;

    if (y < -0.3)
        up = true;
    else if (y > 0.3)
        down = true;

    // If nothing changed, do nothing.
    // Sending the keys down/up events will not change this controller
    // but will prevent the keyboard/virtual-joy to be usable, because
    // this function is called every frame.
    if (left != g_gamepadStatus.left ||
        right != g_gamepadStatus.right ||
        up !=  g_gamepadStatus.up ||
        down != g_gamepadStatus.down)
    {
        //first do the key_up, then the key_down, in case "cursor" mode is used
        //so that the shift key is properly pressed
        if (!left)
            wasm_bindgen.wasm_key_up(g_game, g_gamepadKeys.left);
        if (!right)
            wasm_bindgen.wasm_key_up(g_game, g_gamepadKeys.right);
        if (!down)
            wasm_bindgen.wasm_key_up(g_game, g_gamepadKeys.down);
        if (!up)
            wasm_bindgen.wasm_key_up(g_game, g_gamepadKeys.up);

        if (left)
            wasm_bindgen.wasm_key_down(g_game, g_gamepadKeys.left);
        if (right)
            wasm_bindgen.wasm_key_down(g_game, g_gamepadKeys.right);
        if (down)
            wasm_bindgen.wasm_key_down(g_game, g_gamepadKeys.down);
        if (up)
            wasm_bindgen.wasm_key_down(g_game, g_gamepadKeys.up);
    }

    if (fire != g_gamepadStatus.fire) {
        // The fire never uses shift, so we are safe here
        if (fire)
            wasm_bindgen.wasm_key_down(g_game, g_gamepadKeys.fire);
        else
            wasm_bindgen.wasm_key_up(g_game, g_gamepadKeys.fire);
    }

    g_gamepadStatus = { fire, left, right, up, down };

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

function handleGamepadKeys(evt) {
    let sel = this.selectedIndex;
    if (window.localStorage)
        window.localStorage.setItem("gamepadKeys", sel);
    g_gamepadKeys = CURSOR_KEYS[sel];
    this.blur();
    if (g_game)
        wasm_bindgen.wasm_reset_input(g_game);
}

const CURSOR_KEYS = [
    //cursorkeys
    { left: 0x0834, right: 0x0842, down: 0x0844, up: 0x0843, fire: 0x71 }, //Shift+{5,8,6,7}, SymbolShift
    //kempston
    { left: 0x81, right: 0x80, down: 0x82, up: 0x83, fire: 0x84 },
    //sinclair
    { left: 0x44, right: 0x43, down: 0x42, up: 0x41, fire: 0x40 }, //6, 7, 8, 9, 0
    //protek
    { left: 0x34, right: 0x42, down: 0x44, up: 0x43, fire: 0x40 }, //5, 8, 6, 7, 0
];

function getKeyCode(ev) {
    switch (ev.code) {
    case "ArrowLeft":
        return g_cursorKeys.left;
    case "ArrowRight":
        return g_cursorKeys.right;
    case "ArrowDown":
        return g_cursorKeys.down;
    case "ArrowUp":
        return g_cursorKeys.up;
    case "ControlLeft":
        return g_cursorKeys.fire;

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
    let tape_len;
    try {
        tape_len = wasm_bindgen.wasm_tape_load(g_game, new Uint8Array(data));
    } catch (e) {
        alert(e.message);
        return;
    }

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

function resetDisk() {
    let disk = document.getElementById("load_disk");
    // show the disk as not-inserted
    disk.classList.remove('active');

    // but the button may not be visible
    let model = wasm_bindgen.wasm_game_model(g_game);
    if (model == PLUS3) {
        disk.classList.remove("hidden");
    } else {
        disk.classList.add("hidden");
    }
}

function onLoadDisk(data) {
    try {
        wasm_bindgen.wasm_disk_load(g_game, new Uint8Array(data));
    } catch (e) {
        alert(e.message);
        return false;
    }
    let disk = document.getElementById('load_disk');
    disk.classList.add('active');
    return true;
}

function handleDiskSelect(evt) {
    let f = evt.target.files[0];
    console.log("reading " + f.name);
    let reader = new FileReader();
    reader.onload = function(e) { onLoadDisk(this.result); };
    reader.readAsArrayBuffer(f);
}

function handleReset(evt, model) {
    createGame({ model: model });
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

function handleLoadDisk(evt) {
    if (this.classList.contains('active')) {
        wasm_bindgen.wasm_disk_eject(g_game);
        this.classList.remove('active');
        return;
    }

    let x = document.createElement("input");
    x.type = "file";
    x.accept = [".dsk", ".zip"];
    x.addEventListener('change', handleDiskSelect, false);
    x.click();
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
    x.accept = [".z80", ".rzx", ".zip", ".bin"];
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
    createGame({ snapshot: g_lastSnapshot })
}

function handleSnapshot(evt) {
    console.log("snapshot");
    let data = wasm_bindgen.wasm_snapshot(g_game);
    let blob = new Blob([data], {type: "application/octet-stream"});
    let url = window.URL.createObjectURL(blob);

    saveLastSnapshot(data);

    let a = document.createElement("a");
    a.style.display = "none";
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

function doPause() {
    if (!g_delayed_funcs)
        wasm_bindgen.wasm_reset_input(g_game);

    let pause = document.getElementById('pause');
    pause.classList.add('active');

    if (g_interval !== null) {
        window.clearInterval(g_interval);
        g_interval = null;
    }
}

function doPlay() {
    if (!g_delayed_funcs)
        wasm_bindgen.wasm_reset_input(g_game);

    let pause = document.getElementById('pause');
    pause.classList.remove('active');

    if (g_interval === null) {
        g_frame_next = performance.now() + 20;
        g_interval = setInterval(doFrame, 0);
    }
}

function handlePause(evt) {
    if (g_interval == null)
        doPlay();
    else
        doPause();
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
        keyboard.classList.add("hidden");
    } else {
        this.classList.add('active');
        keyboard.classList.remove("hidden");
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
        g_realCanvas.classList.remove('pixelated');
        if (gl) {
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
        }
    } else {
        g_realCanvas.classList.add('pixelated');
        if (gl) {
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
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

