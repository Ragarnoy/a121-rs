# a121-rs

[![docs.rs](https://docs.rs/a121-rs/badge.svg)](https://docs.rs/a121-rs)
[![crates.io](https://img.shields.io/crates/v/a121-rs.svg)](https://crates.io/crates/a121-rs)
[![crates.io](https://img.shields.io/crates/d/a121-rs.svg)](https://crates.io/crates/a121-rs)

_a121-rs_ is a Rust library providing a high-level abstraction for interfacing with the Acconeer A121 V-Band radar sensor. Designed for use in embedded systems, it offers asynchronous operation through embedded-hal traits, making it a perfect fit for no_std environments.

_a121-rs_ aims to simplify the development process for applications requiring accurate distance measurements and presence detection, leveraging the unique capabilities of the A121 radar sensor.

## Status

Being maintained but definitely looking for assistance

## Features

_a121-rs_ comes with a host of features designed to make working with the A121 sensor as straightforward as possible:

- **No Standard Library Dependencies**: Fully compatible with `no_std` for embedded systems use.
- **Embedded-HAL Async**: Utilizes asynchronous traits from `embedded-hal` for non-blocking sensor operation.
- **Configurable**: Provides a flexible API to configure the radar sensor for various measurement modes and parameters.
- **Examples Provided**: Includes examples for popular hardware platforms like ESP32 and STM32.

### Supported Operating Modes

- **Distance Measurement**: Configure the radar for precise distance measurements to objects.
- **Presence Detection**: Detect the presence of objects or people in a configured detection zone.

## Dependencies

_a121-rs_ builds on [a121-sys](https://crates.io/crates/a121-sys) 0.8, which binds Acconeer A121 SDK v1.13.0.
The static libraries you link against must be from the same SDK version.

The following dependencies are required to use _a121-rs_:
- Acconeer A121 Static Library (and detector libraries for distance and presence detection if the feature is enabled)
- `ACC_RSS_LIBS` set to the directory holding those libraries when building (see [a121-sys](https://crates.io/crates/a121-sys))
- A C cross-compiler for the target, which a121-sys uses to build its C wrapper (e.g. `arm-none-eabi-gcc` for Cortex-M)

## Supported Targets
Supported platforms depend on the availability of the Acconeer A121 Static Library.
Currently, the following targets are supported:
- arm-none-eabihf (gcc, armcc, armclang)
- esp xtensa and riscv

## Getting Started

To include _a121-rs_ in your project, add the following to your `Cargo.toml`:

```toml
[dependencies]
a121-rs = "0.3"
```

The static library expects implementations of math functions like `sqrt` and `sin` to be available.
If you are using a platform that does not provide these functions, you can enable the `libm` feature to use the `libm` crate for floating point operations


See the [documentation](https://docs.rs/a121-rs) for detailed usage instructions and examples.

## Feature flags
| feature        | description                                                                                                                  |
|----------------|------------------------------------------------------------------------------------------------------------------------------|
| distance       | Enable distance measurement module                                                                                           |
| presence       | Enable presence detection module                                                                                             |
| libm           | Use libm crate for floating point operations                                                                                 |
| logger         | Log SDK messages through defmt instead of the C logging bridge, avoiding newlib's `vsnprintf`. Needs a defmt global logger (e.g. `defmt-rtt`) in the final binary. Messages are formatted in Rust, about 5 KB of flash |
| nightly-logger | Deprecated alias for `logger`                                                                                                |
| defmt          | Log through defmt and derive `defmt::Format` on public types                                                                 |
| std            | Enable `std` in a121-sys                                                                                                     |
| heap-stats     | Count the memory the SDK allocates, read with `memory::sdk_heap_stats`, to size the heap                                     |
| stub_library   | Link against generated stub libraries instead of the Acconeer SDK, for host builds and tests without the real libraries      |

## Examples

The `examples/` directory holds one crate per board. Each runs the distance detector and logs the
distances found. Build from the example's directory with `ACC_RSS_LIBS` pointing at the Acconeer
libraries for that target:

```sh
cd examples/xe125-nightly
ACC_RSS_LIBS=/path/to/a121-v1.13.0/lib cargo build --release
cargo run --release   # flashes with probe-rs
```

| Example | Board | Target | Toolchain | a121-rs features | Status |
|---|---|---|---|---|---|
| `xe125-nightly` | XE125 (STM32L431CB) | `thumbv7em-none-eabihf` | nightly, `arm-none-eabi-gcc` and newlib | `distance, logger, heap-stats` | Links against SDK 1.13 |
| `xe121_l433rc` | XE121 on a NUCLEO-L433RC | `thumbv7em-none-eabihf` | stable, no C library | `distance, defmt, logger, libm, heap-stats` | Links against SDK 1.13 |
| `esp32c6-hal` | ESP32-C6 with an XE121 | `riscv32imac-unknown-none-elf` | stable, RISC-V gcc with newlib headers | `distance, logger, libm` | Compile-checked only |
| `esp32s3-hal` | ESP32-S3 | `xtensa-esp32s3-none-elf` | `esp` 1.99 or newer (espup) | `distance, logger, libm` | Compile-checked only |

The STM32 examples log the SDK heap use (`heap-stats`) after detector calibration and after the
first frame. Their 32 KB heap must hold that peak plus the detector buffers (`memory_requirements`),
and the allocator's overhead of a few bytes per allocation. CI checks the STM32 examples against the a121-sys stub libraries,
since the Acconeer libraries are not public.

- `xe125-nightly` links newlib with `arm-none-eabi-gcc` and needs nightly for `-Z pre-link-args`,
  which keeps newlib's smaller `sinf`/`cosf`/`tanf`. Its `.cargo/config.toml` sets `CPATH` for the
  newlib headers of Debian and Ubuntu's `libnewlib-arm-none-eabi`.
- `xe121_l433rc` links with `rust-lld` and no C library: the `logger` and `libm` features provide
  what the SDK would otherwise take from newlib.

The ESP32 examples use esp-hal 1.2 with esp-rtos and flash with `espflash` (`cargo run --release`).
No Acconeer libraries for ESP were available, so they have only been built against the stub
libraries (the ESP32-S3 one only compile-checked, see its README): linking against the real
libraries and running them on hardware are untested. Copy the libraries into the
example's `acc/` directory; its README lists them and the C toolchain each example needs.

## Development and Contribution

The tests run on the host against the a121-sys stub libraries:

```sh
cargo test --features stub_library,distance,presence
```

Contributions to _a121-rs_ are welcome! Whether it's adding new features, fixing bugs, or improving documentation, feel free to open issues and submit pull requests.

## License

_a121-rs_ is distributed under the MIT License. See [LICENSE](https://github.com/Ragarnoy/a121-rs/LICENSE) for more information.

---
