use core::ffi::c_void;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::digital::Wait;

use calibration::CalibrationResult;
use error::SensorError;

use crate::config::RadarConfig;
use crate::hal::{spi_checked, with_spi_status};
use a121_sys::*;

pub mod calibration;
pub mod error;

/// Safety-critical wrapper around raw sensor pointer
pub struct InnerSensor {
    /// Non-null pointer to acc_sensor_t, guaranteed valid by construction.
    inner: NonNull<acc_sensor_t>,
    id: u32,
}

impl InnerSensor {
    /// Creates a new sensor instance from a sensor ID.
    ///
    /// # Safety
    /// - The sensor must be powered on before calling this function
    /// - The sensor must not be used in another sensor instance without a power/reset cycle
    pub fn new(sensor_id: u32) -> Result<Self, SensorError> {
        let (sensor_ptr, spi_failed) = with_spi_status(sensor_id, || unsafe {
            acc_sensor_create(sensor_id as acc_sensor_id_t)
        });
        let Some(inner) = NonNull::new(sensor_ptr) else {
            #[cfg(feature = "defmt")]
            defmt::error!("Failed to create sensor {}", sensor_id);
            return Err(if spi_failed {
                SensorError::SpiTransferFailed
            } else {
                SensorError::InitFailed
            });
        };
        // Owning the pointer destroys a sensor created despite a failed transfer
        let sensor = Self {
            inner,
            id: sensor_id,
        };
        if spi_failed {
            return Err(SensorError::SpiTransferFailed);
        }
        #[cfg(feature = "defmt")]
        defmt::trace!("Successfully created sensor {}", sensor_id);
        Ok(sensor)
    }
}

impl Drop for InnerSensor {
    fn drop(&mut self) {
        // NonNull guarantees non-null, no assertion needed
        unsafe { acc_sensor_destroy(self.inner.as_ptr()) }
    }
}

impl Deref for InnerSensor {
    type Target = acc_sensor_t;

    fn deref(&self) -> &Self::Target {
        // SAFETY: NonNull guarantees valid pointer, exclusive access by ownership
        unsafe { self.inner.as_ref() }
    }
}

impl DerefMut for InnerSensor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: NonNull guarantees valid pointer, exclusive access by &mut self
        unsafe { self.inner.as_mut() }
    }
}

pub(super) struct Sensor<ENABLE, DLY>
where
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    inner: InnerSensor,
    enable_pin: ENABLE,
    dly: DLY,
}

