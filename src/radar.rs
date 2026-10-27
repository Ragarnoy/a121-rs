pub mod version;

use a121_sys::{acc_sensor_connected, acc_sensor_id_t, acc_sensor_t};
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::SpiDevice;
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::digital::Wait;

use alloc::vec;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use crate::config::RadarConfig;
pub use crate::hal::MAX_SENSORS;
use crate::hal::{ensure_registered, register_spi, spi_checked, SpiRegistration, SDK_ALIGN};
use crate::processing::{Processing, ProcessingResult};
use crate::sensor::calibration::CalibrationResult;
use crate::sensor::error::SensorError;
use crate::sensor::{InnerSensor, Sensor};

/// Default scratch buffer size for radar operations (calibration and sensor preparation).
///
/// The SDK needs at least [`RadarConfig::config_buffer_size`] bytes, which depends on the
/// configuration. [`Radar::prepare_sensor`] checks this at runtime and returns
/// [`SensorError::BufferTooSmall`] when the scratch buffer is too small; use a larger
/// `SCRATCH_SIZE` in that case.
pub const DEFAULT_SCRATCH_BUFFER_SIZE: usize = 5_560;

/// Byte buffer aligned for any built-in type, as the SDK requires for the buffers it is given.
///
/// Use it for the buffers passed to [`Radar::measure`] and to the detectors.
#[derive(Clone)]
#[cfg_attr(target_arch = "arm", repr(C, align(8)))]
#[cfg_attr(not(target_arch = "arm"), repr(C, align(16)))]
pub struct RadarBuffer<const N: usize>([u8; N]);

impl<const N: usize> RadarBuffer<N> {
    /// Creates a zeroed buffer.
    pub const fn new() -> Self {
        Self([0; N])
    }
}

impl<const N: usize> Default for RadarBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Deref for RadarBuffer<N> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl<const N: usize> DerefMut for RadarBuffer<N> {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

/// Heap-allocated byte buffer aligned like [`RadarBuffer`], for sizes known only at runtime such as
/// the detectors' buffer sizes.
#[derive(Clone)]
pub struct AlignedBuffer {
    chunks: Vec<RadarBuffer<16>>,
    len: usize,
}

impl AlignedBuffer {
    /// Allocates a zeroed buffer of `len` bytes.
    pub fn new(len: usize) -> Self {
        Self {
            chunks: vec![RadarBuffer::new(); len.div_ceil(16)],
            len,
        }
    }
}

impl Deref for AlignedBuffer {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        // SAFETY: the chunks are contiguous plain bytes, at least `len` of them
        unsafe { core::slice::from_raw_parts(self.chunks.as_ptr().cast::<u8>(), self.len) }
    }
}

impl DerefMut for AlignedBuffer {
    fn deref_mut(&mut self) -> &mut [u8] {
        // SAFETY: as in `deref`, and `&mut self` gives exclusive access
        unsafe { core::slice::from_raw_parts_mut(self.chunks.as_mut_ptr().cast::<u8>(), self.len) }
    }
}

// RadarBuffer has exactly the alignment the SDK asks for on every target
const _: () = assert!(core::mem::align_of::<RadarBuffer<0>>() == SDK_ALIGN);

/// Checks that a buffer handed to the SDK holds at least `min_len` bytes and is aligned for any
/// built-in type.
pub(crate) fn check_buffer(buffer: &[u8], min_len: usize) -> Result<(), SensorError> {
    check_layout(buffer.len(), buffer.as_ptr() as usize, min_len)
}

