# a121-rs

[![docs.rs](https://docs.rs/a121-rs/badge.svg)](https://docs.rs/a121-rs)
[![crates.io](https://img.shields.io/crates/v/a121-rs.svg)](https://crates.io/crates/a121-rs)
[![crates.io](https://img.shields.io/crates/d/a121-rs.svg)](https://crates.io/crates/a121-rs)

_a121-rs_ is a Rust library providing bindings and a high-level abstraction for interfacing with the Acconeer A121 V-Band radar sensor. Designed for use in embedded systems, it offers asynchronous operation through embedded-hal traits, making it a perfect fit for no_std environments.

_a121-rs_ aims to simplify the development process for applications requiring accurate distance measurements and presence detection, leveraging the unique capabilities of the A121 radar sensor.

## Features

_a121-rs_ comes with a host of features designed to make working with the A121 sensor as straightforward as possible:

- **No Standard Library Dependencies**: Fully compatible with `no_std` for embedded systems use.
- **Embedded-HAL Async**: Utilizes asynchronous traits from `embedded-hal` for non-blocking sensor operation.
- **Configurable**: Provides a flexible API to configure the radar sensor for various measurement modes and parameters.
- **Examples Provided**: Includes examples for popular hardware platforms like ESP32 and STM32.

### Supported Operating Modes

- **Distance Measurement**: Configure the radar for precise distance measurements to objects.
- **Presence Detection**: Detect the presence of objects or people in a configured detection zone. (Soon)

## Dependencies

The following dependencies are required to use _a121-rs_:
- Acconeer A121 Static Library (and detector libraries for distance and presence detection if the feature is enabled)
- 'arm-none-eabi-gcc' to build the C wrapper (for stm32 examples)
```bash
# Ubuntu
sudo apt-get install gcc-arm-none-eabi
```
- 'riscv32-unknown-elf-gcc' to build the C wrapper for the esp32c6 (instructions tbd)

The esp example relies on the espup toolchain. Refer to the esp-rs project for configuration instructions.  

## Supported Targets
Supported platforms depend on the availability of the Acconeer A121 Static Library.
Currently, the following targets are supported:
- arm-none-eabihf (gcc, armcc, armclang)
- esp xtensa
- riscv32imac (esp32c6)

## Getting Started

To include _a121-rs_ in your project, add the following to your `Cargo.toml`:

```toml
[dependencies]
a121-rs = "0.1"
```

The static library expects implementations of math functions like `sqrt` and `sin` to be available.
If you are using a platform that does not provide these functions, you can enable the `libm` feature to use the `libm` crate for floating point operations


See the [documentation](https://docs.rs/a121-rs) for detailed usage instructions and examples.

## Feature flags
feature | description
--- | ---
distance | Enable distance measurement module
presence | Enable presence detection module (coming soon)
libm | Use libm crate for floating point operations
nightly-logger | If the C wrapper for logging does not compile with stable rust, enable this feature to use nightly rust with a custom logger

## Examples

Check out the `examples/` directory for demonstrations on how to use _a121-rs_ with various microcontroller units.
These examples cover basic setups and common use cases to help you get started quickly.

## Development and Contribution

Contributions to _a121-rs_ are welcome! Whether it's adding new features, fixing bugs, or improving documentation, feel free to open issues and submit pull requests.

## License

_a121-rs_ is distributed under the MIT License. See [LICENSE](https://github.com/Ragarnoy/a121-rs/LICENSE) for more information.

---
