GBA FLAC Demo
----
[![Video preview](https://github.com/lifning/gba-flac-demo/releases/download/v0.1.0/suzanne_ve.gba_preview.jpg)](https://github.com/lifning/gba-flac-demo/releases/download/v0.1.0/suzanne_ve.gba_preview.mp4)

## Development setup

NOTE: You may have to wait for https://github.com/rust-lang/rust/pull/79863 to get into nightly, or else you'll get multiply-defined symbol errors at link time for __aeabi_memcpy and friends.

Either clone this repo with `git clone --recurse-submodules` or use `git submodule update --init --recursive` to get all the dependencies.

Install `youtube-dl`, `clang++`, `ffmpeg`, `mgba-qt`, `SDL2-devel`, and `arm-none-eabi-{as,gcc,ld,objcopy}` wherever Unix packages are sold.

```sh
rustup toolchain install nightly
rustup component add --toolchain nightly rust-src
```

## Build and run
```sh
cargo run --release
```

## Make ROM for hardware

(Or just for non-development-oriented emulators)

```sh
make
```

## Acknowledgements
Thanks to Lokathor & other contributors to rust-console/gba.

Thanks to Nayuki for the accessible example of FLAC decoding.

Thanks to Leonarth for the VBA-detection trick.

Thanks to endrift for mGBA, without which development would be a huge pain.
