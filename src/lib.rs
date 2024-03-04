#![feature(c_variadic)]
#![no_std]

extern crate alloc;

/// Configuration for the XM125
pub mod config;
#[cfg(any(feature = "distance", feature = "presence"))]
pub mod detector;
/// Hardware Abstraction Layer for the XM125
pub mod hal;
#[cfg(feature = "libm")]
pub mod libm;
pub mod num;
pub mod processing;
pub mod radar;
/// C Bindings to the XM125 SDK
mod rss_bindings;
/// Module to control the XM125 sensor
pub mod sensor;
