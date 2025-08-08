use core::ffi::c_void;

use core::ops::{Deref, DerefMut};
use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;

use embedded_hal_async::digital::Wait;

use calibration::CalibrationResult;
use error::SensorError;

use crate::config::RadarConfig;
use a121_sys::*;

pub mod calibration;
pub mod error;

/// Safety-critical wrapper around raw sensor pointer
pub struct InnerSensor {
    /// Raw pointer to acc_sensor_t. Must never be null once initialized.
    /// Managed exclusively by this type to maintain memory safety.
    inner: *mut acc_sensor_t,
}

impl InnerSensor {
    /// Creates a new sensor instance from a sensor ID.
    ///
    /// # Safety
    /// - The returned pointer must be valid and non-null
    /// - The sensor must be powered on before calling this function
    /// - The sensor must not be used in another sensor instance without a power/reset cycle
    pub fn new(sensor_id: u32) -> Option<Self> {
        let sensor_ptr = unsafe { acc_sensor_create(sensor_id as acc_sensor_id_t) };

        // Runtime safety check for null pointer
        if sensor_ptr.is_null() {
            #[cfg(feature = "defmt")]
            defmt::error!(
                "Failed to create sensor {}: acc_sensor_create returned null",
                sensor_id
            );
            return None;
        }

        #[cfg(feature = "defmt")]
        defmt::trace!("Successfully created sensor {}", sensor_id);
        Some(Self { inner: sensor_ptr })
    }
}

impl Drop for InnerSensor {
    fn drop(&mut self) {
        debug_assert!(!self.inner.is_null(), "Sensor pointer is null in drop");
        unsafe {
            acc_sensor_destroy(self.inner);
        }
    }
}

impl Deref for InnerSensor {
    type Target = acc_sensor_t;

    /// # Safety
    /// Dereference is safe because:
    /// - inner is guaranteed non-null by constructor
    /// - exclusive access is maintained by Rust ownership rules
    /// - pointer remains valid until drop
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner }
    }
}

impl DerefMut for InnerSensor {
    /// # Safety
    /// Dereference is safe because:
    /// - inner is guaranteed non-null by constructor
    /// - exclusive access is maintained by Rust ownership rules
    /// - pointer remains valid until drop
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.inner }
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
    /// Creates a new sensor instance for the given sensor ID.
    ///
    /// A sensor instance represents a physical radar sensor and handles communication with it.
    /// This method will power on the sensor and create a new instance of the sensor.
    ///
    /// # Arguments
    /// * `sensor_id` - The sensor ID to use for communication.
    ///
    /// # Returns
    /// `Some(Sensor)` if the sensor instance was successfully created, `None` otherwise.
    pub fn new(sensor_id: u32, enable_pin: ENABLE, delay: DLY) -> Option<Self> {
        #[cfg(feature = "defmt")]
        defmt::trace!("Creating sensor {}", sensor_id);
        let inner = InnerSensor::new(sensor_id)?;
        Some(Self {
            inner,
            enable_pin,
            dly: delay,
        })
    }

    pub async fn reset_sensor(&mut self) {
        self.disable_sensor().await;
        self.enable_sensor().await;
    }

    pub async fn enable_sensor(&mut self) {
        self.enable_pin.set_high().unwrap();
        self.dly.delay_ms(2).await;
    }

    pub async fn disable_sensor(&mut self) {
        self.enable_pin.set_low().unwrap();
        self.dly.delay_ms(2).await;
    }

    /// Calibrates the sensor asynchronously.
    pub async fn calibrate<SINT: Wait>(
        &mut self,
        interrupt: &mut SINT,
        buffer: &mut [u8],
    ) -> Result<CalibrationResult, SensorError> {
        let mut calibration_complete: bool = false;
        let mut calibration_result = CalibrationResult::new();

        self.reset_sensor().await;

        loop {
            let calibration_attempt = unsafe {
                acc_sensor_calibrate(
                    self.inner.deref_mut(),
                    &mut calibration_complete as *mut bool,
                    calibration_result.mut_ptr(),
                    buffer.as_mut_ptr() as *mut c_void,
                    buffer.len() as u32,
                )
            };

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
                .expect("Failed to wait for interrupt");
        }

        Ok(calibration_result)
    }

    ///
    /// Initiates the calibration process for the sensor and waits asynchronously for a sensor
    /// interrupt to indicate the completion or progress of the calibration.
    /// The sensor must be powered on before calling this function.
    ///
    /// The function starts the calibration process and then waits for a sensor interrupt signal.
    /// Upon receiving the interrupt signal, the function completes, returning the current
    /// calibration result.
    ///
    /// # Arguments
    /// * `buffer` - A buffer used during calibration. A larger buffer might reduce the number of
    ///   transactions between the host and the sensor. The buffer is only used during the duration
    ///   of the calibration call.
    ///
    /// # Returns
    /// `Ok(CalibrationResult)` containing the result of the calibration if the calibration step
    /// was successful.
    /// If the calibration step fails, returns `Err(SensorError::FailedCalibration)`.

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
        let ret;
        unsafe {
            ret = acc_sensor_prepare(
                self.inner.deref_mut(),
                config.ptr(),
                cal_result.mut_ptr(),
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
            );
        }
        if ret {
            #[cfg(feature = "defmt")]
            defmt::trace!("Sensor prepared");
            Ok(())
        } else {
            Err(SensorError::PrepareFailed)
        }
    }

    pub fn check_status(&self) {
        unsafe {
            acc_sensor_status(self.inner.deref());
        }
    }

    /// Prepares the sensor for entering hibernation.
    ///
    /// Should be invoked prior to calling `acc_hal_integration_sensor_disable()`.
    ///
    /// # Returns
    /// `Ok(())` if preparation was successful, `Err(SensorHibernationError)` otherwise.
    pub fn hibernate_on(&mut self) -> Result<(), SensorError> {
        let ret_status: bool;
        unsafe {
            ret_status = acc_sensor_hibernate_on(self.inner.deref_mut());
        }
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
        let ret_status: bool;
        unsafe {
            ret_status = acc_sensor_hibernate_off(self.inner.deref());
        }
        if ret_status {
            Ok(())
        } else {
            Err(SensorError::HibernationOffFailed)
        }
    }

    pub async fn measure<SINT: Wait>(&mut self, mut interrupt: SINT) -> Result<(), SensorError> {
        // Implementation to start the radar measurement
        let success = unsafe { acc_sensor_measure(self.inner.deref_mut()) };
        if success {
            interrupt
                .wait_for_high()
                .await
                .expect("Failed to wait for interrupt");
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
        let success = unsafe {
            acc_sensor_read(
                self.inner.deref(),
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
            )
        };
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
    /// - The caller must ensure the sensor has been properly initialized
    /// - The pointer remains valid only for the lifetime of this Sensor instance
    /// - The caller must not use this pointer after the Sensor is dropped
    /// - Multiple mutable references to the same sensor must not be created
    pub unsafe fn inner(&self) -> *mut acc_sensor_t {
        debug_assert!(
            !self.inner.inner.is_null(),
            "Sensor has not been initialized"
        );
        self.inner.inner
    }
}
