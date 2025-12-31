//! Memory Calculation Module
//!
//! This module provides utilities to calculate memory requirements for radar operations
//! both at runtime and compile-time. This allows embedded systems to pre-validate memory
//! availability and allocate appropriately-sized buffers.
//!
//! The calculations are based on the Acconeer A121 SDK memory model and match the reference
//! implementation from the Python exploration tool.
//!
//! # Memory Types
//!
//! The radar system uses two types of heap memory:
//!
//! - **External Heap**: Used for large data buffers (sweep data, calibration buffers)
//! - **RSS Heap**: Used by the Radar System Software for configuration and processing
//!
//! # Runtime Usage
//!
//! ```no_run
//! use a121_rs::memory::DistanceMemoryCalculator;
//! use a121_rs::detector::distance::config::RadarDistanceConfig;
//!
//! let config = RadarDistanceConfig::default();
//! let calc = DistanceMemoryCalculator::new(&config);
//!
//! // Check total memory requirement before allocating
//! let total_memory = calc.total_memory();
//! println!("Distance detector requires {} bytes", total_memory);
//!
//! // Allocate buffers with correct sizes
//! let mut buffer = vec![0u8; calc.buffer_size()];
//! let mut static_cal = vec![0u8; calc.static_calibration_size()];
//! ```
//!
//! # Compile-Time Usage
//!
//! For static configurations known at compile time, use const functions:
//!
//! ```
//! use a121_rs::memory::{calc_session_external_heap, calc_session_rss_heap};
//!
//! // Configuration parameters known at compile time
//! const NUM_POINTS: u16 = 100;
//! const NUM_SUBSWEEPS: u8 = 1;
//! const SWEEPS_PER_FRAME: u16 = 16;
//!
//! // Calculate at compile time
//! const EXTERNAL_HEAP: usize = calc_session_external_heap(NUM_POINTS, NUM_SUBSWEEPS, SWEEPS_PER_FRAME);
//! const RSS_HEAP: usize = calc_session_rss_heap(NUM_SUBSWEEPS);
//! const TOTAL_MEMORY: usize = EXTERNAL_HEAP + RSS_HEAP;
//!
//! // Use in static buffer allocation
//! static mut BUFFER: [u8; EXTERNAL_HEAP] = [0; EXTERNAL_HEAP];
//! ```
//!
//! Or use the convenience macro:
//!
//! ```
//! use a121_rs::memory_for_session;
//!
//! // Calculate total memory at compile time
//! const TOTAL_MEMORY: usize = memory_for_session!(
//!     num_points: 100,
//!     num_subsweeps: 1,
//!     sweeps_per_frame: 16
//! );
//! ```

use crate::config::RadarConfig;

// Constants from Acconeer A121 SDK memory model
// These match the reference Python implementation

/// Overhead for external heap allocations
const OVERHEAD: usize = 68;

/// Size of calibration buffer
const CALIB_BUFFER: usize = 2492;

/// Bytes per measurement point
const BYTES_PER_POINT: usize = 4;

/// RSS heap memory per subsweep
const RSS_HEAP_PER_SUBSWEEP: usize = 236;

/// RSS heap memory per sensor
const RSS_HEAP_PER_SENSOR: usize = 636;

/// RSS heap memory per configuration
const RSS_HEAP_PER_CONFIG: usize = 512;

/// Size of float in bytes
const SIZE_OF_FLOAT: usize = 4;

/// Presence detector heap overhead
const PRESENCE_HEAP_OVERHEAD: usize = 256;

/// Distance detector heap overhead
const DISTANCE_HEAP_OVERHEAD: usize = 1028;

/// Distance detector heap per processor
const DISTANCE_HEAP_PER_PROCESSOR: usize = 224;

/// Padding length for filtfilt filtering operations (distance detector)
const FILTFILT_PAD_LEN: usize = 9;

/// Number of filter parameters per point for presence detection
const PRESENCE_FILTER_PARAMS: usize = 7;

/// Minimum static calibration buffer size for distance detection
const DISTANCE_MIN_STATIC_CAL_SIZE: usize = 2048;

// ============================================================================
// Compile-Time Memory Calculation Functions
// ============================================================================
//
// These const fn functions can be evaluated at compile time, allowing static
// buffer allocation in embedded systems.

