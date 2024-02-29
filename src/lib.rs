#![no_std]

/// Configuration for the XM125
pub mod config;
mod detector;
/// Hardware Abstraction Layer for the XM125
pub mod hal;
pub mod num;
pub mod processing;
pub mod radar;
/// C Bindings to the XM125 SDK
mod rss_bindings;
/// Module to control the XM125 sensor
pub mod sensor;
