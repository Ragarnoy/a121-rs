use core::ffi::c_void;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use defmt::trace;

use embedded_hal_async::digital::Wait;

use calibration::CalibrationResult;
use data::RadarData;
use error::SensorError;

use crate::config::RadarConfig;
use crate::rss_bindings::*;

pub mod calibration;
pub mod data;
pub mod error;

pub struct Disabled;
pub struct Enabled;
pub struct Ready;
pub struct Hibernating;

pub struct TransitionError<S, SINT: Wait> {
    pub sensor: Sensor<S, SINT>,
    error: SensorError,
}

impl<SINT: Wait> TransitionError<Enabled, SINT> {
    pub fn error(&self) -> SensorError {
        self.error
    }
}

struct InnerSensor {
    inner: *mut acc_sensor_t,
}

impl InnerSensor {
    pub fn new(sensor_id: u32) -> Option<Self> {
        let sensor_ptr = unsafe { acc_sensor_create(sensor_id as acc_sensor_id_t) };
        if sensor_ptr.is_null() {
            None
        } else {
            Some(Self { inner: sensor_ptr })
        }
    }
}

impl Drop for InnerSensor {
    fn drop(&mut self) {
        unsafe {
            acc_sensor_destroy(self.inner);
        }
    }
}

impl Deref for InnerSensor {
    type Target = acc_sensor_t;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner }
    }
}

impl DerefMut for InnerSensor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.inner }
    }
}

pub struct Sensor<S, SINT: Wait> {
    inner: InnerSensor,
    interrupt: SINT,
    sensor_id: u32,
    prepared: bool,
    calibrated: bool,
    _state: PhantomData<S>,
}

impl<SINT: Wait> Sensor<Disabled, SINT> {
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
    pub fn new(sensor_id: u32, interrupt: SINT) -> Option<Self> {
        trace!("Creating sensor {}", sensor_id);
        let inner = InnerSensor::new(sensor_id)?;
        Some(Self {
            inner,
            interrupt,
            sensor_id,
            _state: PhantomData,
            prepared: false,
            calibrated: false,
        })
    }

    /// Enable the sensor.
    ///
    /// This function enables the sensor and resets the internal state of the sensor instance.
    /// The sensor must be powered on before calling this function.
    pub fn enable(self) -> Sensor<Enabled, SINT> {
        Sensor {
            inner: self.inner,
            interrupt: self.interrupt,
            sensor_id: self.sensor_id,
            _state: PhantomData,
            prepared: false,
            calibrated: false,
        }
    }
}

impl<SINT: Wait> Sensor<Enabled, SINT> {
    /// Disable the sensor.
    ///
    /// This function disables the sensor and resets the internal state of the sensor instance.
    /// The sensor must be powered on before calling this function.
    pub fn disable(self) -> Sensor<Disabled, SINT> {
        Sensor {
            inner: self.inner,
            interrupt: self.interrupt,
            sensor_id: self.sensor_id,
            _state: PhantomData,
            prepared: false,
            calibrated: false,
        }
    }

    /// Calibrates the sensor asynchronously.
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
    /// was successful. The calibration might still be ongoing, requiring additional calls to
    /// `calibrate`. If the calibration step fails, returns `Err(SensorError::FailedCalibration)`.
    ///
    /// # Usage
    /// This function should be called in an asynchronous context and awaited.
    /// The caller should check the status of the `CalibrationResult`
    /// to determine if additional calibration steps are required.
    pub async fn calibrate(&mut self, buffer: &mut [u8]) -> Result<CalibrationResult, SensorError> {
        let mut calibration_status: bool = false;
        let mut calibration_result = CalibrationResult::new();
        let calibration_attempt: bool;

        self.calibrated = false;
        unsafe {
            // Start the calibration process
            calibration_attempt = acc_sensor_calibrate(
                self.inner.deref_mut(),
                &mut calibration_status as *mut bool,
                calibration_result.mut_ptr(),
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
            );
        }
        if calibration_attempt {
            if !calibration_status {
                // Wait for the interrupt to occur asynchronously
                self.interrupt
                    .wait_for_low()
                    .await
                    .expect("Failed to wait for interrupt");
                unsafe {
                    acc_sensor_calibrate(
                        self.inner.deref_mut(),
                        &mut calibration_status as *mut bool,
                        calibration_result.mut_ptr(),
                        buffer.as_mut_ptr() as *mut c_void,
                        buffer.len() as u32,
                    );
                }
            }

            self.calibrated = calibration_status;
            Ok(calibration_result)
        } else {
            Err(SensorError::FailedCalibration)
        }
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
        self.prepared = false;
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
            self.prepared = true;
            Ok(())
        } else {
            Err(SensorError::FailedPrepare)
        }
    }

    /// Checks if a sensor is connected and responsive.
    ///
    /// Note that the sensor must be powered on before calling this function.
    ///
    /// # Arguments
    ///
    /// * `sensor_id` - The sensor ID to be used for communication.
    ///
    /// # Returns
    ///
    /// `true` if it is possible to communicate with the sensor, `false` otherwise.
    pub fn is_connected(sensor_id: u32) -> bool {
        unsafe { acc_sensor_connected(sensor_id as acc_sensor_id_t) }
    }

    /// Checks the status of the sensor.
    ///
    /// This function reads out the internal status from the sensor and can be used for
    /// debugging purposes, such as when `acc_hal_integration_wait_for_sensor_interrupt()`
    /// fails. The sensor must be powered on before calling this function.
    ///
    /// # Returns
    /// `Ok(())` if successful, `Err(SensorStatusError)` otherwise.
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
    pub fn hibernate_on(
        mut self,
    ) -> Result<Sensor<Hibernating, SINT>, TransitionError<Enabled, SINT>> {
        let ret_status: bool;
        unsafe {
            ret_status = acc_sensor_hibernate_on(self.inner.deref_mut());
        }
        if ret_status {
            Ok(Sensor {
                inner: self.inner,
                interrupt: self.interrupt,
                sensor_id: self.sensor_id,
                _state: PhantomData,
                prepared: false,
                calibrated: false,
            })
        } else {
            Err(TransitionError {
                sensor: self,
                error: SensorError::FailedHibernate,
            })
        }
    }

    pub fn set_ready(self) -> Result<Sensor<Ready, SINT>, TransitionError<Enabled, SINT>> {
        if self.prepared && self.calibrated {
            Ok(Sensor {
                inner: self.inner,
                interrupt: self.interrupt,
                sensor_id: self.sensor_id,
                _state: PhantomData,
                prepared: true,
                calibrated: true,
            })
        } else {
            Err(TransitionError {
                sensor: self,
                error: SensorError::NotReady,
            })
        }
    }
}

