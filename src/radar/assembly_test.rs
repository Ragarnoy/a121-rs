//! The SDK's assembly test, which checks that a sensor is mounted and connected correctly.

use core::ffi::{c_void, CStr};
use core::ptr::NonNull;

use a121_sys::*;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::SpiDevice;
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::digital::Wait;

use super::{check_buffer, Prepared, Radar, RadarState};
use crate::hal::spi_checked;
use crate::sensor::error::SensorError;

/// Smallest buffer [`Radar::assembly_test`] accepts.
pub const ASSEMBLY_TEST_MIN_BUFFER_SIZE: usize = ACC_RSS_ASSEMBLY_TEST_MIN_BUFFER_SIZE as usize;

/// One of the checks of the assembly test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AssemblyTest {
    /// SPI basic read
    BasicRead,
    /// SPI communication
    Communication,
    /// The enable pin
    EnablePin,
    /// The interrupt pin
    Interrupt,
    /// Clock and supply stability
    ClockAndSupply,
    /// Sensor calibration
    SensorCalibration,
}

impl AssemblyTest {
    /// Every test, to run them all.
    pub const ALL: [Self; 6] = [
        Self::BasicRead,
        Self::Communication,
        Self::EnablePin,
        Self::Interrupt,
        Self::ClockAndSupply,
        Self::SensorCalibration,
    ];

    fn ffi(self) -> acc_rss_assembly_test_test_id_t {
        match self {
            Self::BasicRead => acc_rss_assembly_test_test_id_t_ACC_RSS_ASSEMBLY_TEST_ID_BASIC_READ,
            Self::Communication => {
                acc_rss_assembly_test_test_id_t_ACC_RSS_ASSEMBLY_TEST_ID_COMMUNICATION
            }
            Self::EnablePin => acc_rss_assembly_test_test_id_t_ACC_RSS_ASSEMBLY_TEST_ID_ENABLE_PIN,
            Self::Interrupt => acc_rss_assembly_test_test_id_t_ACC_RSS_ASSEMBLY_TEST_ID_INTERRUPT,
            Self::ClockAndSupply => {
                acc_rss_assembly_test_test_id_t_ACC_RSS_ASSEMBLY_TEST_ID_CLOCK_AND_SUPPLY
            }
            Self::SensorCalibration => {
                acc_rss_assembly_test_test_id_t_ACC_RSS_ASSEMBLY_TEST_ID_SENSOR_CALIBRATION
            }
        }
    }
}

/// The outcome of one test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AssemblyTestOutcome {
    /// The SDK's name for the test, such as `"Basic read test"`
    pub name: &'static str,
    /// Whether the test passed
    pub passed: bool,
}

/// The outcomes of the tests [`Radar::assembly_test`] ran.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AssemblyTestReport {
    outcomes: [AssemblyTestOutcome; AssemblyTest::ALL.len()],
    len: usize,
}

impl AssemblyTestReport {
    /// The outcome of each test that ran, in the SDK's order.
    pub fn outcomes(&self) -> &[AssemblyTestOutcome] {
        &self.outcomes[..self.len]
    }

    /// Whether every test that ran passed.
    pub fn passed(&self) -> bool {
        self.outcomes().iter().all(|outcome| outcome.passed)
    }
}

/// Destroys the SDK's test instance when dropped, including when the test's future is.
struct Instance(NonNull<acc_rss_assembly_test_t>);

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe { acc_rss_assembly_test_destroy(self.0.as_ptr()) }
    }
}

/// Enables the SDK's diagnostic logs for the assembly tests that run afterwards.
pub fn enable_assembly_test_diagnostic_logs() {
    unsafe { acc_rss_assembly_test_enable_diagnostic_logs() }
}

