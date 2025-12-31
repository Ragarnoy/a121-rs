pub mod version;

use a121_sys::{acc_sensor_connected, acc_sensor_id_t, acc_sensor_t};
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::{ErrorKind as SpiErrorKind, SpiDevice};
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::digital::Wait;

use crate::config::RadarConfig;
use crate::hal::AccHalImpl;
use crate::processing::Processing;
use crate::sensor::calibration::CalibrationResult;
use crate::sensor::error::SensorError;
use crate::sensor::Sensor;

/// Default scratch buffer size for radar operations (calibration + measurements).
///
/// This value is sufficient for most configurations. For larger configurations,
/// use `Radar::with_scratch_size` or increase `SCRATCH_SIZE` const generic.
pub const DEFAULT_SCRATCH_BUFFER_SIZE: usize = 5_560;

/// Buffer size needed for sensor prepare operations.
pub const PREPARE_BUFFER_SIZE: usize = 2_560;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RadarState {
    Enabled,
    Ready,
    Hibernating,
}

/// Radar sensor controller with configurable scratch buffer size.
///
/// The `SCRATCH_SIZE` const generic allows compile-time configuration of the
/// internal scratch buffer. Use [`DEFAULT_SCRATCH_BUFFER_SIZE`] for most cases.
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
    pub config: RadarConfig,
    sensor: Sensor<ENABLE, DLY>,
    pub processing: Processing,
    pub(crate) interrupt: SINT,
    _hal: AccHalImpl,
    state: RadarState,
    scratch: [u8; SCRATCH_SIZE],
}

impl<SINT, ENABLE, DLY, const SCRATCH_SIZE: usize> Radar<SINT, ENABLE, DLY, SCRATCH_SIZE>
where
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    const _SCRATCH_SIZE_CHECK: () = assert!(
        SCRATCH_SIZE >= PREPARE_BUFFER_SIZE,
        "SCRATCH_SIZE must be >= PREPARE_BUFFER_SIZE (2560)"
    );

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
        enable_pin.set_low().map_err(|_| SensorError::InitFailed)?;
        delay.delay_ms(50).await; // Longer off time
        enable_pin.set_high().map_err(|_| SensorError::InitFailed)?;
        delay.delay_ms(50).await; // Longer startup time

        // Create and register HAL before creating sensor
        let hal = AccHalImpl::new(spi);
        hal.register()?;

        // Additional delay after HAL registration for sensor to stabilize
        delay.delay_ms(10).await;

        // Create configuration first
        let config = RadarConfig::new()?;

        // Create sensor after HAL is registered and sensor is stable
        let sensor = Sensor::new(id, enable_pin, delay).ok_or(SensorError::InitFailed)?;
        let processing = Processing::new(&config);

        Ok(Radar {
            id,
            config,
            interrupt,
            sensor,
            processing,
            _hal: hal,
            state: RadarState::Enabled,
            scratch: [0u8; SCRATCH_SIZE],
        })
    }

    /// Returns the scratch buffer size for this radar instance.
    pub const fn scratch_buffer_size(&self) -> usize {
        SCRATCH_SIZE
    }

    pub fn prepare_sensor(
        &mut self,
        calibration_result: &mut CalibrationResult,
    ) -> Result<(), SensorError> {
        if self.state != RadarState::Enabled {
            return Err(SensorError::NotReady);
        }

        let buf = &mut self.scratch[..PREPARE_BUFFER_SIZE];
        self.sensor.prepare(&self.config, calibration_result, buf)?;
        self.state = RadarState::Ready;
        Ok(())
    }

    pub fn hibernate_off(&mut self) -> Result<(), SensorError> {
        if self.state != RadarState::Hibernating {
            return Err(SensorError::NotReady);
        }

        self.sensor.hibernate_off()?;
        self.state = RadarState::Ready;
        Ok(())
    }

    pub async fn measure(&mut self, data: &mut [u8]) -> Result<(), SensorError> {
        if self.state != RadarState::Ready {
            return Err(SensorError::NotReady);
        }

        self.sensor.measure(&mut self.interrupt).await?;
        self.sensor.read(data)?;
        Ok(())
    }

    pub fn hibernate_on(&mut self) -> Result<(), SensorError> {
        if self.state != RadarState::Ready {
            return Err(SensorError::NotReady);
        }

        self.sensor.hibernate_on()?;
        self.state = RadarState::Hibernating;
        Ok(())
    }

    /// Get the current state of the radar
    pub fn state(&self) -> RadarState {
        self.state
    }

    /// Get the radar sensor ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Calibrate the sensor - available in any state
    pub async fn calibrate(&mut self) -> Result<CalibrationResult, SensorError> {
        let buf = &mut self.scratch[..];
        self.sensor.calibrate(&mut self.interrupt, buf).await
    }

    /// Reset the sensor - available in any state
    pub async fn reset_sensor(&mut self) {
        self.sensor.reset_sensor().await;
        // Reset to enabled state after sensor reset
        self.state = RadarState::Enabled;
    }

    /// Check if sensor is connected - available in any state
    pub fn is_connected(&self) -> bool {
        unsafe { acc_sensor_connected(self.id as acc_sensor_id_t) }
    }

    /// Check sensor status - available in any state
    pub fn check_status(&self) {
        self.sensor.check_status();
    }

    /// Get raw sensor pointer - available in any state
    /// # Safety
    /// This function is unsafe because it returns a raw pointer to the sensor.
    pub unsafe fn inner_sensor(&self) -> *mut acc_sensor_t {
        debug_assert!(!self.sensor.inner().is_null(), "Sensor pointer is null");
        self.sensor.inner()
    }
}
