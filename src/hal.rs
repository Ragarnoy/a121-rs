use core::cell::RefCell;
use core::ffi::{c_char, c_void};

#[cfg(feature = "defmt")]
use core::ffi::CStr;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embedded_hal::spi::{ErrorKind as SpiErrorKind, SpiDevice};

use a121_sys::{acc_hal_a121_t, acc_hal_optimization_t, acc_rss_hal_register, acc_sensor_id_t};
#[cfg(feature = "defmt")]
use a121_sys::{
    acc_log_level_t, acc_log_level_t_ACC_LOG_LEVEL_DEBUG as LOG_DEBUG,
    acc_log_level_t_ACC_LOG_LEVEL_ERROR as LOG_ERROR,
    acc_log_level_t_ACC_LOG_LEVEL_INFO as LOG_INFO,
    acc_log_level_t_ACC_LOG_LEVEL_VERBOSE as LOG_VERBOSE,
    acc_log_level_t_ACC_LOG_LEVEL_WARNING as LOG_WARNING,
};

#[cfg(any(feature = "logger", test))]
mod printf;

pub type RadarSpi = dyn SpiDevice<u8, Error = SpiErrorKind> + Send;
pub type RefRadarSpi = &'static mut RadarSpi;

/// Global instance of a Mutex, wrapping a RefCell that optionally contains a mutable reference to a `SpiBus`.
///
/// `SPI_INSTANCE` is used to store and provide controlled access to the SPI device required by the radar sensor.
/// The `Mutex` ensures thread-safe access in environments where multi-threading is possible, while the `RefCell`
/// allows for mutable access to the SPI device. This setup is crucial for enabling SPI communications in a safe
/// and controlled manner within the radar sensor's hardware abstraction layer.
///
/// # Safety
///
/// The access to the `SPI_INSTANCE` is controlled via a mutex to prevent concurrent access issues.
/// However, care must be taken to ensure that the SPI device is properly initialized before use
/// and is not accessed after it has been freed or gone out of scope.
static SPI_INSTANCE: Mutex<CriticalSectionRawMutex, RefCell<Option<RefRadarSpi>>> =
    Mutex::new(RefCell::new(None));

/// Represents the hardware abstraction layer implementation for the radar sensor.
///
/// This struct encapsulates the necessary functionality to interface with the radar sensor
/// using the SPI communication protocol and provides methods for memory management and logging.
pub struct AccHalImpl {
    inner: acc_hal_a121_t,
}

impl AccHalImpl {
    /// Constructs a new `AccHalImpl` instance, registering the SPI device and initializing
    /// the radar hardware abstraction layer.
    ///
    /// # Arguments
    ///
    /// * `spi` - A reference to an SPI device that implements the `SpiBus` trait.
    pub fn new<SPI>(spi: &'static mut SPI) -> Self
    where
        SPI: SpiDevice<u8, Error = SpiErrorKind> + Send + 'static,
    {
        let inner = acc_hal_a121_t {
            max_spi_transfer_size: u16::MAX,
            mem_alloc: Some(mem_alloc),
            mem_free: Some(mem_free),
            transfer: Some(Self::transfer8_function),
            #[cfg(feature = "logger")]
            log: Some(logger),
            #[cfg(not(feature = "logger"))]
            log: Some(a121_sys::c_log_stub),
            optimization: acc_hal_optimization_t { transfer16: None },
        };
        SPI_INSTANCE.lock(|cell| cell.replace(Some(spi)));
        Self { inner }
    }

    /// Transfer function for 16-bit data used by the radar SDK.
    ///
    /// This function is registered as part of the HAL and is called by the radar SDK to
    /// perform SPI transfers.
    ///
    /// # Safety
    ///
    /// This function is unsafe as it involves raw pointers and direct hardware access.
    #[allow(dead_code)]
    extern "C" fn transfer16_function(
        _sensor_id: acc_sensor_id_t,
        _buffer: *mut u16,
        _buffer_length: usize,
    ) {
        #[cfg(feature = "defmt")]
        {
            let tmp_buf = unsafe { core::slice::from_raw_parts_mut(_buffer, _buffer_length) };
            defmt::trace!(
                "Transfer16 function called: buffer={:#X} (size:{})",
                tmp_buf,
                _buffer_length
            );
        }
        // Borrow a mutable reference to the SpiBus
        SPI_INSTANCE.lock(|cell| unsafe {
            let mut binding = cell.borrow_mut();
            let _spi = binding.as_mut().unwrap_unchecked();
            // Perform the SPI transfer
            todo!("Perform the SPI 16 transfer");
        });
    }

    extern "C" fn transfer8_function(
        _sensor_id: acc_sensor_id_t,
        buffer: *mut u8,
        buffer_length: usize,
    ) {
        let tmp_buf = unsafe { core::slice::from_raw_parts_mut(buffer, buffer_length) };
        // Borrow a mutable reference to the SpiBus
        SPI_INSTANCE.lock(|cell| {
            let mut binding = cell.borrow_mut();
            let Some(spi) = binding.as_mut() else {
                #[cfg(feature = "defmt")]
                defmt::error!("SPI transfer requested before an SPI device was registered");
                return;
            };
            // Perform the SPI transfer. The SDK callback cannot report errors, so log and continue.
            if let Err(_e) = spi.transfer_in_place(tmp_buf) {
                #[cfg(feature = "defmt")]
                defmt::error!("SPI transfer failed: {}", defmt::Debug2Format(&_e));
            }
        });
    }

    /// Registers the HAL implementation with the radar SDK.
    ///
    /// This method should be called to register the HAL implementation, allowing the
    /// radar sensor to communicate using the provided SPI interface.
    /// # Errors
    ///
    /// Returns `SensorError::InitFailed` when the HAL registration fails.
    #[inline(always)]
    pub fn register(&self) -> Result<(), crate::sensor::error::SensorError> {
        #[cfg(feature = "defmt")]
        defmt::trace!("Registering HAL");
        let result = unsafe { acc_rss_hal_register(&self.inner) };
        if result {
            Ok(())
        } else {
            Err(crate::sensor::error::SensorError::InitFailed)
        }
    }
}

use alloc::alloc::{alloc, dealloc, Layout};

/// Size and alignment of the header placed in front of each SDK allocation.
///
/// The SDK needs memory aligned for any built-in type: 8 bytes on 32-bit Arm, and 16 bytes on
/// RISC-V and 64-bit targets, where `long double` is 16 bytes.
const ALLOC_HEADER: usize = if cfg!(target_arch = "arm") { 8 } else { 16 };
const _: () = assert!(core::mem::size_of::<usize>() <= ALLOC_HEADER);

/// Allocates memory for use by the radar SDK.
///
/// Uses Rust's GlobalAlloc to avoid C malloc symbol conflicts.
/// The size is stored in an [`ALLOC_HEADER`]-byte header before the returned pointer for use in
/// deallocation. The header keeps the returned pointer aligned for any built-in type, as the SDK requires.
///
/// # Safety
///
/// This function is unsafe as it performs raw pointer manipulation.
unsafe extern "C" fn mem_alloc(size: usize) -> *mut c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }

    // Allocate extra space to store the size before the user data
    let total = match size.checked_add(ALLOC_HEADER) {
        Some(t) => t,
        None => return core::ptr::null_mut(),
    };

    let layout = match Layout::from_size_align(total, ALLOC_HEADER) {
        Ok(l) => l,
        Err(_) => return core::ptr::null_mut(),
    };

    let ptr = alloc(layout);
    if ptr.is_null() {
        return core::ptr::null_mut();
    }

    // Store the size at the beginning
    *(ptr as *mut usize) = size;

    // Return pointer after the size metadata
    ptr.add(ALLOC_HEADER) as *mut c_void
}

