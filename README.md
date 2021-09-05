GBA FLAC Demo
----

[![Screenshot](https://github.com/lifning/gba-flac-demo/raw/readme-assets/suzanne_ve.gba_preview.jpg)<br>Video preview](https://github.com/lifning/gba-flac-demo/raw/readme-assets/suzanne_ve.gba_preview.mp4)

## Usage notes

If you run this in something other than a hardware GBA, DS, or 3DS+open_gba_firm, be sure to enable the **interframe blending** feature in the settings, to simulate the LCD ghosting that occurs on the portables' hardware, which this demo requires for its visual effects.
- In the official GameBoy Player Start-up Disc, that's `Z Button : Options`|`Sharpness`|`Soft`
- In GameBoyInterface, that's `--filter=blend` in the .dol+cli arguments.
- In mGBA, that's `Audio/Video`|:heavy_check_mark:`Interframe blending`. You may also need to **disable** `Tools`|`Settings...`|`BIOS`|:white_large_square:`Use BIOS file if found` to avoid a crash whose cause I have yet to determine. Press the Select button after enabling Info logging (`Tools`|`View logs...`|:ballot_box_with_check:`Info`) to view licenses of third-party runtime dependency crates which require a copyright message to be reproduced in binary distributions.
- In VisualBoyAdvance-M, that's `Options`|`Video`|`Change interframe blending`; select that option until the status bar (`Options`|`Video`|`Status bar`) says "Using interframe blending #2". Note that despite having a workaround baked into the demo to prevent flickering, there will still be some visible inaccuracy in the textbox rendering.
- No$GBA and NanoboyAdvance do not seem to have an interframe blending feature, but otherwise render correctly.
- While normally an accurate emulator, I currently can't recommend higan for this demo in particular, as it struggles with rendering the scanline effects properly (without flickering), and I don't yet have a way of detecting when the demo is running in higan to enable workarounds. But for completeness / in case the problem gets fixed after this writing, it's `Settings`|`Video...`|:ballot_box_with_check:`Interframe Blending`

## Caveat for developers

The quality of a lot of the code here isn't what I'd call production-grade, or even idiomatic Rust; this was primarily a demo thrown together to demonstrate to myself that Rust was viable at all for targetting GBA hardware with a nontrivial workload (that is, more than just [drawing three pixels to a framebuffer](https://www.coranac.com/tonc/text/first.htm)). There were already growing pains in the codebase by the time I finished this (particularly the mutable static global used for interfacing with the hardware in ways that completely neglect a lot of what Rust brings to the table in terms of Fearless Concurrency:tm:)

## Development setup

Either clone this repo with `git clone --recurse-submodules` or use `git submodule update --init --recursive` to get all the dependencies.

Install `youtube-dl`, `clang++`, `auto{conf,make}`, `libtool`, `pkg-config`, `gettext`, `ffmpeg`, `mgba-qt`, `SDL2-devel`, `SDL2_image-devel`, and `arm-none-eabi-{as,gcc,ld,objcopy}` wherever Unix packages are sold. (If a cross-compile GCC toolchain for `arm-none-eabi` isn't packaged for your distribution, you may choose to simply use the one included in devkitARM from devkitPro, but devkitPro is not *required*)

You'll need at least the nightly-2021-01-15 (or so) Rust toolchain.
```sh
rustup toolchain install nightly
rustup component add --toolchain nightly rust-src
```

If you're on Windows, you'll probably want to try doing all this in WSL or LxSS or whatever they're calling it these days. [It might work!](https://ld-linux.so/)

## Build and run
```sh
cargo run --release
```

## Make ROM for hardware

(Or just for non-development-oriented emulators)

You'll need `gbafix` from either cargo (`cargo install gbafix`) or devkitPro.

```sh
make
```

You'll find the built ROM at `target/flac-demo-release.gba`.

## Acknowledgements
Thanks to Lokathor & other contributors to rust-console/gba.

Thanks to Nayuki for the accessible example of FLAC decoding.

Thanks to Leonarth for the VBA-detection trick.

Thanks to endrift for mGBA, without which development would be a huge pain.
