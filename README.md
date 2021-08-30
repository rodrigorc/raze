# R.A.Z.E. A ZX Spectrum emulator

This project is part of a friendly competition to build an emulator using Rust and WebAssembly.

Check the [live version here](https://rodrigorc.github.io/raze/).

## About this project

R.A.Z.E. stands for "Rusty Attempt to a Z80 Emulator", o something like that. It was build mainly to learn Rust, but then compiling to WebAssembly is just too easy not to do it.

## Controls

You can use the keyboard mostly normally; right-control key is mapped to SymbolShift. The joystick emulation is mapped from the cursor keys for direction and left-control for fire. You can choose the type of joystick emulated in the drop-down menu `Cursor keys`.

There is also experimental support for gamepads: The gamepad will always emulate a Kempston joystick, great for multiplayer games. Remember that this is experimental, so your controller may or may not work. Patches are welcomed!

In the buttons below you can find the shortcut keys for some useful functions (such as F11 for fullscreen, etc.).

## What can it do

R.A.Z.E. emulates the ZX Spectrum 48K and 128K more or less completely. It supports loading TAP and TZX tape dumps, and Z80 snapshots. It is also able to save snapshots using the same format.

You can also load ZIP files with tapes or snapshots inside. Just do not open a ZIP with several valid files, because it won't know what to do with them.

What works and what not

 * It includes the 48K ROM and the 128K ROM. You can add `?48k=N` or `?48k=Y` to the url to force an initial mode, or use the `Reset` buttons below.
 * All documented CPU instructions and most undocumented ones are emulated.
 * CPU flags X and Y are only partially emulated.
 * CPU timing is an approximation. In particular memory contention timing is not totally accurate, but good enough for most purposes (loading tapes, border bars, etc).
 * Loading TAP and TZX files, either directly or from ZIP files. TZX support is somewhat around 90% (if you have some file that does not work and you think it should, please send it to me). You can load a tape dump directly from the URL by adding `?tape=<url>`.
 * Loading and saving Z80 snapshots. Only 48K and 128K snapshots, obviously. You can load a snapshot directly from the URL by adding `?snapshot=<url>`.
 * Currently you cannot save tape files. You can try to save it and you will hear the sound, but there is no way to record the data.
 * Emulation of the internal speaker (thanks to MAME for some ideas about sound filtering). The 128K sound generator (AY-3-8910) is also emulated.
 * Support for joystick Kempston, Sinclair and Protek. Experimental support of gamepads.
 * It uses WebGL for rendereng if available. It falls back to Canvas2D if not. You can force the Canvas2D mode adding `?webgl=N` to the url.
 * In 128k mode, it actually implements the banking of the +2A, although it does not ship the necessary ROMs. This is useful for the full RAM mode used by some programs, such as [this great Pacman emulator](http://simonowen.com/spectrum/pacemuzx/).

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

That that's all! Now you can launch a local sever such as `python -m http.server` and point your browser to the appropriate url.

## LICENSE

As most of the Rust ecosystem, the source code of this projects is published under the MIT License. See [LICENCE.MIT](LICENSE.MIT) for the full details.

ZX Spectrum ROMs are copyrighted by Amstrad. Amstrad have kindly given their permission for the redistribution of their copyrighted material but retain that copyright. See the included [ROMs.txt](ROMs.txt) file for details.
