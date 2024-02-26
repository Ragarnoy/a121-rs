#![no_std]

extern crate alloc;

use talc::{ClaimOnOom, Span, Talc, Talck};

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

static mut ARENA: [u8; 10000] = [0; 10000];

#[global_allocator]
static ALLOCATOR: Talck<spin::Mutex<()>, ClaimOnOom> = Talc::new(unsafe {
    // if we're in a hosted environment, the Rust runtime may allocate before
    // main() is called, so we need to initialize the arena automatically
    ClaimOnOom::new(Span::from_const_array(core::ptr::addr_of!(ARENA)))
})
.lock();

extern "C" {
    #[allow(dead_code)]
    fn snprintf(buf: *mut i8, len: u32, fmt: *const i8, ...) -> i32;
}
