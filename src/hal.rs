use core::cell::{Cell, RefCell};
use core::ffi::{c_char, c_void};
use core::ptr::NonNull;

#[cfg(feature = "defmt")]
use core::ffi::CStr;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embedded_hal::spi::SpiDevice;

use a121_sys::{acc_hal_a121_t, acc_hal_optimization_t, acc_rss_hal_register, acc_sensor_id_t};
#[cfg(feature = "defmt")]
use a121_sys::{
    acc_log_level_t, acc_log_level_t_ACC_LOG_LEVEL_DEBUG as LOG_DEBUG,
    acc_log_level_t_ACC_LOG_LEVEL_ERROR as LOG_ERROR,
    acc_log_level_t_ACC_LOG_LEVEL_INFO as LOG_INFO,
    acc_log_level_t_ACC_LOG_LEVEL_VERBOSE as LOG_VERBOSE,
    acc_log_level_t_ACC_LOG_LEVEL_WARNING as LOG_WARNING,
};

use crate::sensor::error::SensorError;

#[cfg(any(feature = "logger", test))]
mod printf;

/// The most sensors that can be in use at once, each with its own SPI device.
///
/// The SDK does not document whether it is reentrant, so SDK calls on different radars must not
/// run concurrently, for example from executors at different interrupt priorities.
pub const MAX_SENSORS: usize = 4;

/// Transfers a buffer in place on the SPI device behind the pointer, returning `false` on failure.
type TransferFn = unsafe fn(NonNull<()>, &mut [u8]) -> bool;

/// The SPI device of one sensor, with whether a transfer failed since the flag was last read.
///
/// The device is type-erased: `transfer` was instantiated for the type `spi` points to.
#[derive(Clone, Copy)]
struct Slot {
    sensor_id: u32,
    spi: NonNull<()>,
    transfer: TransferFn,
    failed: bool,
}

// SAFETY: `spi` comes from a `&'static mut` to a `Send` device (see `register_spi`), and is only
// dereferenced by the transfer callback during an SDK call on its sensor.
unsafe impl Send for Slot {}

/// The sensors in use, keyed by sensor id.
struct SlotTable<const N: usize> {
    slots: [Option<Slot>; N],
}

impl<const N: usize> SlotTable<N> {
    const fn new() -> Self {
        Self { slots: [None; N] }
    }

    fn find(&mut self, sensor_id: u32) -> Option<&mut Slot> {
        self.slots
            .iter_mut()
            .flatten()
            .find(|slot| slot.sensor_id == sensor_id)
    }

    /// Fails with [`SensorError::SensorUnavailable`] if the sensor is already in use or every
    /// slot is taken.
    fn claim(
        &mut self,
        sensor_id: u32,
        spi: NonNull<()>,
        transfer: TransferFn,
    ) -> Result<(), SensorError> {
        if self.find(sensor_id).is_some() {
            return Err(SensorError::SensorUnavailable);
        }
        let free = self
            .slots
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or(SensorError::SensorUnavailable)?;
        *free = Some(Slot {
            sensor_id,
            spi,
            transfer,
            failed: false,
        });
        Ok(())
    }

    fn release(&mut self, sensor_id: u32) {
        for slot in &mut self.slots {
            if slot.is_some_and(|slot| slot.sensor_id == sensor_id) {
                *slot = None;
            }
        }
    }

    fn device(&mut self, sensor_id: u32) -> Option<(NonNull<()>, TransferFn)> {
        self.find(sensor_id).map(|slot| (slot.spi, slot.transfer))
    }

    fn record_failure(&mut self, sensor_id: u32) {
        if let Some(slot) = self.find(sensor_id) {
            slot.failed = true;
        }
    }

    /// Returns and clears whether a transfer failed. A sensor without an SPI device cannot
    /// transfer, so it always reports a failure.
    fn take_failure(&mut self, sensor_id: u32) -> bool {
        self.find(sensor_id)
            .is_none_or(|slot| core::mem::take(&mut slot.failed))
    }
}

static SLOTS: Mutex<CriticalSectionRawMutex, RefCell<SlotTable<MAX_SENSORS>>> =
    Mutex::new(RefCell::new(SlotTable::new()));

/// Keeps a sensor's SPI device registered until dropped.
pub(crate) struct SpiRegistration {
    sensor_id: u32,
}

impl Drop for SpiRegistration {
    fn drop(&mut self) {
        SLOTS.lock(|slots| slots.borrow_mut().release(self.sensor_id));
    }
}