/// Frees memory previously allocated by mem_alloc.
///
/// Retrieves the size from before the pointer and deallocates the full block.
///
/// # Safety
///
/// This function is unsafe as it performs raw pointer manipulation.
/// The pointer must have been allocated by mem_alloc.
unsafe extern "C" fn mem_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }

    // Retrieve the size stored before the user data
    let base = (ptr as *mut u8).sub(ALLOC_HEADER);
    let size = *(base as *mut usize);

    let total = size + ALLOC_HEADER;
    let layout = Layout::from_size_align_unchecked(total, ALLOC_HEADER);

    dealloc(base, layout);
}

/// Emits an SDK log message at the matching defmt level.
///
/// The SDK levels are ordered by verbosity (DEBUG is the most verbose), so they are mapped by rank
/// rather than by name: VERBOSE goes to `debug` and DEBUG to `trace`.
#[cfg(feature = "defmt")]
fn log_message(level: acc_log_level_t, module: &str, message: &str) {
    match level {
        LOG_ERROR => defmt::error!("{=str}: {=str}", module, message),
        LOG_WARNING => defmt::warn!("{=str}: {=str}", module, message),
        LOG_INFO => defmt::info!("{=str}: {=str}", module, message),
        LOG_VERBOSE => defmt::debug!("{=str}: {=str}", module, message),
        LOG_DEBUG => defmt::trace!("{=str}: {=str}", module, message),
        _ => defmt::error!(
            "Unknown log level {}: {=str}: {=str}",
            level,
            module,
            message
        ),
    }
}

/// Converts a C string from the SDK to `&str`, treating NULL and invalid UTF-8 as empty.
///
/// # Safety
///
/// `ptr` must be NULL or point to a valid NUL-terminated string.
#[cfg(feature = "defmt")]
unsafe fn sdk_str<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or("")
}

/// SDK log callback used with the `logger` feature, replacing the C logging bridge.
///
/// Renders the format string and its arguments in Rust, so newlib's `vsnprintf` is not needed.
/// Messages longer than 256 bytes are cut and end with `…`. Floats are rendered with 7 significant
/// digits.
#[cfg(feature = "logger")]
unsafe extern "C" fn logger(
    level: acc_log_level_t,
    module: *const c_char,
    format: *const c_char,
    mut args: ...
) {
    let mut message = printf::LineBuf::<256>::new();
    if !format.is_null() {
        let format = unsafe { CStr::from_ptr(format) }.to_bytes();
        // Writing to a LineBuf never fails; overflow is truncated
        let _ = unsafe { printf::format(format, &mut args, &mut message) };
    }
    unsafe { log_message(level, sdk_str(module), message.as_str()) }
}

/// This function is called by the C logging bridge in a121-sys with an already formatted message.
///
/// It is defined even with the `logger` feature, so the bridge always links.
///
/// # Safety
/// This function is unsafe because it takes a raw pointer.
#[no_mangle]
pub unsafe extern "C" fn rust_log(_level: u32, _message: *const c_char) {
    #[cfg(feature = "defmt")]
    unsafe {
        log_message(_level, "", sdk_str(_message))
    }
}