impl<SINT: Wait> Sensor<Hibernating, SINT> {
    /// Restores the sensor after exiting hibernation.
    ///
    /// Should be invoked after calling `acc_hal_integration_sensor_enable()`.
    ///
    /// # Returns
    /// `Ok(())` if unpreparation was successful, `Err(SensorHibernationError)` otherwise.
    pub fn hibernate_off(
        mut self,
    ) -> Result<Sensor<Enabled, SINT>, TransitionError<Hibernating, SINT>> {
        let ret_status: bool;
        unsafe {
            ret_status = acc_sensor_hibernate_off(self.inner.deref_mut());
        }
        if ret_status {
            Ok(Sensor {
                inner: self.inner,
                interrupt: self.interrupt,
                sensor_id: self.sensor_id,
                _state: PhantomData,
                prepared: false,
                calibrated: false,
            })
        } else {
            Err(TransitionError {
                sensor: self,
                error: SensorError::FailedHibernate,
            })
        }
    }
}

impl<SINT: Wait> Sensor<Ready, SINT> {
    /// Starts a radar measurement with a previously prepared configuration.
    ///
    /// This function initiates a radar measurement based on a configuration that must have been
    /// set up and prepared in advance. Ensure the sensor is powered on and calibration and
    /// preparation steps have been completed before calling this function.
    ///
    /// # Preconditions
    ///
    /// - The sensor must be powered on.
    /// - `calibrate` must have been successfully called.
    /// - `prepare` must have been successfully called.
    ///
    /// # Arguments
    ///
    /// * `sensor` - The sensor instance to use for measurement.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the measurement was successfully started, `Err(SensorError)` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use embedded_hal_async::digital::Wait;
    /// use rad_hard_sys::sensor::*;
    /// use rad_hard_sys::sensor::error::SensorError;
    ///  async fn foo<SINT: Wait>(sensor: &mut Sensor<Ready, SINT>) -> Result<(), SensorError> {
    /// sensor.measure().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn measure(&mut self) -> Result<(), SensorError> {
        // Implementation to start the radar measurement
        let success = unsafe { acc_sensor_measure(self.inner.deref_mut()) };
        if success {
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use embedded_hal_async::digital::Wait;
    /// use rad_hard_sys::sensor::*;
    /// use rad_hard_sys::sensor::data::RadarData;
    /// use rad_hard_sys::sensor::error::SensorError;
    ///  async fn foo<SINT: Wait>(sensor: &mut Sensor<Ready, SINT>) -> Result<(), SensorError> {
    /// let mut data_buffer = RadarData::default();
    /// sensor.read(&mut data_buffer).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn read(&self, buffer: &mut RadarData) -> Result<(), SensorError> {
        // Implementation to read the radar data
        let success = unsafe {
            acc_sensor_read(
                self.inner.deref(),
                buffer.data.as_mut_ptr() as *mut _,
                buffer.data.len() as u32,
            )
        };
        if success {
            Ok(())
        } else {
            Err(SensorError::ReadError)
        }
    }

    /// Calibrates the sensor asynchronously.
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
    /// was successful. The calibration might still be ongoing, requiring additional calls to
    /// `calibrate`. If the calibration step fails, returns `Err(SensorError::FailedCalibration)`.
    ///
    /// # Usage
    /// This function should be called in an asynchronous context and awaited.
    /// The caller should check the status of the `CalibrationResult`
    /// to determine if additional calibration steps are required.
    pub async fn recalibrate(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<CalibrationResult, SensorError> {
        // Start the calibration process
        let mut calibration_status: bool = false;
        let mut calibration_result = CalibrationResult::new();
        self.calibrated = false;
        unsafe {
            acc_sensor_calibrate(
                self.inner.deref_mut(),
                &mut calibration_status as *mut bool,
                calibration_result.mut_ptr(),
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
            );
        }

        // Wait for the interrupt to occur asynchronously
        self.interrupt
            .wait_for_low()
            .await
            .expect("Failed to wait for interrupt");

        self.calibrated = calibration_status;
        Ok(calibration_result)
    }

    pub async fn reprepare(
        &mut self,
        config: &RadarConfig,
        cal_result: &mut CalibrationResult,
        buffer: &mut [u8],
    ) -> Result<(), SensorError> {
        self.prepared = false;
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
            self.prepared = true;
            Ok(())
        } else {
            Err(SensorError::FailedPrepare)
        }
    }
}
