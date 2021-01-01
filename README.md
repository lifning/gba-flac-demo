GBA FLAC Demo
----

## Development setup

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