impl<ENABLE, DLY> Sensor<ENABLE, DLY>
where
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    /// Wraps a created sensor with its enable pin.
    pub fn new(inner: InnerSensor, enable_pin: ENABLE, delay: DLY) -> Self {
        Self {
            inner,
            enable_pin,
            dly: delay,
        }
    }

    /// Destroys the sensor and returns its enable pin and delay.
    pub fn into_parts(self) -> (ENABLE, DLY) {
        let Self {
            inner,
            enable_pin,
            dly,
        } = self;
        drop(inner);
        (enable_pin, dly)
    }

    pub async fn reset_sensor(&mut self) -> Result<(), SensorError> {
        self.disable_sensor().await?;
        self.enable_sensor().await
    }

    pub async fn enable_sensor(&mut self) -> Result<(), SensorError> {
        self.enable_pin
            .set_high()
            .map_err(|_| SensorError::EnablePinFailed)?;
        self.dly.delay_ms(2).await;
        Ok(())
    }

    pub async fn disable_sensor(&mut self) -> Result<(), SensorError> {
        self.enable_pin
            .set_low()
            .map_err(|_| SensorError::EnablePinFailed)?;
        self.dly.delay_ms(2).await;
        Ok(())
    }

    /// Calibrates the sensor asynchronously.
    pub async fn calibrate<SINT: Wait>(
        &mut self,
        interrupt: &mut SINT,
        buffer: &mut [u8],
    ) -> Result<CalibrationResult, SensorError> {
        let mut calibration_complete: bool = false;
        let mut calibration_result = CalibrationResult::new();

        self.reset_sensor().await?;

        loop {
            let calibration_attempt = spi_checked(self.inner.id, || unsafe {
                acc_sensor_calibrate(
                    self.inner.deref_mut(),
                    &mut calibration_complete as *mut bool,
                    calibration_result.mut_ptr(),
                    buffer.as_mut_ptr() as *mut c_void,
                    buffer.len() as u32,
                )
            })?;

            // Check if the calibration attempt was successful
            if !calibration_attempt {
                return Err(SensorError::CalibrationFailed);
            }

            // Break the loop if calibration is complete
            if calibration_complete {
                break;
            }

            // Wait for the interrupt signal asynchronously
            interrupt
                .wait_for_high()
                .await
                .map_err(|_| SensorError::CalibrationFailed)?;
        }

        Ok(calibration_result)
    }

    /// Prepares the sensor for measurement with a given configuration.
    ///
    /// It's possible to reconfigure the sensor by calling this function multiple times.
    /// However, the sensor must not be measuring when calling this function. If a previous
    /// call was made to `acc_sensor_measure`, use `acc_hal_integration_wait_for_sensor_interrupt`
    /// to wait for the measurement to complete.
    ///
    /// # Arguments
    /// * `config` - The configuration to prepare for.
    /// * `cal_result` - The calibration result to prepare for.
    /// * `buffer` - Memory used during preparation. A larger buffer might mean fewer transactions
    ///   between the host and sensor.
    ///
    /// # Returns
    /// `Ok(())` if successful, `Err(SensorError)` otherwise.
    pub fn prepare(
        &mut self,
        config: &RadarConfig,
        cal_result: &mut CalibrationResult,
        buffer: &mut [u8],
    ) -> Result<(), SensorError> {
        let ret = spi_checked(self.inner.id, || unsafe {
            acc_sensor_prepare(
                self.inner.deref_mut(),
                config.ptr(),
                cal_result.mut_ptr(),
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
            )
        })?;
        if ret {
            #[cfg(feature = "defmt")]
            defmt::trace!("Sensor prepared");
            Ok(())
        } else {
            Err(SensorError::PrepareFailed)
        }
    }

    pub fn check_status(&self) -> Result<(), SensorError> {
        spi_checked(self.inner.id, || unsafe {
            acc_sensor_status(self.inner.deref())
        })
    }

    /// Prepares the sensor for entering hibernation.
    ///
    /// Should be invoked prior to calling `acc_hal_integration_sensor_disable()`.
    ///
    /// # Returns
    /// `Ok(())` if preparation was successful, `Err(SensorHibernationError)` otherwise.
    pub fn hibernate_on(&mut self) -> Result<(), SensorError> {
        let ret_status = spi_checked(self.inner.id, || unsafe {
            acc_sensor_hibernate_on(self.inner.deref_mut())
        })?;
        if ret_status {
            Ok(())
        } else {
            Err(SensorError::HibernationOnFailed)
        }
    }

    /// Restores the sensor after exiting hibernation.
    ///
    /// Should be invoked after calling `acc_hal_integration_sensor_enable()`.
    ///
    /// # Returns
    /// `Ok(())` if unpreparation was successful, `Err(SensorHibernationError)` otherwise.
    pub fn hibernate_off(&self) -> Result<(), SensorError> {
        let ret_status = spi_checked(self.inner.id, || unsafe {
            acc_sensor_hibernate_off(self.inner.deref())
        })?;
        if ret_status {
            Ok(())
        } else {
            Err(SensorError::HibernationOffFailed)
        }
    }

    pub async fn measure<SINT: Wait>(&mut self, mut interrupt: SINT) -> Result<(), SensorError> {
        // Implementation to start the radar measurement
        let success = spi_checked(self.inner.id, || unsafe {
            acc_sensor_measure(self.inner.deref_mut())
        })?;
        if success {
            interrupt
                .wait_for_high()
                .await
                .map_err(|_| SensorError::MeasurementError)?;
            Ok(())
        } else {
            Err(SensorError::MeasurementError)
        }
    }

    /// Reads out radar data from the sensor.
    ///
    /// This function should be called after starting a measurement with `measure`. It reads
    /// the radar data into a provided buffer. The function will wait for the sensor interrupt
    /// to become active before attempting to read the data.
    ///
    /// # Preconditions
    /// - The sensor must be powered on.
    /// - `measure` must be called before each call to this function.
    /// - The sensor interrupt must be active.
    ///
    /// # Arguments
    /// * `buffer` - A mutable slice where the radar data will be stored.
    ///
    /// # Returns
    /// `Ok(())` if data was successfully read into the buffer, `Err(SensorError)` otherwise.
    pub fn read(&self, buffer: &mut [u8]) -> Result<(), SensorError> {
        // Implementation to read the radar data
        let success = spi_checked(self.inner.id, || unsafe {
            acc_sensor_read(
                self.inner.deref(),
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
            )
        })?;
        if success {
            Ok(())
        } else {
            Err(SensorError::ReadError)
        }
    }

    /// Returns a mutable pointer to the underlying sensor.
    ///
    /// # Safety
    /// This function is unsafe because:
    /// - The pointer remains valid only for the lifetime of this Sensor instance
    /// - The caller must not use this pointer after the Sensor is dropped
    /// - Multiple mutable references to the same sensor must not be created
    pub unsafe fn inner(&self) -> *mut acc_sensor_t {
        // NonNull guarantees non-null pointer
        self.inner.inner.as_ptr()
    }
}