/// Calculate external heap memory for a session at compile time
///
/// # Parameters
/// - `num_points_per_subsweep`: Number of points per subsweep
/// - `num_subsweeps`: Number of subsweeps
/// - `sweeps_per_frame`: Number of sweeps per frame
///
/// # Example
/// ```
/// use a121_rs::memory::calc_session_external_heap;
/// const BUFFER_SIZE: usize = calc_session_external_heap(100, 1, 16);
/// static mut BUFFER: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
/// ```
pub const fn calc_session_external_heap(
    num_points_per_subsweep: u16,
    num_subsweeps: u8,
    sweeps_per_frame: u16,
) -> usize {
    let total_points =
        num_points_per_subsweep as usize * num_subsweeps as usize * sweeps_per_frame as usize;
    let buffer_size = total_points * BYTES_PER_POINT;

    let base = if buffer_size > CALIB_BUFFER {
        buffer_size
    } else {
        CALIB_BUFFER
    };

    base + OVERHEAD
}

/// Calculate RSS heap memory for a session at compile time
///
/// # Parameters
/// - `num_subsweeps`: Number of subsweeps
///
/// # Example
/// ```
/// use a121_rs::memory::calc_session_rss_heap;
/// const RSS_MEMORY: usize = calc_session_rss_heap(1);
/// ```
pub const fn calc_session_rss_heap(num_subsweeps: u8) -> usize {
    RSS_HEAP_PER_CONFIG + (num_subsweeps as usize * RSS_HEAP_PER_SUBSWEEP) + RSS_HEAP_PER_SENSOR
}

/// Calculate total session memory at compile time
///
/// # Parameters
/// - `num_points_per_subsweep`: Number of points per subsweep
/// - `num_subsweeps`: Number of subsweeps
/// - `sweeps_per_frame`: Number of sweeps per frame
///
/// # Example
/// ```
/// use a121_rs::memory::calc_session_total;
/// const TOTAL_MEMORY: usize = calc_session_total(100, 1, 16);
/// ```
pub const fn calc_session_total(
    num_points_per_subsweep: u16,
    num_subsweeps: u8,
    sweeps_per_frame: u16,
) -> usize {
    calc_session_external_heap(num_points_per_subsweep, num_subsweeps, sweeps_per_frame)
        + calc_session_rss_heap(num_subsweeps)
}

/// Calculate external heap memory for presence detection at compile time
///
/// # Parameters
/// - `num_points`: Total number of points across all subsweeps
/// - `num_subsweeps`: Number of subsweeps
/// - `sweeps_per_frame`: Number of sweeps per frame
///
/// # Example
/// ```
/// use a121_rs::memory::calc_presence_external_heap;
/// const BUFFER_SIZE: usize = calc_presence_external_heap(100, 1, 16);
/// ```
pub const fn calc_presence_external_heap(
    num_points: u16,
    num_subsweeps: u8,
    sweeps_per_frame: u16,
) -> usize {
    let session_ext = calc_session_external_heap(num_points, num_subsweeps, sweeps_per_frame);
    let presence_ext = num_points as usize * 2 * SIZE_OF_FLOAT;
    session_ext + presence_ext
}

/// Calculate RSS heap memory for presence detection at compile time
///
/// # Parameters
/// - `num_points`: Total number of points across all subsweeps
/// - `num_subsweeps`: Number of subsweeps
///
/// # Example
/// ```
/// use a121_rs::memory::calc_presence_rss_heap;
/// const RSS_MEMORY: usize = calc_presence_rss_heap(100, 1);
/// ```
pub const fn calc_presence_rss_heap(num_points: u16, num_subsweeps: u8) -> usize {
    let session_rss = RSS_HEAP_PER_CONFIG + (num_subsweeps as usize * RSS_HEAP_PER_SUBSWEEP);
    let presence_rss = num_points as usize * PRESENCE_FILTER_PARAMS * SIZE_OF_FLOAT;
    PRESENCE_HEAP_OVERHEAD + presence_rss + RSS_HEAP_PER_SENSOR + session_rss
}

/// Calculate total presence detection memory at compile time
///
/// # Parameters
/// - `num_points`: Total number of points across all subsweeps
/// - `num_subsweeps`: Number of subsweeps
/// - `sweeps_per_frame`: Number of sweeps per frame
///
/// # Example
/// ```
/// use a121_rs::memory::calc_presence_total;
/// const TOTAL_MEMORY: usize = calc_presence_total(100, 1, 16);
/// ```
pub const fn calc_presence_total(
    num_points: u16,
    num_subsweeps: u8,
    sweeps_per_frame: u16,
) -> usize {
    calc_presence_external_heap(num_points, num_subsweeps, sweeps_per_frame)
        + calc_presence_rss_heap(num_points, num_subsweeps)
}