impl<SPI, SINT, ENABLE, DLY, const SCRATCH_SIZE: usize> Radar<SPI, SINT, ENABLE, DLY, SCRATCH_SIZE>
where
    SPI: SpiDevice<u8> + 'static,
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    /// Runs the SDK's assembly test, which checks the sensor's SPI, enable pin, interrupt pin,
    /// clock and supply, and calibration, for example at the end of a production line.
    ///
    /// Runs the `tests` given, such as [`AssemblyTest::ALL`]. `buffer` is the test's working
    /// memory, of at least [`ASSEMBLY_TEST_MIN_BUFFER_SIZE`] bytes.
    ///
    /// The sensor is power-cycled before and after the test, so the radar is enabled and
    /// unprepared afterwards, as after [`Radar::reset_sensor`]: calibrate it again before
    /// measuring. The interrupt pin has no timeout, so a sensor whose interrupt never rises keeps
    /// the test waiting; an error from the pin is reported to the test as a timeout.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::NotReady`] if the radar is hibernating,
    /// [`SensorError::BufferTooSmall`] or [`SensorError::BufferMisaligned`] for an unsuitable
    /// `buffer`, and [`SensorError::InitFailed`] if the SDK could not create the test. A failed
    /// test is not an error: see [`AssemblyTestReport::passed`].
    ///
    /// If the test fails to run, for example on an SPI or enable pin failure, or the future is
    /// dropped before it completes, the radar is [faulted](RadarState::Faulted).
    pub async fn assembly_test(
        &mut self,
        tests: &[AssemblyTest],
        buffer: &mut [u8],
    ) -> Result<AssemblyTestReport, SensorError> {
        if self.state == RadarState::Hibernating {
            return Err(SensorError::NotReady);
        }
        check_buffer(buffer, ASSEMBLY_TEST_MIN_BUFFER_SIZE)?;
        // The SDK wants a multiple of 8 bytes
        let buffer_len = (buffer.len() & !7).min(u32::MAX as usize & !7);

        self.prepared = Prepared::None;
        self.begin_busy();
        let reset = self.sensor.reset_sensor().await;
        self.fault_on_error(reset)?;

        let id = self.id();
        let created = unsafe {
            acc_rss_assembly_test_create(id, buffer.as_mut_ptr() as *mut c_void, buffer_len as u32)
        };
        let Some(created) = NonNull::new(created) else {
            // Nothing ran since the power cycle
            self.state = RadarState::Enabled;
            return Err(SensorError::InitFailed);
        };
        let instance = Instance(created);
        unsafe {
            acc_rss_assembly_test_disable_all_tests(instance.0.as_ptr());
            for test in tests {
                acc_rss_assembly_test_enable(instance.0.as_ptr(), test.ffi());
            }
        }

        let mut status = acc_rss_test_integration_status_t_ACC_RSS_TEST_INTEGRATION_STATUS_OK;
        loop {
            let state = spi_checked(id, || unsafe {
                acc_rss_assembly_test_execute(instance.0.as_ptr(), status)
            });
            let state = self.fault_on_error(state)?;
            status = acc_rss_test_integration_status_t_ACC_RSS_TEST_INTEGRATION_STATUS_OK;
            #[allow(non_upper_case_globals)]
            match state {
                acc_rss_test_state_t_ACC_RSS_TEST_STATE_ONGOING => {}
                acc_rss_test_state_t_ACC_RSS_TEST_STATE_TOGGLE_ENABLE_PIN => {
                    let toggled = self.sensor.reset_sensor().await;
                    self.fault_on_error(toggled)?;
                }
                acc_rss_test_state_t_ACC_RSS_TEST_STATE_WAIT_FOR_INTERRUPT => {
                    if self.interrupt.wait_for_high().await.is_err() {
                        status =
                            acc_rss_test_integration_status_t_ACC_RSS_TEST_INTEGRATION_STATUS_TIMEOUT;
                    }
                }
                acc_rss_test_state_t_ACC_RSS_TEST_STATE_COMPLETE => break,
                _ => return self.fault_on_error(Err(SensorError::ProcessingFailed)),
            }
        }

        let report = report(&instance);
        drop(instance);
        // The SDK asks for a power cycle after the test
        let reset = self.sensor.reset_sensor().await;
        self.fault_on_error(reset)?;
        self.state = RadarState::Enabled;
        Ok(report)
    }
}

/// Copies the outcomes out of the test instance.
fn report(instance: &Instance) -> AssemblyTestReport {
    let mut report = AssemblyTestReport {
        outcomes: [AssemblyTestOutcome {
            name: "",
            passed: false,
        }; AssemblyTest::ALL.len()],
        len: 0,
    };
    let mut count = 0u16;
    let results = unsafe { acc_rss_assembly_test_get_results(instance.0.as_ptr(), &mut count) };
    if results.is_null() {
        return report;
    }
    report.len = (count as usize).min(report.outcomes.len());
    // SAFETY: the SDK returns `count` results, which live as long as the instance
    let results = unsafe { core::slice::from_raw_parts(results, report.len) };
    for (outcome, result) in report.outcomes.iter_mut().zip(results) {
        outcome.passed = result.test_result;
        if !result.test_name.is_null() {
            // SAFETY: the names are NUL-terminated string literals in the SDK, which live as long
            // as the program
            outcome.name = unsafe { CStr::from_ptr(result.test_name) }
                .to_str()
                .unwrap_or("");
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tests_map_to_the_sdk_ids_in_order() {
        for (index, test) in AssemblyTest::ALL.into_iter().enumerate() {
            assert_eq!(test.ffi() as usize, index);
        }
    }
}
