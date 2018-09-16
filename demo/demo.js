'use strict';
let Module = {};

let utfDecoder = new TextDecoder('utf-8');
let getStr = function (ptr, len) {
    let slice = new Uint8Array(Module.memory.buffer, ptr, len);
    return utfDecoder.decode(slice);
};

var actx = new AudioContext();
var audio_next = 0;
var BORDER_COLORS = ["#000000", "#0000d7", "#d70000", "#d700d7", "#00d700", "#00d7d7", "#d7d700", "#d7d7d7"];

function onDocumentLoad() {
    let canvas = document.getElementById('game-layer');
    let ctx = canvas.getContext('2d');
    ctx.imageSmoothingEnabled = false;

    let imports = {
        env: {
            log: (ptr, len) => console.log(getStr(ptr, len)),
            alert: (ptr, len) => alert(getStr(ptr, len)),
            putImageData: (border, w, h, ptr, len) => {
                let data = new Uint8ClampedArray(Module.memory.buffer, ptr, len);
                let img = new ImageData(data, w, h);
                ctx.fillStyle = BORDER_COLORS[border];
                ctx.fillRect(0.0, 0.0, 800.0, 600.0);
                ctx.putImageData(img, 16, 12);
                ctx.drawImage(canvas, 16, 12, 256, 192, 16, 12, 768, 576);
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
            window.addEventListener('keydown', ev => onKeyDown(ev))
            window.addEventListener('keyup', ev => onKeyUp(ev))
            audio_next = actx.currentTime;
            setInterval(function(){
                if (audio_next - actx.currentTime < 0.05)
                    Module.exports.wasm_draw_frame(Module.game);
            }, 0);
        });

    document.getElementById('files').addEventListener('change', handleFileSelect, false);
    document.getElementById('snapshot').addEventListener('click', handleSnapshot, false);
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

function handleFileSelect(evt) {
    var f = evt.target.files[0];
    console.log("reading " + f.name);
    var reader = new FileReader();
    reader.onload = function(e) {
        let data = this.result;
        console.log("data " + data.byteLength);
        var ptr = Module.exports.wasm_alloc(data.byteLength);
        var d = new Uint8Array(Module.memory.buffer, ptr, data.byteLength);
        d.set(new Uint8Array(data));
        Module.exports.wasm_load_file(Module.game, ptr, data.byteLength);
    }
    var data = reader.readAsArrayBuffer(f);
}

function handleSnapshot(evt) {
    console.log("snapshot");
    let ptr = Module.exports.wasm_snapshot(Module.game);
    var data = new Uint8Array(Module.memory.buffer, ptr, 0x10000 + 29);
    var blob = new Blob([data], {type: "application/octet-stream"});
    var url = window.URL.createObjectURL(blob);
    window.open(url);
    //window.URL.revokeObjectURL(url);
    //Module.exports.wasm_free_snapshot(ptr);
}

document.addEventListener("DOMContentLoaded", onDocumentLoad);