/// Calculate external heap memory for distance detection at compile time
///
/// This is a conservative estimate for distance detection.
///
/// # Parameters
/// - `num_points`: Total number of points across all subsweeps
/// - `num_subsweeps`: Number of subsweeps
/// - `sweeps_per_frame`: Number of sweeps per frame
///
/// # Example
/// ```
/// use a121_rs::memory::calc_distance_external_heap;
/// const BUFFER_SIZE: usize = calc_distance_external_heap(100, 1, 16);
/// ```
pub const fn calc_distance_external_heap(
    num_points: u16,
    num_subsweeps: u8,
    sweeps_per_frame: u16,
) -> usize {
    let session_ext = calc_session_external_heap(num_points, num_subsweeps, sweeps_per_frame);

    // Work buffer for filtering (with padding)
    let work_buffer = (num_points as usize + 2 * FILTFILT_PAD_LEN) * 2 * SIZE_OF_FLOAT;

    // Calibration buffers (conservative estimate)
    let calib_buffer = num_points as usize * SIZE_OF_FLOAT * 3;

    // Additional buffer for close range if using multiple sweeps
    let close_range_buffer = if sweeps_per_frame > 1 {
        sweeps_per_frame as usize * num_points as usize * SIZE_OF_FLOAT
    } else {
        0
    };

    session_ext + work_buffer + calib_buffer + close_range_buffer
}

/// Calculate RSS heap memory for distance detection at compile time
///
/// # Parameters
/// - `num_subsweeps`: Number of subsweeps
///
/// # Example
/// ```
/// use a121_rs::memory::calc_distance_rss_heap;
/// const RSS_MEMORY: usize = calc_distance_rss_heap(1);
/// ```
pub const fn calc_distance_rss_heap(num_subsweeps: u8) -> usize {
    let session_rss = RSS_HEAP_PER_CONFIG + (num_subsweeps as usize * RSS_HEAP_PER_SUBSWEEP);
    // Conservative estimate: assume 2 processors
    let processor_heap = DISTANCE_HEAP_PER_PROCESSOR * 2;
    DISTANCE_HEAP_OVERHEAD + processor_heap + RSS_HEAP_PER_SENSOR + session_rss
}

/// Calculate total distance detection memory at compile time
///
/// # Parameters
/// - `num_points`: Total number of points across all subsweeps
/// - `num_subsweeps`: Number of subsweeps
/// - `sweeps_per_frame`: Number of sweeps per frame
///
/// # Example
/// ```
/// use a121_rs::memory::calc_distance_total;
/// const TOTAL_MEMORY: usize = calc_distance_total(100, 1, 16);
/// ```
pub const fn calc_distance_total(
    num_points: u16,
    num_subsweeps: u8,
    sweeps_per_frame: u16,
) -> usize {
    calc_distance_external_heap(num_points, num_subsweeps, sweeps_per_frame)
        + calc_distance_rss_heap(num_subsweeps)
}

/// Calculate static calibration buffer size for distance detection at compile time
///
/// # Parameters
/// - `num_points`: Total number of points across all subsweeps
///
/// # Example
/// ```
/// use a121_rs::memory::calc_distance_static_cal_size;
/// const CAL_BUFFER_SIZE: usize = calc_distance_static_cal_size(100);
/// static mut CAL_BUFFER: [u8; CAL_BUFFER_SIZE] = [0; CAL_BUFFER_SIZE];
/// ```
pub const fn calc_distance_static_cal_size(num_points: u16) -> usize {
    let calculated = num_points as usize * SIZE_OF_FLOAT * 2;
    if calculated > DISTANCE_MIN_STATIC_CAL_SIZE {
        calculated
    } else {
        DISTANCE_MIN_STATIC_CAL_SIZE
    }
}

// ============================================================================
// Compile-Time Convenience Macros
// ============================================================================

/// Calculate total session memory at compile time (macro version)
///
/// # Example
/// ```
/// use a121_rs::memory_for_session;
/// const MEMORY: usize = memory_for_session!(
///     num_points: 100,
///     num_subsweeps: 1,
///     sweeps_per_frame: 16
/// );
/// ```
#[macro_export]
macro_rules! memory_for_session {
    (num_points: $points:expr, num_subsweeps: $subsweeps:expr, sweeps_per_frame: $sweeps:expr) => {
        $crate::memory::calc_session_total($points, $subsweeps, $sweeps)
    };
}