/// Registers `spi` as the SPI device of `sensor_id` until the returned guard is dropped.
///
/// # Safety
///
/// `spi` must come from a `&'static mut SPI`, and nothing else may access the device until the
/// guard is dropped.
pub(crate) unsafe fn register_spi<SPI>(
    sensor_id: u32,
    spi: NonNull<SPI>,
) -> Result<SpiRegistration, SensorError>
where
    SPI: SpiDevice<u8> + Send + 'static,
{
    SLOTS.lock(|slots| {
        slots
            .borrow_mut()
            .claim(sensor_id, spi.cast(), transfer_with::<SPI>)
    })?;
    Ok(SpiRegistration { sensor_id })
}

/// # Safety
///
/// `spi` must point to an `SPI` registered with [`register_spi`], with no other access to it.
unsafe fn transfer_with<SPI: SpiDevice<u8>>(spi: NonNull<()>, buffer: &mut [u8]) -> bool {
    match unsafe { spi.cast::<SPI>().as_mut() }.transfer_in_place(buffer) {
        Ok(()) => true,
        Err(_e) => {
            #[cfg(feature = "defmt")]
            defmt::error!("SPI transfer failed: {}", defmt::Debug2Format(&_e));
            false
        }
    }
}

/// SPI transfer callback for the SDK. It cannot return an error, so a failure is recorded for
/// [`spi_checked`] to report.
extern "C" fn transfer8_function(
    sensor_id: acc_sensor_id_t,
    buffer: *mut u8,
    buffer_length: usize,
) {
    let buffer = unsafe { core::slice::from_raw_parts_mut(buffer, buffer_length) };
    let Some((spi, transfer)) = SLOTS.lock(|slots| slots.borrow_mut().device(sensor_id)) else {
        #[cfg(feature = "defmt")]
        defmt::error!(
            "SPI transfer requested for sensor {} without an SPI device",
            sensor_id
        );
        return;
    };
    // The transfer runs outside the critical section, so interrupts stay enabled. Only an SDK
    // call on this sensor transfers on its device, and that call holds the sensor's radar, which
    // is not `Sync`; the slot is released only when the radar is dropped.
    if !unsafe { transfer(spi, buffer) } {
        SLOTS.lock(|slots| slots.borrow_mut().record_failure(sensor_id));
    }
}

/// Runs an SDK call that talks to the sensor, and fails if any SPI transfer to it failed
/// during the call.
///
/// The SDK can report success after a failed transfer, since it never learns about it.
pub(crate) fn spi_checked<T>(sensor_id: u32, f: impl FnOnce() -> T) -> Result<T, SensorError> {
    match with_spi_status(sensor_id, f) {
        (result, false) => Ok(result),
        (_, true) => Err(SensorError::SpiTransferFailed),
    }
}

/// Runs an SDK call and returns its result with whether an SPI transfer to the sensor failed
/// during it, for calls whose result must be cleaned up either way.
pub(crate) fn with_spi_status<T>(sensor_id: u32, f: impl FnOnce() -> T) -> (T, bool) {
    SLOTS.lock(|slots| slots.borrow_mut().take_failure(sensor_id));
    let result = f();
    let failed = SLOTS.lock(|slots| slots.borrow_mut().take_failure(sensor_id));
    (result, failed)
}

/// The HAL handed to the SDK. It is static because the SDK may keep the pointer it is given.
static HAL: acc_hal_a121_t = acc_hal_a121_t {
    max_spi_transfer_size: u16::MAX,
    mem_alloc: Some(mem_alloc),
    mem_free: Some(mem_free),
    transfer: Some(transfer8_function),
    #[cfg(feature = "logger")]
    log: Some(logger),
    #[cfg(not(feature = "logger"))]
    log: Some(a121_sys::c_log_stub),
    optimization: acc_hal_optimization_t { transfer16: None },
};

static HAL_REGISTERED: Mutex<CriticalSectionRawMutex, Cell<bool>> = Mutex::new(Cell::new(false));

/// Registers the HAL with the SDK, once.
///
/// # Errors
///
/// Returns [`SensorError::InitFailed`] when the SDK rejects the HAL; the next call tries again.
pub(crate) fn ensure_registered() -> Result<(), SensorError> {
    if HAL_REGISTERED.lock(Cell::get) {
        return Ok(());
    }
    #[cfg(feature = "defmt")]
    defmt::trace!("Registering HAL");
    if unsafe { acc_rss_hal_register(&HAL) } {
        HAL_REGISTERED.lock(|registered| registered.set(true));
        Ok(())
    } else {
        Err(SensorError::InitFailed)
    }
}

use alloc::alloc::{alloc, dealloc, Layout};

