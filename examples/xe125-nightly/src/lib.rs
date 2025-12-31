#![no_std]

extern crate alloc;

use core::cell::RefCell;
use core::ffi::c_void;
use core::ptr::addr_of_mut;
use embassy_stm32::gpio::Output;
use embassy_stm32::mode::Async;
use embassy_stm32::rcc::{
    LsConfig, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv, PllRDiv, PllSource, Sysclk,
};
use embassy_stm32::spi::{Config, Spi};
use embassy_stm32::time::Hertz;
use embassy_time::Delay;
use embedded_hal_bus::spi::ExclusiveDevice;
use talc::{ClaimOnOom, Span, Talc, Talck};

use crate::adapter::SpiAdapter;

pub mod adapter;

// Heap for dynamic allocation - increased size for C library usage
const HEAP_SIZE: usize = 32 * 1024; // 32KB heap (more space for C library)
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[global_allocator]
pub static ALLOCATOR: Talck<spin::Mutex<()>, ClaimOnOom> = Talc::new(unsafe {
    // Initialize with the heap memory using proper slice conversion
    ClaimOnOom::new(Span::from_array(addr_of_mut!(HEAP)))
})
.lock();

pub type SpiDeviceMutex = ExclusiveDevice<Spi<'static, Async>, Output<'static>, Delay>;
pub static mut SPI_DEVICE: Option<RefCell<SpiAdapter<SpiDeviceMutex>>> = None;

pub fn xm125_spi_config() -> Config {
    let mut spi_config = Config::default();
    spi_config.frequency = Hertz(1_000_000);
    spi_config
}

pub fn xm125_clock_config() -> embassy_stm32::Config {
    let mut config = embassy_stm32::Config::default();
    config.rcc.hsi = true;
    config.rcc.hse = None;
    config.rcc.msi = None;
    config.rcc.sys = Sysclk::PLL1_R;
    config.rcc.pll = Some(Pll {
        source: PllSource::HSI,
        prediv: PllPreDiv::DIV1,
        mul: PllMul::MUL10,
        divp: Some(PllPDiv::DIV7),
        divq: Some(PllQDiv::DIV2),
        divr: Some(PllRDiv::DIV2),
    });
    config.rcc.pllsai1 = Some(Pll {
        source: PllSource::HSI,
        prediv: PllPreDiv::DIV1,
        mul: PllMul::MUL8,
        divp: Some(PllPDiv::DIV7),
        divq: Some(PllQDiv::DIV2),
        divr: Some(PllRDiv::DIV2),
    });
    config.rcc.ls = LsConfig::default_lsi();
    config
}

// C allocator shims - required for C SDK code that calls malloc directly.
// Note: a121-rs itself uses Rust's GlobalAlloc (above), but the underlying
// C SDK may still call malloc/free/calloc/realloc in its internal code.

#[no_mangle]
extern "C" fn malloc(size: usize) -> *mut c_void {
    use alloc::alloc::GlobalAlloc;

    if size == 0 {
        return core::ptr::null_mut();
    }

    // Allocate extra space to store the size before the user data
    let total_size = match size.checked_add(size_of::<usize>()) {
        Some(v) => v,
        None => return core::ptr::null_mut(),
    };
    let layout = core::alloc::Layout::from_size_align(total_size, 8)
        .unwrap_or_else(|_| core::panic!("Invalid malloc size: {}", size));

    unsafe {
        let ptr = ALLOCATOR.alloc(layout);
        if ptr.is_null() {
            return core::ptr::null_mut();
        }

        // Store the user-requested size at the beginning
        *(ptr as *mut usize) = size;

        // Return pointer to user data (after the size)
        (ptr as *mut usize).offset(1) as *mut c_void
    }
}

#[no_mangle]
extern "C" fn free(ptr: *mut c_void) {
    use alloc::alloc::GlobalAlloc;

    if ptr.is_null() {
        return;
    }

    // We need to store the size somehow. A common approach is to store
    // the size just before the allocated block
    unsafe {
        let size_ptr = (ptr as *mut usize).offset(-1);
        let size = *size_ptr;

        // Create the layout that was used for allocation
        let total_size = size
            .checked_add(size_of::<usize>())
            .expect("Size overflow in free()");
        let layout =
            core::alloc::Layout::from_size_align(total_size, 8).expect("Invalid layout in free()");

        // Deallocate using talc
        ALLOCATOR.dealloc(size_ptr as *mut u8, layout);
    }
}

// Rust
#[no_mangle]
extern "C" fn calloc(count: usize, size: usize) -> *mut c_void {
    // Detect overflow
    let total_size = match count.checked_mul(size) {
        Some(n) => n,
        None => {
            return core::ptr::null_mut();
        }
    };

    if total_size == 0 {
        return core::ptr::null_mut();
    }

    let ptr = malloc(total_size);
    if !ptr.is_null() {
        unsafe {
            core::ptr::write_bytes(ptr as *mut u8, 0, total_size);
        }
    }
    ptr
}

#[no_mangle]
extern "C" fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    if ptr.is_null() {
        return malloc(size);
    }

    if size == 0 {
        free(ptr);
        return core::ptr::null_mut();
    }

    // Get the old size from the stored metadata
    let old_size = unsafe {
        let size_ptr = (ptr as *mut usize).offset(-1);
        *size_ptr
    };

    // Allocate new memory
    let new_ptr = malloc(size);
    if new_ptr.is_null() {
        return core::ptr::null_mut();
    }

    // Copy old data to new memory (up to the smaller of old/new size)
    let copy_size = core::cmp::min(old_size, size);
    unsafe {
        core::ptr::copy_nonoverlapping(ptr as *const u8, new_ptr as *mut u8, copy_size);
    }

    // Free the old memory
    free(ptr);

    new_ptr
}

// Stub _sbrk that signals no traditional heap available
#[no_mangle]
extern "C" fn _sbrk(_incr: isize) -> *mut c_void {
    // Return error to force newlib to use our malloc instead
    (-1isize) as *mut c_void
}
