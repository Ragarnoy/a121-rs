pub mod version;

use a121_sys::{acc_sensor_connected, acc_sensor_id_t, acc_sensor_t};
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::{ErrorKind as SpiErrorKind, SpiDevice};
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::digital::Wait;

use alloc::vec;
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

use crate::config::RadarConfig;
use crate::hal::{spi_checked, AccHalImpl, SDK_ALIGN};
use crate::processing::{Processing, ProcessingResult};
use crate::sensor::calibration::CalibrationResult;
use crate::sensor::error::SensorError;
use crate::sensor::Sensor;

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
/// # Example
///
/// ```ignore
/// // Use default buffer size
/// let radar: Radar<_, _, _> = Radar::new(id, spi, interrupt, enable, delay).await?;
///
/// // Use custom buffer size for larger configurations
/// let radar: Radar<_, _, _, 8192> = Radar::new(id, spi, interrupt, enable, delay).await?;
/// ```
pub struct Radar<SINT, ENABLE, DLY, const SCRATCH_SIZE: usize = DEFAULT_SCRATCH_BUFFER_SIZE>
where
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    id: u32,
    pub(crate) config: RadarConfig,
    sensor: Sensor<ENABLE, DLY>,
    pub(crate) interrupt: SINT,
    _hal: AccHalImpl,
    state: RadarState,
    /// Kept while hibernating, so waking up knows whether the sensor is still prepared
    prepared: Prepared,
    scratch: RadarBuffer<SCRATCH_SIZE>,
}

impl<SINT, ENABLE, DLY, const SCRATCH_SIZE: usize> Radar<SINT, ENABLE, DLY, SCRATCH_SIZE>
where
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    pub async fn new<SPI>(
        id: u32,
        spi: &'static mut SPI,
        interrupt: SINT,
        mut enable_pin: ENABLE,
        mut delay: DLY,
    ) -> Result<Radar<SINT, ENABLE, DLY, SCRATCH_SIZE>, SensorError>
    where
        SPI: SpiDevice<u8, Error = SpiErrorKind> + Send + 'static,
    {
        // Extended power cycle: ensure sensor is off first for longer
        enable_pin
            .set_low()
            .map_err(|_| SensorError::EnablePinFailed)?;
        delay.delay_ms(50).await; // Longer off time
        enable_pin
            .set_high()
            .map_err(|_| SensorError::EnablePinFailed)?;
        delay.delay_ms(50).await; // Longer startup time

        // Create and register HAL before creating sensor
        let hal = AccHalImpl::new(spi);
        hal.register()?;

        // Additional delay after HAL registration for sensor to stabilize
        delay.delay_ms(10).await;

        // Create configuration first
        let config = RadarConfig::new()?;

        // Create sensor after HAL is registered and sensor is stable
        let sensor = Sensor::new(id, enable_pin, delay)?;

        Ok(Radar {
            id,
            config,
            interrupt,
            sensor,
            _hal: hal,
            state: RadarState::Enabled,
            prepared: Prepared::None,
            scratch: RadarBuffer::new(),
        })
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
    /// [`Radar::prepare_sensor`], and the errors of [`Processing::execute`].
    pub fn process(&mut self, data: &mut [u8]) -> Result<ProcessingResult, SensorError> {
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
        self.sensor
            .prepare(&self.config, calibration_result, &mut self.scratch)?;
        self.prepared = Prepared::Radar(processing);
        self.state = RadarState::Ready;
        Ok(())
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
    pub async fn hibernate_off(&mut self) -> Result<(), SensorError> {
        if self.state != RadarState::Hibernating {
            return Err(SensorError::NotReady);
        }

        self.sensor.enable_sensor().await?;
        if let Err(e) = self.sensor.hibernate_off() {
            // Still hibernating: power the sensor down again so the state stays truthful
            let _ = self.sensor.disable_sensor().await;
            return Err(e);
        }
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

        self.sensor.measure(&mut self.interrupt).await?;
        self.sensor.read(data)?;
        Ok(())
    }

    /// Puts the sensor in hibernation: saves its state, then disables it with the enable pin.
    ///
    /// If disabling fails, the sensor is hibernating but still powered; the radar is in the
    /// hibernating state either way, and [`Radar::hibernate_off`] wakes it.
    ///
    /// If an SPI transfer fails, the sensor may or may not be hibernating. The radar leaves the
    /// ready state so it cannot measure; [`Radar::reset_sensor`] recovers it.
    pub async fn hibernate_on(&mut self) -> Result<(), SensorError> {
        if self.state != RadarState::Ready {
            return Err(SensorError::NotReady);
        }

        if let Err(e) = self.sensor.hibernate_on() {
            if e == SensorError::SpiTransferFailed {
                self.prepared = Prepared::None;
                self.state = RadarState::Enabled;
            }
            return Err(e);
        }
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
        self.id
    }

    /// Calibrate the sensor - available in any state
    ///
    /// Calibration power-cycles the sensor, so the radar is back in the enabled state
    /// afterwards and must be prepared again before measuring.
    ///
    /// The SDK needs a scratch buffer of at least the configuration's buffer size; calibration
    /// fails if `SCRATCH_SIZE` is too small.
    pub async fn calibrate(&mut self) -> Result<CalibrationResult, SensorError> {
        self.prepared = Prepared::None;
        self.state = RadarState::Enabled;
        self.sensor
            .calibrate(&mut self.interrupt, &mut self.scratch)
            .await
    }

    /// Reset the sensor - available in any state
    pub async fn reset_sensor(&mut self) -> Result<(), SensorError> {
        self.prepared = Prepared::None;
        // The sensor is unprepared after a power cycle, even if re-enabling fails
        self.state = RadarState::Enabled;
        self.sensor.reset_sensor().await
    }

    /// Check if sensor is connected - available in any state
    ///
    /// Returns `false` if an SPI transfer failed while checking.
    pub fn is_connected(&self) -> bool {
        spi_checked(|| unsafe { acc_sensor_connected(self.id as acc_sensor_id_t) }).unwrap_or(false)
    }

    /// Logs the sensor status through the SDK logger - available in any state
    pub fn check_status(&self) -> Result<(), SensorError> {
        self.sensor.check_status()
    }

    /// Get raw sensor pointer - available in any state
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
