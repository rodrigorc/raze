'use strict';
let Module = {};

let utfDecoder = new TextDecoder('utf-8');
let getStr = function (ptr, len) {
    let slice = new Uint8Array(Module.memory.buffer, ptr, len);
    return utfDecoder.decode(slice);
};

var actx = new AudioContext();
var audio_next = 0;
var turbo = false;

function onDocumentLoad() {
    let canvas = document.getElementById('game-layer');
    let ctx = canvas.getContext('2d');
    ctx.imageSmoothingEnabled = false;

    let imports = {
        env: {
            consolelog: (ptr, len) => console.log(getStr(ptr, len)),
            alert: (ptr, len) => alert(getStr(ptr, len)),
            putImageData: (w, h, ptr, len) => {
                let data = new Uint8ClampedArray(Module.memory.buffer, ptr, len);
                let img = new ImageData(data, w, h);
                ctx.putImageData(img, 0, 0);
                ctx.drawImage(canvas, 0, 0, w, h, 0, 0, 3*w, 3*h);
            },
            putSoundData: (ptr, len) => {
                let asrc = actx.createBufferSource();
                let abuf = actx.createBuffer(1, len, len * 50); // 50ms
                let data = abuf.getChannelData(0);
                let slice = new Uint8Array(Module.memory.buffer, ptr, len);
                for (let i = 0; i < len; ++i)
                    data[i] = slice[i] ? 1 : -1;
                asrc.buffer = abuf;
                asrc.connect(actx.destination);

                asrc.start(audio_next);
                audio_next = Math.max(audio_next, actx.currentTime) + abuf.duration;
            }
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
            Object.assign(Module, {
                wasm: wasm,
                exports: exports,
                memory: exports.memory,
            });
            Module.game = exports.wasm_main();
            window.addEventListener('keydown', onKeyDown)
            window.addEventListener('keyup', onKeyUp)
            window.addEventListener('focus', onFocus)
            window.addEventListener('blur', onBlur)
            audio_next = actx.currentTime;
            setInterval(function(){
                if (turbo) {
                    Module.exports.wasm_draw_frame(Module.game, true);
                } else if (audio_next - actx.currentTime < 0.05)
                    Module.exports.wasm_draw_frame(Module.game, false);
            }, 0);
        });

    document.getElementById('load_tape').addEventListener('click', handleLoadTape, false);
    document.getElementById('snapshot').addEventListener('click', handleSnapshot, false);
    document.getElementById('load_snapshot').addEventListener('click', handleLoadSnapshot, false);
    document.getElementById('fullscreen').addEventListener('click', handleFullscreen, false);
    document.getElementById('turbo').addEventListener('click', handleTurbo, false);
}

function onKeyDown(ev) {
    //console.log(ev.code);
    let key = getKeyCode(ev);
    if (key == undefined)
        return;
    Module.exports.wasm_key_down(Module.game, key);
    ev.preventDefault();
}
function onKeyUp(ev) {
    let key = getKeyCode(ev);
    if (key == undefined)
        return;
    Module.exports.wasm_key_up(Module.game, key);
    ev.preventDefault();
}
function onFocus(ev) {
    Module.exports.wasm_reset_input(Module.game);
}
function onBlur(ev) {
    Module.exports.wasm_reset_input(Module.game);
}

function getKeyCode(ev) {
    switch (ev.code) {
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
        return 0xf040; //Shift+0
    case "ArrowLeft":
        return 0xf034; //Shift+5
    case "ArrowRight":
        return 0xf042; //Shift+8
    case "ArrowDown":
        return 0xf044; //Shift+6
    case "ArrowUp":
        return 0xf043; //Shift+7
    //Joystick
    case "Numpad0":
    case "ControlLeft":
    case "ControlRight":
        return 0x84;
    case "Numpad8":
        return 0x83;
    case "Numpad4":
        return 0x81;
    case "Numpad5":
        return 0x82;
    case "Numpad6":
        return 0x80;
    default:
        return null;
    }
}

function handleTapeSelect(evt) {
    var f = evt.target.files[0];
    console.log("reading " + f.name);
    var reader = new FileReader();
    reader.onload = function(e) {
        let data = this.result;
        console.log("data " + data.byteLength);
        var ptr = Module.exports.wasm_alloc(data.byteLength);
        var d = new Uint8Array(Module.memory.buffer, ptr, data.byteLength);
        d.set(new Uint8Array(data));
        Module.exports.wasm_load_tape(Module.game, ptr, data.byteLength);
    }
    reader.readAsArrayBuffer(f);
}

function handleLoadTape(evt) {
    var x = document.createElement("input");
    x.type = "file";
    x.accept = ".tap";
    x.addEventListener('change', handleTapeSelect, false);
    x.click();
}

function handleSnapshotSelect(evt) {
    var f = evt.target.files[0];
    console.log("reading " + f.name);
    var reader = new FileReader();
    reader.onload = function(e) {
        let data = this.result;
        console.log("data " + data.byteLength);
        var ptr = Module.exports.wasm_alloc(data.byteLength);
        var d = new Uint8Array(Module.memory.buffer, ptr, data.byteLength);
        d.set(new Uint8Array(data));
        Module.exports.wasm_load_snapshot(Module.game, ptr, data.byteLength);
    }
    reader.readAsArrayBuffer(f);
}

function handleLoadSnapshot(evt) {
    var x = document.createElement("input");
    x.type = "file";
    x.accept = ".spec";
    x.addEventListener('change', handleSnapshotSelect, false);
    x.click();
}

function handleSnapshot(evt) {
    console.log("snapshot");
    const SNAPSHOT_SIZE = 0x10000 + 29;
    let ptr = Module.exports.wasm_snapshot(Module.game);
    var data = new Uint8Array(Module.memory.buffer, ptr, SNAPSHOT_SIZE);
    var blob = new Blob([data], {type: "application/octet-stream"});
    var url = window.URL.createObjectURL(blob);

    var a = document.createElement("a");
    a.style = "display: none";
    a.href = url;
    a.download = "snapshot.spec";
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    window.URL.revokeObjectURL(url);
    Module.exports.wasm_free_snapshot(ptr, SNAPSHOT_SIZE);
}
function handleFullscreen(evt) {
    console.log("fullscreen");
    var canvas = document.getElementById('game-layer');
    var fs = canvas.requestFullscreen || canvas.mozRequestFullScreen || canvas.webkitRequestFullScreen || canvas.msRequestFullscreen;
    if (fs)
        fs.call(canvas);
}

function handleTurbo(evt) {
    turbo = this.checked;
}

document.addEventListener("DOMContentLoaded", onDocumentLoad);
