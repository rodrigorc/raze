# R.A.Z.E. A ZX Spectrum emulator

This project is part of a friendly competition to build an emulator using Rust and WebAssembly.

Check the [live version here](https://rodrigorc.github.io/raze/).

## About this project

R.A.Z.E. stands for "Rusty Attempt to a Z80 Emulator", or something like that. It was built mainly to learn Rust, but then compiling to WebAssembly was just too easy not to do.

## Controls

You can use the keyboard mostly normally; SymbolShift is mapped to the right-control key and to the two Alt keys. The joystick emulation is mapped from the cursor keys for direction and left-control for fire. You can choose the type of joystick emulated in the drop-down menu `Cursor keys`.

There is also experimental support for gamepads: The gamepad will always emulate a Kempston joystick, great for multiplayer games. Remember that this is experimental, so your controller may or may not work. Patches are welcomed!

In the buttons below you can find the shortcut keys for some useful functions (such as F11 for fullscreen, etc.).

## What can it do

R.A.Z.E. emulates the ZX Spectrum 48K and 128K more or less completely. It supports loading TAP and TZX tape dumps, Z80 snapshots and RZX recordings. It is also able to save snapshots using the Z80 format.

You can also load ZIP files with tapes, snapshots or recordings inside. Just do not open a ZIP with several valid files, because it won't know what to do with them.

What works and what not

 * It includes the 48K ROM, the 128K ROM and the +3 ROM. You can select the model with the `Reset` buttons below.
 * All documented CPU instructions and most undocumented ones are emulated.
 * CPU flags X and Y are only partially emulated.
 * CPU timing is an approximation. In particular memory contention timing is not totally accurate, but good enough for most purposes (loading tapes, border bars, etc).
 * Loading TAP and TZX files, either directly or from ZIP files. TZX support is somewhat around 90% (if you have some file that does not work and you think it should, please send it to me).
 * Loading and saving Z80 snapshots. Only 48K and 128K snapshots, obviously.
 * Loading DSK floppy disks (single or double sided), only available on the +3 model.
 * Currently you cannot save tape files. You can try to save it and you will hear the sound, but there is no way to record the data.
 * Emulation of the internal speaker. The 128K sound generator (AY-3-8910) is also emulated.
 * Support for joystick Kempston, Sinclair and Protek. Experimental support of gamepads.
 * It uses WebGL for rendering if available. It falls back to Canvas2D if not.
 * In 128K mode, it actually implements the banking of the +2A, although it does not ship the necessary ROMs. This is useful for the full RAM mode used by some programs, such as [this great Pacman emulator](http://simonowen.com/spectrum/pacemuzx/).
 * You can inspect and modify arbitrary memory locations with the Poke and Peek controls.
 * You can also load a custom ROM, for example the ones used by the few games that were distributed on cartridge back in the day. This forces the 48K model. Note that snapshots saved while a custom ROM is loaded may not be usable on other emulators, as they will not know how to load the ROM from the snapshot.

## URL parameters

You can customize the behaviour of the web version by adding query parameters to the URL:

| Parameter | Value(s) | Description |
|---|---|---|
| `snapshot` | URL | Load a Z80 or RZX snapshot, or a ROM file, from the given URL. |
| `tape` | URL | Load a TAP or TZX tape from the given URL and type the corresponding LOAD sequence. |
| `disk` | URL | Load a DSK disk from the given URL. Only relevant for the +3 model; if no model is given, the +3 is assumed. |
| `48k` | boolean | Start in 48K mode. |
| `plus3` | boolean | Start in +3 mode. |
| `128k` | boolean | Start in 128K mode. This is the default if no model is specified. |
| `webgl` | boolean | Use WebGL rendering (default). Set to `N` to force Canvas2D. |
| `dither` | boolean | Enable dithering. |
| `border` | `x` or `x,y` | Set the left/right (`x`) and top/bottom (`y`) border size. |
| `cursorKeys` | 0-3 | Select the joystick type: `0` cursor, `1` Kempston, `2` Sinclair, `3` Protek. |

Boolean parameters are enabled by any value whose first character is not `0`, `n` or `f` (e.g. `1`, `Y`, `yes`); an empty value also enables them.

## How to build

If you want to build this project yourself, first of all  you need a recent Rust toolchain and the `wasm32-unknown-unknown target`. If you use `rustup` just run:

```
$ rustup target add wasm32-unknown-unknown
```

You also need `wasm-pack`, so if you do not have it do:

```
$ cargo install wasm-pack
```

Then clone this repository and build it with this command:

```
$ wasm-pack build --no-typescript --target web --release
```

Alternatively you can use the following [xtask](https://github.com/matklad/cargo-xtask):

```
$ cargo xtask pack
```

And that's all! Now you can launch a local server such as `python -m http.server` and point your browser to the appropriate url.

## LICENSE

As most of the Rust ecosystem, the source code of this project is published under the MIT License. See [LICENSE.MIT](LICENSE.MIT) for the full details.

ZX Spectrum ROMs are copyrighted by Amstrad. Amstrad have kindly given their permission for the redistribution of their copyrighted material but retain that copyright. See the included [ROMs.txt](ROMs.txt) file for details.
