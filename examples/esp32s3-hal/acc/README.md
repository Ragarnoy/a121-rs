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
source ~/export-esp.sh   # Espressif's libclang and the xtensa-esp32s3-elf-gcc C compiler
```

`a121-sys` finds the compiler and its newlib headers itself.

Then build and flash with `cargo run --release` (runs `espflash flash --monitor`). Logs are defmt.

Without the libraries, `cargo check --release --features a121-rs/stub_library` compile-checks the
example against stub libraries. With esp toolchain 1.99 (gcc 15.2), linking fails with
`l32r: literal placed after use`, which a minimal esp-hal program also hits, so linking and running
on hardware have not been tested with this version of the example.
