pub mod error;
pub mod version;

use crate::config::RadarConfig;
use crate::config::RadarConfigUnlocked;
use crate::hal::AccHalImpl;
use crate::processing::Processing;
use crate::radar::error::RadarError;
use crate::sensor::calibration::CalibrationResult;
use crate::sensor::error::SensorError::*;
use crate::sensor::Sensor;
use a121_sys::{acc_sensor_connected, acc_sensor_id_t, acc_sensor_t};
use core::marker::PhantomData;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::{ErrorKind as SpiErrorKind, SpiDevice};
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::digital::Wait;
use state_shift::{impl_state, type_state};

#[type_state(
    states = (Enabled, Ready, Hibernating),
    slots = (Enabled)
)]
pub struct Radar<SINT, ENABLE, DLY>
where
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    id: u32,
    config: Option<RadarConfig>,
    sensor: Sensor<ENABLE, DLY>,
    pub processing: Processing,
    pub(crate) interrupt: SINT,
    _hal: AccHalImpl,
}

#[impl_state]
impl<SINT, ENABLE, DLY> Radar<SINT, ENABLE, DLY>
where
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    #[require(Enabled)]
    pub async fn new<SPI>(
        id: u32,
        spi: &'static mut SPI,
        interrupt: SINT,
        mut enable_pin: ENABLE,
        mut delay: DLY,
    ) -> Radar<SINT, ENABLE, DLY>
    where
        SPI: SpiDevice<u8, Error = SpiErrorKind> + Send + 'static,
    {
        enable_pin.set_high().unwrap();
        delay.delay_ms(2).await;
        let hal = AccHalImpl::new(spi);
        hal.register();
        let config = RadarConfig::new();
        let sensor = Sensor::new(id, enable_pin, delay).expect("Failed to create sensor");
        let processing = Processing::new(&config);
        Radar {
            id,
            config: Some(config),
            interrupt,
            sensor,
            processing,
            _hal: hal,
        }
    }

    #[require(Enabled)]
    #[switch_to(Ready)]
    pub fn prepare_sensor(
        mut self,
        calibration_result: &mut CalibrationResult,
    ) -> Result<Radar<SINT, ENABLE, DLY>, RadarError> {
        let mut buf = [0u8; 2560];
        let config = &self.config.as_ref().unwrap();
        if self
            .sensor
            .prepare(config, calibration_result, &mut buf)
            .is_ok()
        {
            Ok(Radar {
                id: self.id,
                config: Some(self.config.unwrap()),
                sensor: self.sensor,
                processing: self.processing,
                interrupt: self.interrupt,
                _hal: self._hal,
                _state: PhantomData,
            })
        } else {
            Err(RadarError::SensorError(PrepareFailed))
        }
    }

    #[require(Hibernating)]
    #[switch_to(Ready)]
    pub fn hibernate_off(self) -> Result<Radar<SINT, ENABLE, DLY>, RadarError> {
        if self.sensor.hibernate_off().is_ok() {
            Ok(Radar {
                id: self.id,
                config: self.config,
                sensor: self.sensor,
                processing: self.processing,
                interrupt: self.interrupt,
                _hal: self._hal,
                _state: PhantomData,
            })
        } else {
            Err(RadarError::SensorError(HibernationOffFailed))
        }
    }

    #[require(Ready)]
    pub async fn measure(&mut self, data: &mut [u8]) -> Result<(), RadarError> {
        if (self.sensor.measure(&mut self.interrupt).await).is_ok() {
            if self.sensor.read(data).is_ok() {
                Ok(())
            } else {
                Err(RadarError::SensorError(ReadError))
            }
        } else {
            Err(RadarError::SensorError(MeasurementError))
        }
    }

    #[require(Ready)]
    #[switch_to(Hibernating)]
    pub fn hibernate_on(mut self) -> Result<Radar<SINT, ENABLE, DLY>, RadarError> {
        if self.sensor.hibernate_on().is_ok() {
            Ok(Radar {
                id: self.id,
                config: self.config,
                sensor: self.sensor,
                processing: self.processing,
                interrupt: self.interrupt,
                _hal: self._hal,
                _state: PhantomData,
            })
        } else {
            Err(RadarError::SensorError(HibernationOnFailed))
        }
    }

    // Methods available in all states
    #[require(A)]
    pub fn id(&self) -> u32 {
        self.id
    }

    #[require(A)]
    pub async fn calibrate(&mut self) -> Result<CalibrationResult, RadarError> {
        let mut buf = [0u8; 5560];
        self.sensor
            .calibrate(&mut self.interrupt, &mut buf)
            .await
            .map_err(|e| RadarError::SensorError(e))
    }

    #[require(A)]
    pub async fn reset_sensor(&mut self) {
        self.sensor.reset_sensor().await;
    }

    #[require(A)]
    pub fn is_connected(&self) -> bool {
        unsafe { acc_sensor_connected(self.id as acc_sensor_id_t) }
    }

    #[require(A)]
    pub fn check_status(&self) {
        self.sensor.check_status();
    }

    #[require(A)]
    pub unsafe fn inner_sensor(&self) -> *mut acc_sensor_t {
        debug_assert!(!self.sensor.inner().is_null(), "Sensor pointer is null");
        self.sensor.inner()
    }

    /// Applies configuration changes to the radar sensor atomically.
    ///
    /// Temporarily unlocks the configuration, applies changes via the provided closure,
    /// resets the sensor, then locks the configuration again. If any step fails, the
    /// configuration remains unchanged.
    ///
    /// # Arguments
    /// * `f` - Closure receiving mutable access to unlocked configuration, returns Result
    ///
    /// # Example
    /// ```no_run
    /// radar.apply_config(|config| {
    ///     config.set_start_point(100);
    ///     config.set_profile(RadarProfile::AccProfile1)?;
    ///     Ok(())
    /// }).await?;
    /// ```
    #[require(Ready)]
    pub async fn apply_config<F, E>(&mut self, f: F) -> Result<(), RadarError>
    where
        E: Into<RadarError>,
        F: FnOnce(&mut RadarConfig<RadarConfigUnlocked>) -> Result<(), E>,
    {
        // Unlock the config temporarily
        let mut tmp = self.config.take().unwrap().unlock();

        // Let the closure modify the config
        match f(&mut tmp) {
            Ok(()) => {
                // Reset the sensor to apply changes
                self.reset_sensor().await;

                // Lock the config again
                self.config = Some(tmp.lock());

                Ok(())
            }
            Err(e) => {
                // Lock the config again
                self.config = Some(tmp.lock());

                return Err(e.into());
            }
        }
    }

    #[require(A)]
    pub fn config(&self) -> &RadarConfig {
        self.config.as_ref().unwrap()
    }
}