/// Calculate total presence detection memory at compile time (macro version)
///
/// # Example
/// ```
/// use a121_rs::memory_for_presence;
/// const MEMORY: usize = memory_for_presence!(
///     num_points: 100,
///     num_subsweeps: 1,
///     sweeps_per_frame: 16
/// );
/// ```
#[macro_export]
macro_rules! memory_for_presence {
    (num_points: $points:expr, num_subsweeps: $subsweeps:expr, sweeps_per_frame: $sweeps:expr) => {
        $crate::memory::calc_presence_total($points, $subsweeps, $sweeps)
    };
}

/// Calculate total distance detection memory at compile time (macro version)
///
/// # Example
/// ```
/// use a121_rs::memory_for_distance;
/// const MEMORY: usize = memory_for_distance!(
///     num_points: 100,
///     num_subsweeps: 1,
///     sweeps_per_frame: 16
/// );
/// ```
#[macro_export]
macro_rules! memory_for_distance {
    (num_points: $points:expr, num_subsweeps: $subsweeps:expr, sweeps_per_frame: $sweeps:expr) => {
        $crate::memory::calc_distance_total($points, $subsweeps, $sweeps)
    };
}

// ============================================================================
// Runtime Memory Calculation (original implementation)
// ============================================================================

/// Memory breakdown for a radar configuration
#[derive(Debug, Clone, Copy)]
pub struct MemoryRequirements {
    /// External heap memory required (bytes)
    pub external_heap: usize,
    /// RSS heap memory required (bytes)
    pub rss_heap: usize,
    /// Total memory required (bytes)
    pub total: usize,
}

impl MemoryRequirements {
    /// Creates a new MemoryRequirements instance
    pub fn new(external_heap: usize, rss_heap: usize) -> Self {
        Self {
            external_heap,
            rss_heap,
            total: external_heap + rss_heap,
        }
    }
}

/// Base radar session memory calculator
pub struct SessionMemoryCalculator<'a> {
    config: &'a RadarConfig,
}

impl<'a> SessionMemoryCalculator<'a> {
    /// Creates a new session memory calculator
    pub fn new(config: &'a RadarConfig) -> Self {
        Self { config }
    }

    /// Calculates total number of points across all subsweeps and frames
    fn total_num_points(&self) -> usize {
        let num_subsweeps = self.config.num_subsweep() as usize;
        let sweeps_per_frame = self.config.sweeps_per_frame() as usize;

        let mut total_points = 0;
        for i in 0..num_subsweeps {
            let subsweep = self.config.get_subsweep(i as u8).unwrap();
            total_points += subsweep.num_points(self.config) as usize;
        }

        total_points * sweeps_per_frame
    }

    /// Calculates external heap memory for a sweep operation
    pub fn external_heap(&self) -> usize {
        let total_points = self.total_num_points();
        let buffer_size = total_points * BYTES_PER_POINT;
        buffer_size.max(CALIB_BUFFER) + OVERHEAD
    }

    /// Calculates RSS heap memory for a sweep operation.
    ///
    /// This includes all RSS heap components: per-config, per-subsweep, and per-sensor overhead.
    /// Consistent with compile-time `calc_session_rss_heap()`.
    pub fn rss_heap(&self) -> usize {
        let num_subsweeps = self.config.num_subsweep() as usize;
        RSS_HEAP_PER_CONFIG + num_subsweeps * RSS_HEAP_PER_SUBSWEEP + RSS_HEAP_PER_SENSOR
    }

    /// Calculates total memory requirements
    pub fn memory_requirements(&self) -> MemoryRequirements {
        MemoryRequirements::new(self.external_heap(), self.rss_heap())
    }
}

/// Presence detector memory calculator
pub struct PresenceMemoryCalculator<'a> {
    config: &'a RadarConfig,
}

impl<'a> PresenceMemoryCalculator<'a> {
    /// Creates a new presence memory calculator from a radar configuration
    ///
    /// Note: This assumes the RadarConfig has been configured by the presence detector.
    /// For accurate results, use this after calling presence detector configuration methods.
    pub fn new(config: &'a RadarConfig) -> Self {
        Self { config }
    }

    /// Calculates total number of points for presence detection
    fn total_num_points(&self) -> usize {
        let num_subsweeps = self.config.num_subsweep() as usize;
        let mut total_points = 0;
        for i in 0..num_subsweeps {
            let subsweep = self.config.get_subsweep(i as u8).unwrap();
            total_points += subsweep.num_points(self.config) as usize;
        }
        total_points
    }

    /// Calculates external heap memory for presence detection
    pub fn external_heap(&self) -> usize {
        let session_calc = SessionMemoryCalculator::new(self.config);
        let session_ext = session_calc.external_heap();

        let num_points = self.total_num_points();
        let presence_ext = num_points * 2 * SIZE_OF_FLOAT;

        session_ext + presence_ext
    }