fn check_layout(len: usize, addr: usize, min_len: usize) -> Result<(), SensorError> {
    if len < min_len {
        Err(SensorError::BufferTooSmall)
    } else if !addr.is_multiple_of(SDK_ALIGN) {
        Err(SensorError::BufferMisaligned)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RadarState {
    /// Powered on, but not prepared for the current configuration.
    Enabled,
    /// Prepared and ready to measure.
    Ready,
    /// Hibernating with the enable pin low.
    Hibernating,
    /// The sensor's state is unknown: an operation failed in a way that can leave it busy or
    /// half-configured, such as an SPI transfer failure or a missed interrupt while measuring, or
    /// an operation's future was dropped before it completed. [`Radar::reset_sensor`] and
    /// [`Radar::calibrate`] power-cycle the sensor and recover it; every other operation that
    /// drives the sensor is refused. The diagnostics [`Radar::is_connected`] and
    /// [`Radar::check_status`] stay available.
    Faulted,
}

/// What the sensor was last prepared for.
enum Prepared {
    None,
    /// [`Radar::prepare_sensor`], with the processing for the prepared configuration.
    Radar(Processing),
    /// A detector, which measures into buffers of `buffer_size` bytes.
    #[cfg(any(feature = "distance", feature = "presence"))]
    Detector {
        buffer_size: usize,
    },
}

/// Radar sensor controller with configurable scratch buffer size.
///
/// The `SCRATCH_SIZE` const generic sets the size of the internal scratch buffer used for
/// calibration and preparation. [`DEFAULT_SCRATCH_BUFFER_SIZE`] fits most configurations.
///
/// Up to [`MAX_SENSORS`] radars can exist at once, each with its own sensor id and SPI device.
///
/// # Example
///
/// ```ignore
/// // Use default buffer size
/// let radar: Radar<_, _, _, _> = Radar::new(id, spi, interrupt, enable, delay).await?;
///
/// // Use custom buffer size for larger configurations
/// let radar: Radar<_, _, _, _, 8192> = Radar::new(id, spi, interrupt, enable, delay).await?;
/// ```
pub struct Radar<SPI, SINT, ENABLE, DLY, const SCRATCH_SIZE: usize = DEFAULT_SCRATCH_BUFFER_SIZE>
where
    SPI: SpiDevice<u8> + 'static,
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    pub(crate) config: RadarConfig,
    sensor: Sensor<ENABLE, DLY>,
    pub(crate) interrupt: SINT,
    state: RadarState,
    /// Kept while hibernating, so waking up knows whether the sensor is still prepared
    prepared: Prepared,
    scratch: RadarBuffer<SCRATCH_SIZE>,
    /// Releases the sensor's SPI device. Declared after `sensor`, so the sensor is destroyed
    /// first.
    registration: SpiRegistration,
    /// The SPI device, used through the HAL until [`Radar::free`] hands it back.
    spi: NonNull<SPI>,
    /// Keeps the radar neither `Send` nor `Sync`: the HAL transfers on the SPI device outside a
    /// critical section, which is only sound while the device is used from a single thread.
    _single_thread: PhantomData<*mut ()>,
}

/// A failed [`Radar::new`], with the resources it was given, so it can be retried.
pub struct RadarInitError<SPI: 'static, SINT, ENABLE, DLY> {
    /// Why the radar could not be created.
    pub error: SensorError,
    /// The SPI device.
    pub spi: &'static mut SPI,
    /// The sensor interrupt pin.
    pub interrupt: SINT,
    /// The sensor enable pin.
    pub enable_pin: ENABLE,
    /// The delay provider.
    pub delay: DLY,
}

impl<SPI, SINT, ENABLE, DLY> From<RadarInitError<SPI, SINT, ENABLE, DLY>> for SensorError {
    fn from(error: RadarInitError<SPI, SINT, ENABLE, DLY>) -> Self {
        error.error
    }
}

impl<SPI, SINT, ENABLE, DLY> core::fmt::Debug for RadarInitError<SPI, SINT, ENABLE, DLY> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RadarInitError")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl<SPI, SINT, ENABLE, DLY> core::fmt::Display for RadarInitError<SPI, SINT, ENABLE, DLY> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "radar creation failed: {}", self.error)
    }
}

