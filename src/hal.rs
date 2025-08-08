use core::cell::RefCell;
use core::ffi::{c_char, c_void};

#[cfg(feature = "defmt")]
use core::ffi::CStr;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embedded_hal::spi::{ErrorKind as SpiErrorKind, SpiDevice};

use a121_sys::{acc_hal_a121_t, acc_hal_optimization_t, acc_rss_hal_register, acc_sensor_id_t};

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
            #[cfg(feature = "nightly-logger")]
            log: Some(logger),
            #[cfg(not(feature = "nightly-logger"))]
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
        SPI_INSTANCE.lock(|cell| unsafe {
            let mut binding = cell.borrow_mut();
            let spi = binding.as_mut().unwrap_unchecked();
            // Perform the SPI transfer
            spi.transfer_in_place(tmp_buf).unwrap_unchecked();
        });
    }

    /// Registers the HAL implementation with the radar SDK.
    ///
    /// This method should be called to register the HAL implementation, allowing the
    /// radar sensor to communicate using the provided SPI interface.
    ///
    /// # Panics
    ///
    /// Panics if the HAL registration fails.
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

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

/// Allocates memory for use by the radar SDK.
///
/// # Safety
///
/// This function is unsafe as it performs raw pointer manipulation.
unsafe extern "C" fn mem_alloc(size: usize) -> *mut c_void {
    malloc(size)
}

/// Frees memory previously allocated for the radar SDK.
///
/// # Safety
///
/// This function is unsafe as it performs raw pointer manipulation.
unsafe extern "C" fn mem_free(ptr: *mut c_void) {
    free(ptr);
}

#[cfg(feature = "nightly-logger")]
unsafe extern "C" fn logger(
    level: a121_sys::acc_log_level_t,
    module: *const c_char,
    format: *const c_char,
    mut _va: ...
) {
    let module = unsafe { CStr::from_ptr(module) };
    let format = unsafe { CStr::from_ptr(format) };
    let message = format.to_str().unwrap_or("");

    match level {
        0 => defmt::error!("{}: {}", module.to_str().unwrap_or(""), message),
        1 => defmt::warn!("{}: {}", module.to_str().unwrap_or(""), message),
        2 => defmt::info!("{}: {}", module.to_str().unwrap_or(""), message),
        3 => defmt::debug!("{}: {}", module.to_str().unwrap_or(""), message),
        4 => defmt::trace!("{}: {}", module.to_str().unwrap_or(""), message),
        _ => defmt::error!("Unknown log level: {}", level),
    }
}

#[cfg(not(feature = "nightly-logger"))]
/// This function is called by the C stub to log messages from the SDK.
/// # Safety
/// This function is unsafe because it takes a raw pointer.
#[no_mangle]
pub unsafe extern "C" fn rust_log(_level: u32, _message: *const c_char) {
    #[cfg(feature = "defmt")]
    {
        let c_str = unsafe { CStr::from_ptr(_message) };
        let str_slice = c_str.to_str().unwrap_or("");
        match _level {
            0 => defmt::error!("{}", str_slice),
            1 => defmt::warn!("{}", str_slice),
            2 => defmt::info!("{}", str_slice),
            3 => defmt::debug!("{}", str_slice),
            4 => defmt::trace!("{}", str_slice),
            _ => defmt::error!("Unknown log level: {}", _level),
        }
    }
}
