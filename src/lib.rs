#![cfg_attr(feature = "nightly-logger", feature(c_variadic))]
#![no_std]

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
