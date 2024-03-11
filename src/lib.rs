#![cfg_attr(feature = "nightly-logger", feature(c_variadic))]
#![cfg_attr(not(any(test, feature = "std")), no_std)]

//! The `a121-rs` library offers Rust bindings for interfacing with the Acconeer A121 radar sensor,
//! designed for embedded systems. Unlike traditional networking or sensor libraries, `a121-rs` focuses
//! on providing low-level access to the sensor's capabilities while abstracting the complexities
//! into a more Rust-centric API. This library is tailored for real-time applications where direct
//! control over sensor data acquisition and processing is crucial.
//!
//! `a121-rs` is structured to offer both high-level abstractions for easy integration into applications
//! and low-level access for fine-tuned control.
//!
//! # Sensor Modes and Configuration
//! The core functionality of `a121-rs` is exposed through its sensor mode configurations, allowing
//! users to switch between distance measurement and presence detection(_soon_). The configuration API,
//! found in the module [`config`](config/index.html), enables detailed customization of the
//! radar's parameters, such as sweep frequency, power levels, and processing options.
//!
//! # Data Acquisition and Processing
//! At the heart of `a121-rs` is the data acquisition and processing layer. This layer, accessible
//! through the [`radar`](radar/index.html) module, provides the mechanisms to initiate sensor
//! sweeps, retrieve raw data, and apply post-processing for noise reduction and signal
//! enhancement.
//!
//! # Hardware Abstraction Layer (HAL)
//! `a121-rs` employs an embedded-hal compatible layer, found in the [`hal`](hal/index.html) module,
//! to abstract over the specific hardware interfaces used to communicate with the A121 sensor.
//! This design allows `a121-rs` to be hardware agnostic, supporting a wide range of embedded platforms
//! by leveraging the embedded-hal ecosystem.
//! There is still work to be done to make the HAL more flexible and configurable, but the current
//! implementation is a good starting point for most use cases.
//!
//! # Examples and Use Cases
//! The library comes with a set of examples, located in the `examples` directory, demonstrating
//! common use cases and configurations for the A121 sensor. These examples cover basic setups for
//! different hardware platforms and provide a starting point for integrating the A121 sensor into
//! your projects.
//!

extern crate alloc;

/// Configuration for the radar sensor
pub mod config;
#[cfg(any(feature = "distance", feature = "presence"))]
/// Detector modules for the radar sensor
pub mod detector;
/// Hardware Abstraction Layer equivalent to the C API
pub mod hal;
#[cfg(feature = "libm")]
/// Math functions definitions from the libm crate
pub mod libm;
/// Number definitions for the radar sensor
pub mod num;
/// Processing modules for the radar sensor
pub mod processing;
/// Main radar module, interfacing with the radar sensor
pub mod radar;
/// C Bindings to the Acconeer Radar System Software
mod rss_bindings;
/// Sensor module for the radar sensor
mod sensor;
