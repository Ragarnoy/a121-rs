#![no_std]

extern crate alloc;

/// Configuration for the XM125
pub mod config;
pub mod detector;
/// Hardware Abstraction Layer for the XM125
pub mod hal;
pub mod num;
pub mod processing;
pub mod radar;
/// C Bindings to the XM125 SDK
mod rss_bindings;
/// Module to control the XM125 sensor
pub mod sensor;

extern "C" {
    #[allow(dead_code)]
    fn snprintf(buf: *mut i8, len: u32, fmt: *const i8, ...) -> i32;
}