    /// Calculates RSS heap memory for presence detection
    pub fn rss_heap(&self) -> usize {
        let session_calc = SessionMemoryCalculator::new(self.config);
        let sweep_rss = session_calc.rss_heap();

        let num_points = self.total_num_points();
        let presence_rss = num_points * PRESENCE_FILTER_PARAMS * SIZE_OF_FLOAT;

        PRESENCE_HEAP_OVERHEAD + presence_rss + sweep_rss
    }

    /// Calculates total memory requirements for presence detection
    pub fn memory_requirements(&self) -> MemoryRequirements {
        MemoryRequirements::new(self.external_heap(), self.rss_heap())
    }

    /// Returns the buffer size needed for presence detection operations
    ///
    /// This is the size you should allocate for the buffer passed to
    /// `prepare_detector()` and `detect_presence()` methods.
    pub fn buffer_size(&self) -> usize {
        // Buffer size is primarily the external heap requirement
        // This is a conservative estimate
        self.external_heap()
    }
}

/// Distance detector memory calculator
///
/// Note: This provides conservative estimates for distance detection memory requirements.
/// The actual memory usage depends on measurement type, threshold method, and other
/// configuration parameters that may not be fully accessible through the Rust API.
pub struct DistanceMemoryCalculator<'a> {
    config: &'a RadarConfig,
}

impl<'a> DistanceMemoryCalculator<'a> {
    /// Creates a new distance memory calculator from a radar configuration
    ///
    /// Note: This assumes the RadarConfig has been configured by the distance detector.
    /// For accurate results, use this after calling distance detector configuration methods.
    pub fn new(config: &'a RadarConfig) -> Self {
        Self { config }
    }

    /// Calculates total number of points for distance detection
    fn total_num_points(&self) -> usize {
        let num_subsweeps = self.config.num_subsweep() as usize;
        let mut total_points = 0;
        for i in 0..num_subsweeps {
            let subsweep = self.config.get_subsweep(i as u8).unwrap();
            total_points += subsweep.num_points(self.config) as usize;
        }
        total_points
    }

    /// Calculates external heap memory for distance detection
    ///
    /// This is a simplified calculation that provides a conservative estimate.
    /// The actual implementation in the SDK considers measurement types,
    /// threshold methods, and processor configurations.
    pub fn external_heap(&self) -> usize {
        let session_calc = SessionMemoryCalculator::new(self.config);
        let session_ext = session_calc.external_heap();

        // Conservative estimate for distance processing buffers
        // Includes work buffers, calibration buffers, and noise buffers
        let num_points = self.total_num_points();
        let sweeps_per_frame = self.config.sweeps_per_frame() as usize;

        // Work buffer for filtering (with padding)
        let work_buffer = (num_points + 2 * FILTFILT_PAD_LEN) * 2 * SIZE_OF_FLOAT;

        // Calibration buffers (conservative estimate)
        // In practice this depends on threshold method
        let calib_buffer = num_points * SIZE_OF_FLOAT * 3;

        // Additional buffer for close range if using multiple sweeps
        let close_range_buffer = if sweeps_per_frame > 1 {
            sweeps_per_frame * num_points * SIZE_OF_FLOAT
        } else {
            0
        };

        session_ext + work_buffer + calib_buffer + close_range_buffer
    }

    /// Calculates RSS heap memory for distance detection
    pub fn rss_heap(&self) -> usize {
        let session_calc = SessionMemoryCalculator::new(self.config);
        let sweep_rss = session_calc.rss_heap();

        // Conservative estimate: assume 2 processors
        let processor_heap = DISTANCE_HEAP_PER_PROCESSOR * 2;

        DISTANCE_HEAP_OVERHEAD + processor_heap + sweep_rss
    }

    /// Calculates total memory requirements for distance detection
    pub fn memory_requirements(&self) -> MemoryRequirements {
        MemoryRequirements::new(self.external_heap(), self.rss_heap())
    }

    /// Returns the buffer size needed for distance detection operations
    ///
    /// This is the size you should allocate for the buffer passed to
    /// `calibrate_detector()` and processing methods.
    pub fn buffer_size(&self) -> usize {
        // Conservative estimate for main processing buffer
        self.external_heap()
    }

    /// Returns the static calibration result buffer size
    ///
    /// This is the size you should allocate for the static calibration buffer.
    pub fn static_calibration_size(&self) -> usize {
        let num_points = self.total_num_points();
        (num_points * SIZE_OF_FLOAT * 2).max(DISTANCE_MIN_STATIC_CAL_SIZE)
    }
}
