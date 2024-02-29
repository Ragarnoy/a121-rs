use core::fmt::{Debug, Display, Formatter};
use core::marker::PhantomData;

use embedded_hal::spi::{ErrorKind as SpiErrorKind, SpiDevice};
use embedded_hal_async::digital::Wait;

use crate::config::RadarConfig;
use crate::hal::AccHalImpl;
use crate::processing::Processing;
use crate::rss_bindings::{
    acc_sensor_connected, acc_sensor_id_t, acc_sensor_t, acc_version_get_hex,
};
use crate::sensor::calibration::CalibrationResult;
use crate::sensor::data::RadarData;
use crate::sensor::error::SensorError;
use crate::sensor::Sensor;

pub struct Enabled;
pub struct Ready;
pub struct Hibernating;

trait RadarState {}

impl RadarState for Enabled {}
impl RadarState for Ready {}
impl RadarState for Hibernating {}

pub struct TransitionError<STATE, SINT: Wait> {
    pub radar: Radar<STATE, SINT>,
    error: SensorError,
}

impl<S, SINT: Wait> Debug for TransitionError<S, SINT> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "TransitionError: {:?}", self.error)
    }
}

impl<S, SINT: Wait> From<TransitionError<S, SINT>> for SensorError {
    fn from(e: TransitionError<S, SINT>) -> Self {
        e.error
    }
}

pub struct Radar<STATE, SINT>
where
    SINT: Wait,
{
    id: u32,
    pub config: RadarConfig,
    sensor: Sensor,
    pub processing: Processing,
    pub(crate) interrupt: SINT,
    _hal: AccHalImpl,
    _state: PhantomData<STATE>,
}

/// Radar Sensor Software Version
/// 0xMMMMmmPP where M is major, m is minor and P is patch
#[derive(Debug)]
pub struct RssVersion {
    version: u32,
}

impl defmt::Format for RssVersion {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "{}.{}.{}", self.major(), self.minor(), self.patch())
    }
}

impl Display for RssVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.major(), self.minor(), self.patch())
    }
}

impl RssVersion {
    pub fn new(version: u32) -> Self {
        Self { version }
    }

    pub fn major(&self) -> u16 {
        ((self.version & 0xFFFF0000) >> 16) as u16
    }

    pub fn minor(&self) -> u8 {
        ((self.version & 0x0000FF00) >> 8) as u8
    }

    pub fn patch(&self) -> u8 {
        (self.version & 0x000000FF) as u8
    }
}

impl<SINT> Radar<Enabled, SINT>
where
    SINT: Wait,
{
    pub fn new<SPI>(id: u32, spi: &'static mut SPI, interrupt: SINT) -> Radar<Enabled, SINT>
    where
        SPI: SpiDevice<u8, Error = SpiErrorKind> + Send + 'static,
    {
        let hal = AccHalImpl::new(spi);
        hal.register();
        let config = RadarConfig::default();
        let sensor = Sensor::new(id).expect("Failed to create sensor");
        let processing = Processing::new(&config);
        Self {
            id,
            config,
            interrupt,
            sensor,
            processing,
            _hal: hal,
            _state: PhantomData,
        }
    }

    pub fn prepare_sensor(
        mut self,
        calibration_result: &mut CalibrationResult,
    ) -> Result<Radar<Ready, SINT>, TransitionError<Enabled, SINT>> {
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
            Err(TransitionError {
                radar: self,
                error: SensorError::PrepareFailed,
            })
        }
    }
}

impl<SINT: Wait> Radar<Hibernating, SINT> {
    pub fn hibernate_off(self) -> Result<Radar<Ready, SINT>, TransitionError<Hibernating, SINT>> {
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
            Err(TransitionError {
                radar: self,
                error: SensorError::HibernationOffFailed,
            })
        }
    }
}

impl<SINT: Wait> Radar<Ready, SINT> {
    pub async fn measure<'a>(&mut self) -> Result<RadarData, SensorError> {
        if (self.sensor.measure(&mut self.interrupt).await).is_ok() {
            let mut data = RadarData::new();
            if self.sensor.read(&mut data).is_ok() {
                Ok(data)
            } else {
                Err(SensorError::ReadError)
            }
        } else {
            Err(SensorError::MeasurementError)
        }
    }

    pub fn hibernate_on(
        mut self,
    ) -> Result<Radar<Hibernating, SINT>, TransitionError<Ready, SINT>> {
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
            Err(TransitionError {
                radar: self,
                error: SensorError::HibernationOnFailed,
            })
        }
    }
}

impl<STATE, SINT> Radar<STATE, SINT>
where
    SINT: Wait,
{
    pub fn id(&self) -> u32 {
        self.id
    }

    pub async fn calibrate(&mut self) -> Result<CalibrationResult, SensorError> {
        let mut buf = [0u8; 5560];
        self.sensor.calibrate(&mut self.interrupt, &mut buf).await
    }

    /// Checks if a sensor is connected and responsive.
    ///
    /// Note that the sensor must be powered on before calling this function.
    ///
    /// # Returns
    ///
    /// `true` if it is possible to communicate with the sensor, `false` otherwise.
    pub fn is_connected(&self) -> bool {
        unsafe { acc_sensor_connected(self.id as acc_sensor_id_t) }
    }

    /// Checks the status of the sensor.
    ///
    /// This function reads out the internal status from the sensor and can be used for
    /// debugging purposes. The log is printed out through the log interface.
    /// The sensor must be powered on before calling this function.
    pub fn check_status(&self) {
        self.sensor.check_status();
    }

    pub unsafe fn inner_sensor(&self) -> *mut acc_sensor_t {
        self.sensor.inner()
    }
}

pub fn rss_version() -> RssVersion {
    let version = unsafe { acc_version_get_hex() };
    RssVersion::new(version)
}