/// Alignment of any built-in type, which the SDK requires for memory and buffers it is given: 8
/// bytes on 32-bit Arm, and 16 bytes on RISC-V and 64-bit targets, where `long double` is 16 bytes.
pub(crate) const SDK_ALIGN: usize = if cfg!(target_arch = "arm") { 8 } else { 16 };

/// Size and alignment of the header placed in front of each SDK allocation.
const ALLOC_HEADER: usize = SDK_ALIGN;
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use embedded_hal::spi::{ErrorKind, ErrorType, Operation};

    #[derive(Debug)]
    pub(crate) struct MockError;

    impl embedded_hal::spi::Error for MockError {
        fn kind(&self) -> ErrorKind {
            ErrorKind::Other
        }
    }

    /// Inverts the bytes it transfers, or fails every transfer.
    pub(crate) struct MockSpi {
        pub(crate) fail: bool,
    }

    impl ErrorType for MockSpi {
        type Error = MockError;
    }

    impl SpiDevice<u8> for MockSpi {
        fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), MockError> {
            if self.fail {
                return Err(MockError);
            }
            for operation in operations {
                if let Operation::TransferInPlace(words) = operation {
                    words.iter_mut().for_each(|word| *word = !*word);
                }
            }
            Ok(())
        }
    }

    fn leak_spi(fail: bool) -> NonNull<MockSpi> {
        NonNull::from(alloc::boxed::Box::leak(alloc::boxed::Box::new(MockSpi {
            fail,
        })))
    }

    unsafe fn no_transfer(_: NonNull<()>, _: &mut [u8]) -> bool {
        true
    }

    #[test]
    fn slot_table_claims_and_releases() {
        let mut table = SlotTable::<2>::new();
        let spi = NonNull::<u8>::dangling().cast();
        assert_eq!(table.claim(1, spi, no_transfer), Ok(()));
        assert_eq!(
            table.claim(1, spi, no_transfer),
            Err(SensorError::SensorUnavailable)
        );
        assert_eq!(table.claim(2, spi, no_transfer), Ok(()));
        assert_eq!(
            table.claim(3, spi, no_transfer),
            Err(SensorError::SensorUnavailable)
        );
        table.release(1);
        assert!(table.device(1).is_none());
        assert_eq!(table.claim(3, spi, no_transfer), Ok(()));
        assert!(table.device(2).is_some());
    }

    #[test]
    fn slot_table_tracks_failures_per_sensor() {
        let mut table = SlotTable::<2>::new();
        let spi = NonNull::<u8>::dangling().cast();
        table.claim(1, spi, no_transfer).unwrap();
        table.claim(2, spi, no_transfer).unwrap();
        table.record_failure(1);
        assert!(!table.take_failure(2));
        assert!(table.take_failure(1));
        assert!(!table.take_failure(1));
        // No device, no transfer
        assert!(table.take_failure(3));
    }

    // The tests below use the static table, with sensor ids no other test uses. Tests run in
    // parallel, so all tests together may hold at most MAX_SENSORS slots at once: 1001, 1002,
    // 1003 here and 2001 in radar.rs use all 4.

    #[test]
    fn transfers_reach_the_sensors_device() {
        let registration = unsafe { register_spi(1001, leak_spi(false)) }.unwrap();
        let mut buffer = [0x0f, 0xf0];
        let result = spi_checked(1001, || {
            transfer8_function(1001, buffer.as_mut_ptr(), buffer.len());
            7
        });
        assert_eq!(result, Ok(7));
        assert_eq!(buffer, [0xf0, 0x0f]);
        assert_eq!(
            unsafe { register_spi(1001, leak_spi(false)) }.err(),
            Some(SensorError::SensorUnavailable)
        );
        drop(registration);
        assert!(unsafe { register_spi(1001, leak_spi(false)) }.is_ok());
    }

    #[test]
    fn spi_failures_are_reported_for_their_sensor_only() {
        let _failing = unsafe { register_spi(1002, leak_spi(true)) }.unwrap();
        let _working = unsafe { register_spi(1003, leak_spi(false)) }.unwrap();
        let mut buffer = [0u8; 4];
        let mut transfer = |sensor_id| transfer8_function(sensor_id, buffer.as_mut_ptr(), 4);
        assert_eq!(
            spi_checked(1002, || transfer(1002)),
            Err(SensorError::SpiTransferFailed)
        );
        assert_eq!(spi_checked(1003, || transfer(1002)), Ok(()));
        // The flag does not leak into the next call
        assert_eq!(spi_checked(1002, || 2), Ok(2));
        assert_eq!(spi_checked(1003, || transfer(1003)), Ok(()));
    }
}
