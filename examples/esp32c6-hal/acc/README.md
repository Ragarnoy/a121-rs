# Acconeer static libraries

`.cargo/config.toml` points `ACC_RSS_LIBS` at this directory. Copy the A121 libraries for the
ESP32-C6 (RISC-V, `rv32imac`/`ilp32`) from the Acconeer SDK here:

- `libacconeer_a121.a`
- `libacc_detector_distance_a121.a`
- `libacc_detector_presence_a121.a` (only for the `presence` feature)

To point at another directory, set `ACC_RSS_LIBS` in your environment or edit `.cargo/config.toml`.

## Build environment

`a121-sys` compiles a small C file and generates bindings with `bindgen`, so it needs a RISC-V C
toolchain with newlib on `PATH`: `riscv32-esp-elf-gcc` (from espup or ESP-IDF) or
`riscv32-unknown-elf-gcc`. It finds the compiler and its headers itself. To use another compiler,
set `CC_riscv32imac_unknown_none_elf`.

Then build and flash with `cargo run --release` (runs `espflash flash --monitor`). Logs are defmt over
the USB serial JTAG.

Without the libraries, `cargo build --release --features a121-rs/stub_library` builds the example
against stub libraries, which link but do nothing. Linking against the real libraries and running
on hardware have not been tested with this version of the example.