#[cfg(feature = "defmt")]
impl<SPI, SINT, ENABLE, DLY> defmt::Format for RadarInitError<SPI, SINT, ENABLE, DLY> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "RadarInitError({})", self.error)
    }
}

impl<SPI, SINT, ENABLE, DLY, const SCRATCH_SIZE: usize> Radar<SPI, SINT, ENABLE, DLY, SCRATCH_SIZE>
where
    SPI: SpiDevice<u8> + 'static,
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    /// Powers the sensor up and creates the radar for sensor `id`, which talks to the sensor
    /// through `spi`.
    ///
    /// # Errors
    ///
    /// Returns the resources with the error, so creation can be retried:
    /// [`SensorError::SensorUnavailable`] if a radar for `id` exists or [`MAX_SENSORS`] radars
    /// exist, [`SensorError::EnablePinFailed`], [`SensorError::InitFailed`] if the SDK could not
    /// register the HAL or create the configuration or sensor, or
    /// [`SensorError::SpiTransferFailed`]. `RadarInitError` converts to [`SensorError`] with `?`.
    ///
    /// Dropping the future before it completes, for example on a timeout, drops the SPI device
    /// with it.
    pub async fn new(
        id: u32,
        spi: &'static mut SPI,
        interrupt: SINT,
        mut enable_pin: ENABLE,
        mut delay: DLY,
    ) -> Result<Self, RadarInitError<SPI, SINT, ENABLE, DLY>> {
        // Only raw pointers to the device are kept from here on, so handing the reference back
        // does not invalidate them
        let spi = NonNull::from(spi);
        match Self::create(id, spi, &mut enable_pin, &mut delay).await {
            Ok((registration, config, inner)) => Ok(Radar {
                config,
                sensor: Sensor::new(inner, enable_pin, delay),
                interrupt,
                state: RadarState::Enabled,
                prepared: Prepared::None,
                scratch: RadarBuffer::new(),
                registration,
                spi,
                _single_thread: PhantomData,
            }),
            Err(error) => Err(RadarInitError {
                error,
                // SAFETY: the device's registration was dropped, so this is its only user again
                spi: unsafe { &mut *spi.as_ptr() },
                interrupt,
                enable_pin,
                delay,
            }),
        }
    }

    async fn create(
        id: u32,
        spi: NonNull<SPI>,
        enable_pin: &mut ENABLE,
        delay: &mut DLY,
    ) -> Result<(SpiRegistration, RadarConfig, InnerSensor), SensorError> {
        ensure_registered()?;
        // SAFETY: `spi` comes from a `&'static mut SPI` that only the HAL uses until the
        // registration is dropped
        let registration = unsafe { register_spi(id, spi) }?;

        // Extended power cycle: ensure sensor is off first for longer
        enable_pin
            .set_low()
            .map_err(|_| SensorError::EnablePinFailed)?;
        delay.delay_ms(50).await; // Longer off time
        enable_pin
            .set_high()
            .map_err(|_| SensorError::EnablePinFailed)?;
        // Startup time, then time for the sensor to settle before it is created
        delay.delay_ms(60).await;

        let config = RadarConfig::new()?;
        #[cfg(feature = "defmt")]
        defmt::trace!("Creating sensor {}", id);
        let sensor = InnerSensor::new(id)?;
        Ok((registration, config, sensor))
    }

    /// Destroys the radar and returns its SPI device and pins, for example to create it again.
    ///
    /// The sensor stays powered.
    pub fn free(self) -> (&'static mut SPI, SINT, ENABLE, DLY) {
        let Radar {
            config,
            sensor,
            interrupt,
            prepared,
            registration,
            spi,
            ..
        } = self;
        let (enable_pin, delay) = sensor.into_parts();
        drop(prepared);
        drop(config);
        drop(registration);
        // SAFETY: the sensor is destroyed and the device's registration dropped, so this is its
        // only user again
        (unsafe { &mut *spi.as_ptr() }, interrupt, enable_pin, delay)
    }

    /// Returns the scratch buffer size for this radar instance.
    pub const fn scratch_buffer_size(&self) -> usize {
        SCRATCH_SIZE
    }

    /// The radar configuration.
    pub fn config(&self) -> &RadarConfig {
        &self.config
    }

    /// The radar configuration, for changing it.
    ///
    /// When the sensor was prepared with [`Radar::prepare_sensor`], it keeps measuring with the
    /// configuration it was prepared with, so the radar leaves the ready state and must be
    /// prepared again. Detectors use their own configuration and are not affected.
    pub fn config_mut(&mut self) -> &mut RadarConfig {
        if matches!(self.prepared, Prepared::Radar(_)) {
            self.unprepare();
        }
        &mut self.config
    }

    /// The processing for the prepared configuration, once [`Radar::prepare_sensor`] succeeded.
    pub fn processing(&self) -> Option<&Processing> {
        match &self.prepared {
            Prepared::Radar(processing) => Some(processing),
            _ => None,
        }
    }

    /// Processes a frame read by [`Radar::measure`].
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::NotReady`] when the sensor was not prepared with
    /// [`Radar::prepare_sensor`], or the radar is not ready, for example after a failed
    /// measurement. Returns [`SensorError::BufferTooSmall`] when `data` is shorter
    /// than [`Processing::buffer_size`], and [`SensorError::BufferMisaligned`] when it is not
    /// aligned for any built-in type.
    pub fn process(&mut self, data: &mut [u8]) -> Result<ProcessingResult, SensorError> {
        if self.state != RadarState::Ready {
            return Err(SensorError::NotReady);
        }
        match &mut self.prepared {
            Prepared::Radar(processing) => processing.execute(data),
            _ => Err(SensorError::NotReady),
        }
    }

    /// Prepares the sensor for the current configuration and creates the matching processing.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::BufferTooSmall`] when `SCRATCH_SIZE` is smaller than the
    /// configuration's buffer size.
    pub fn prepare_sensor(
        &mut self,
        calibration_result: &mut CalibrationResult,
    ) -> Result<(), SensorError> {
        if self.state != RadarState::Enabled {
            return Err(SensorError::NotReady);
        }

        let processing = Processing::new(&self.config)?;
        if processing.buffer_size() > SCRATCH_SIZE {
            return Err(SensorError::BufferTooSmall);
        }
        self.unprepare();
        let prepared = self
            .sensor
            .prepare(&self.config, calibration_result, &mut self.scratch);
        self.fault_on_spi_error(prepared)?;
        self.prepared = Prepared::Radar(processing);
        self.state = RadarState::Ready;
        Ok(())
    }

    /// Marks the sensor's state as unknown until it is power-cycled.
    pub(crate) fn fault(&mut self) {
        self.prepared = Prepared::None;
        self.state = RadarState::Faulted;
    }

    /// Faults the radar if `result` is an error.
    pub(crate) fn fault_on_error<T>(
        &mut self,
        result: Result<T, SensorError>,
    ) -> Result<T, SensorError> {
        if result.is_err() {
            self.fault();
        }
        result
    }

    /// Faults the radar if an SPI transfer failed: the sensor may be half-configured. Used by
    /// prepares, which can be retried after a clean SDK failure.
    pub(crate) fn fault_on_spi_error<T>(
        &mut self,
        result: Result<T, SensorError>,
    ) -> Result<T, SensorError> {
        if let Err(SensorError::SpiTransferFailed) = result {
            self.fault();
        }
        result
    }

    /// Marks the radar faulted for the duration of an async operation that leaves the sensor
    /// busy if it is interrupted, so a dropped future leaves the radar faulted. The operation
    /// sets the final state when it completes. The preparation is kept for it to restore; every
    /// way out of the faulted state clears it.
    fn begin_busy(&mut self) {
        self.state = RadarState::Faulted;
    }

    /// Runs a distance detector calibration: calls `step` until it reports completion, waiting
    /// for the sensor interrupt between steps. `step` gets the completion flag to fill in and
    /// returns the SDK's success flag.
    ///
    /// The SDK prepares the sensor with the detector's own configurations, so the radar is
    /// enabled and unprepared afterwards. A failure leaves it faulted.
    #[cfg(feature = "distance")]
    pub(crate) async fn run_detector_calibration(
        &mut self,
        mut step: impl FnMut(&mut bool) -> Result<bool, SensorError>,
    ) -> Result<(), SensorError> {
        self.check_awake()?;
        self.prepared = Prepared::None;
        self.begin_busy();
        loop {
            let mut complete = false;
            let attempt = step(&mut complete)
                .and_then(|success| success.then_some(()).ok_or(SensorError::CalibrationFailed));
            self.fault_on_error(attempt)?;
            if complete {
                break;
            }
            let waited = self
                .interrupt
                .wait_for_high()
                .await
                .map_err(|_| SensorError::CalibrationFailed);
            self.fault_on_error(waited)?;
        }
        self.state = RadarState::Enabled;
        Ok(())
    }

    /// Fails with [`SensorError::NotReady`] unless the sensor is powered and in a known state.
    #[cfg(any(feature = "distance", feature = "presence"))]
    pub(crate) fn check_awake(&self) -> Result<(), SensorError> {
        match self.state {
            RadarState::Enabled | RadarState::Ready => Ok(()),
            RadarState::Hibernating | RadarState::Faulted => Err(SensorError::NotReady),
        }
    }

    /// Forgets the preparation, before the sensor is prepared again or after it lost it.
    pub(crate) fn unprepare(&mut self) {
        self.prepared = Prepared::None;
        if self.state == RadarState::Ready {
            self.state = RadarState::Enabled;
        }
    }

    /// Wakes the sensor from hibernation: enables it, then restores its state.
    ///
    /// The radar is ready again, unless its configuration was changed while hibernating, in
    /// which case it must be prepared again.
    ///
    /// If waking fails, the sensor may be half-woken: the radar is
    /// [faulted](RadarState::Faulted).
    pub async fn hibernate_off(&mut self) -> Result<(), SensorError> {
        if self.state != RadarState::Hibernating {
            return Err(SensorError::NotReady);
        }

        self.sensor.enable_sensor().await?;
        let woken = self.sensor.hibernate_off();
        self.fault_on_error(woken)?;
        self.state = match self.prepared {
            Prepared::None => RadarState::Enabled,
            _ => RadarState::Ready,
        };
        Ok(())
    }

    /// Measures and reads a frame into `data`.
    ///
    /// # Errors
    ///
    /// `data` must be aligned for any built-in type (see [`RadarBuffer`]), or this returns
    /// [`SensorError::BufferMisaligned`]. It must hold the buffer size of the prepared
    /// configuration or detector, or this returns [`SensorError::BufferTooSmall`].
    ///
    /// If the measurement or the read fails, or the future is dropped before it completes, the
    /// sensor may still be measuring: the radar is [faulted](RadarState::Faulted).
    pub async fn measure(&mut self, data: &mut [u8]) -> Result<(), SensorError> {
        if self.state != RadarState::Ready {
            return Err(SensorError::NotReady);
        }
        let min_len = match &self.prepared {
            Prepared::None => 0,
            Prepared::Radar(processing) => processing.buffer_size(),
            #[cfg(any(feature = "distance", feature = "presence"))]
            Prepared::Detector { buffer_size } => *buffer_size,
        };
        check_buffer(data, min_len)?;

        self.begin_busy();
        let measured = self.sensor.measure(&mut self.interrupt).await;
        self.fault_on_error(measured)?;
        let read = self.sensor.read(data);
        self.fault_on_error(read)?;
        self.state = RadarState::Ready;
        Ok(())
    }

    /// Puts the sensor in hibernation: saves its state, then disables it with the enable pin.
    ///
    /// If disabling fails, the sensor is hibernating but still powered; the radar is in the
    /// hibernating state either way, and [`Radar::hibernate_off`] wakes it.
    ///
    /// If hibernating fails, the sensor may or may not be hibernating: the radar is
    /// [faulted](RadarState::Faulted).
    pub async fn hibernate_on(&mut self) -> Result<(), SensorError> {
        if self.state != RadarState::Ready {
            return Err(SensorError::NotReady);
        }

        let hibernated = self.sensor.hibernate_on();
        self.fault_on_error(hibernated)?;
        self.state = RadarState::Hibernating;
        self.sensor.disable_sensor().await
    }

    /// Get the current state of the radar
    pub fn state(&self) -> RadarState {
        self.state
    }

    /// Marks the sensor as prepared after a detector prepared it through the SDK, for
    /// measurements into buffers of `buffer_size` bytes.
    #[cfg(any(feature = "distance", feature = "presence"))]
    pub(crate) fn mark_prepared(&mut self, buffer_size: usize) {
        self.prepared = Prepared::Detector { buffer_size };
        self.state = RadarState::Ready;
    }

    /// Get the radar sensor ID
    pub fn id(&self) -> u32 {
        self.sensor.id()
    }

    /// Calibrate the sensor - available in any state
    ///
    /// Calibration power-cycles the sensor, so the radar is back in the enabled state
    /// afterwards and must be prepared again before measuring.
    ///
    /// The SDK needs a scratch buffer of at least the configuration's buffer size; calibration
    /// fails if `SCRATCH_SIZE` is too small.
    ///
    /// If calibration fails, or the future is dropped before it completes, the radar is
    /// [faulted](RadarState::Faulted).
    pub async fn calibrate(&mut self) -> Result<CalibrationResult, SensorError> {
        self.prepared = Prepared::None;
        self.begin_busy();
        let calibrated = self
            .sensor
            .calibrate(&mut self.interrupt, &mut self.scratch)
            .await;
        let calibration = self.fault_on_error(calibrated)?;
        self.state = RadarState::Enabled;
        Ok(calibration)
    }

    /// Reset the sensor - available in any state
    ///
    /// Recovers a [faulted](RadarState::Faulted) radar. If the enable pin fails, or the future is
    /// dropped before it completes, the radar stays faulted.
    pub async fn reset_sensor(&mut self) -> Result<(), SensorError> {
        self.prepared = Prepared::None;
        self.begin_busy();
        let reset = self.sensor.reset_sensor().await;
        self.fault_on_error(reset)?;
        // The sensor is unprepared after a power cycle
        self.state = RadarState::Enabled;
        Ok(())
    }

    /// Check if sensor is connected - available in any state
    ///
    /// Returns `false` if an SPI transfer failed while checking.
    pub fn is_connected(&self) -> bool {
        let id = self.id();
        spi_checked(id, || unsafe {
            acc_sensor_connected(id as acc_sensor_id_t)
        })
        .unwrap_or(false)
    }

    /// Logs the sensor status through the SDK logger - available in any state
    pub fn check_status(&self) -> Result<(), SensorError> {
        self.sensor.check_status()
    }

    /// Get raw sensor pointer - available in any state
    ///
    /// SDK calls made through it bypass the SPI failure checks and the radar's state tracking.
    /// # Safety
    /// This function is unsafe because it returns a raw pointer to the sensor.
    pub unsafe fn inner_sensor(&self) -> *mut acc_sensor_t {
        debug_assert!(!self.sensor.inner().is_null(), "Sensor pointer is null");
        self.sensor.inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hal::tests::MockSpi;
    use core::convert::Infallible;
    use core::future::Future;
    use core::task::{Context, Poll, Waker};

    struct MockPin;

    impl embedded_hal::digital::ErrorType for MockPin {
        type Error = Infallible;
    }

    impl OutputPin for MockPin {
        fn set_low(&mut self) -> Result<(), Infallible> {
            Ok(())
        }

        fn set_high(&mut self) -> Result<(), Infallible> {
            Ok(())
        }
    }

    impl Wait for MockPin {
        async fn wait_for_high(&mut self) -> Result<(), Infallible> {
            Ok(())
        }

        async fn wait_for_low(&mut self) -> Result<(), Infallible> {
            Ok(())
        }

        async fn wait_for_rising_edge(&mut self) -> Result<(), Infallible> {
            Ok(())
        }

        async fn wait_for_falling_edge(&mut self) -> Result<(), Infallible> {
            Ok(())
        }

        async fn wait_for_any_edge(&mut self) -> Result<(), Infallible> {
            Ok(())
        }
    }

    struct NoDelay;

    impl DelayNs for NoDelay {
        async fn delay_ns(&mut self, _ns: u32) {}
    }

    /// Runs a future whose awaits all complete immediately.
    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = core::pin::pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
                return output;
            }
        }
    }

    #[test]
    fn failed_creation_returns_the_resources_and_frees_the_sensor() {
        let _lock = crate::hal::tests::lock_static_slots();
        let spi: &'static mut MockSpi =
            alloc::boxed::Box::leak(alloc::boxed::Box::new(MockSpi { fail: false }));
        let spi_addr = spi as *const MockSpi;
        let error = block_on(Radar::<MockSpi, MockPin, MockPin, NoDelay>::new(
            2001, spi, MockPin, MockPin, NoDelay,
        ))
        .err()
        .unwrap();
        // The stub library cannot create a configuration
        assert_eq!(error.error, SensorError::InitFailed);
        assert_eq!(error.spi as *const MockSpi, spi_addr);
        // The sensor id is free again: a retry fails the same way, not as unavailable
        let retry = block_on(Radar::<MockSpi, MockPin, MockPin, NoDelay>::new(
            2001,
            error.spi,
            error.interrupt,
            error.enable_pin,
            error.delay,
        ))
        .err()
        .unwrap();
        assert_eq!(retry.error, SensorError::InitFailed);
        assert_eq!(SensorError::from(retry), SensorError::InitFailed);
    }

    #[test]
    fn radar_buffer_is_aligned_for_the_sdk() {
        assert_eq!(core::mem::align_of::<RadarBuffer<5560>>(), SDK_ALIGN);
        let buffer = RadarBuffer::<5560>::new();
        assert_eq!(buffer.len(), 5560);
        assert!((buffer.as_ptr() as usize).is_multiple_of(SDK_ALIGN));
    }

    #[test]
    fn aligned_buffer_covers_its_length() {
        for len in [0, 1, 16, 17, 5560] {
            let mut buffer = AlignedBuffer::new(len);
            assert_eq!(buffer.len(), len);
            assert!((buffer.as_ptr() as usize).is_multiple_of(SDK_ALIGN));
            buffer.fill(0xAB);
            assert!(buffer.iter().all(|&b| b == 0xAB));
        }
    }

    #[test]
    fn buffer_checks() {
        assert_eq!(check_layout(10, 0, 11), Err(SensorError::BufferTooSmall));
        assert_eq!(check_layout(10, 4, 10), Err(SensorError::BufferMisaligned));
        assert_eq!(check_layout(10, SDK_ALIGN * 3, 10), Ok(()));
        assert_eq!(check_layout(10, SDK_ALIGN, 0), Ok(()));
        let buffer = RadarBuffer::<32>::new();
        assert_eq!(
            check_buffer(&buffer[1..], 0),
            Err(SensorError::BufferMisaligned)
        );
        assert_eq!(check_buffer(&buffer, 32), Ok(()));
    }
}
