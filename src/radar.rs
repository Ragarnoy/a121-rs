pub mod version;

use a121_sys::{acc_sensor_connected, acc_sensor_id_t, acc_sensor_t};
use core::marker::PhantomData;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::{ErrorKind as SpiErrorKind, SpiDevice};
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::digital::Wait;
use state_shift::{impl_state, type_state};

use crate::config::RadarConfig;
use crate::hal::AccHalImpl;
use crate::processing::Processing;
use crate::sensor::calibration::CalibrationResult;
use crate::sensor::error::SensorError;
use crate::sensor::Sensor;

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
    pub config: RadarConfig,
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
        let config = RadarConfig::default();
        let sensor = Sensor::new(id, enable_pin, delay).expect("Failed to create sensor");
        let processing = Processing::new(&config);
        Radar {
            id,
            config,
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
    ) -> Result<Radar<SINT, ENABLE, DLY>, SensorError> {
        let mut buf = [0u8; 2560];
        if self
            .sensor
            .prepare(&self.config, calibration_result, &mut buf)
            .is_ok()
        {
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
            Err(SensorError::PrepareFailed)
        }
    }

    #[require(Hibernating)]
    #[switch_to(Ready)]
    pub fn hibernate_off(self) -> Result<Radar<SINT, ENABLE, DLY>, SensorError> {
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
            Err(SensorError::HibernationOffFailed)
        }
    }

    #[require(Ready)]
    pub async fn measure(&mut self, data: &mut [u8]) -> Result<(), SensorError> {
        if (self.sensor.measure(&mut self.interrupt).await).is_ok() {
            if self.sensor.read(data).is_ok() {
                Ok(())
            } else {
                Err(SensorError::ReadError)
            }
        } else {
            Err(SensorError::MeasurementError)
        }
    }

    #[require(Ready)]
    #[switch_to(Hibernating)]
    pub fn hibernate_on(mut self) -> Result<Radar<SINT, ENABLE, DLY>, SensorError> {
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
            Err(SensorError::HibernationOnFailed)
        }
    }

    // Methods available in all states
    #[require(A)]
    pub fn id(&self) -> u32 {
        self.id
    }

    #[require(A)]
    pub async fn calibrate(&mut self) -> Result<CalibrationResult, SensorError> {
        let mut buf = [0u8; 5560];
        self.sensor.calibrate(&mut self.interrupt, &mut buf).await
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
}
