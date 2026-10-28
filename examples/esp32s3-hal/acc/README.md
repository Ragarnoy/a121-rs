# Acconeer static libraries

`.cargo/config.toml` points `ACC_RSS_LIBS` at this directory. Copy the A121 libraries for the
ESP32-S3 (Xtensa) from the Acconeer SDK here:

- `libacconeer_a121.a`
- `libacc_detector_distance_a121.a`
- `libacc_detector_presence_a121.a` (only for the `presence` feature)

To point at another directory, set `ACC_RSS_LIBS` in your environment or edit `.cargo/config.toml`.

## Build environment

Install the Xtensa Rust toolchain with `espup install` (the `esp` toolchain must be 1.99 or newer),
then:

```sh
source ~/export-esp.sh   # LIBCLANG_PATH and the xtensa-esp32s3-elf-gcc C compiler
# bindgen: target the ESP32-S3 and use the toolchain's newlib headers instead of the host's
export BINDGEN_EXTRA_CLANG_ARGS_xtensa_esp32s3_none_elf="--target=xtensa-esp-elf -mcpu=esp32s3 --sysroot=$(xtensa-esp32s3-elf-gcc -print-sysroot)"
```

bindgen does not rerun when these variables change: after changing them, run `cargo clean -p a121-sys`.

Then build and flash with `cargo run --release` (runs `espflash flash --monitor`). Logs are defmt.

Without the libraries, `cargo check --release --features a121-rs/stub_library` compile-checks the
example against stubs. a121-sys builds the stubs with the host compiler for this target, so
they cannot be linked: use `cargo check`, not `cargo build`. Linking against the real libraries and running on hardware have not been
tested with this version of the example.
