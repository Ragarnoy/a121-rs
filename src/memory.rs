//! Memory sizing for radar sessions and detectors
//!
//! The SDK uses two kinds of memory:
//!
//! - **Buffers** the application allocates and passes in, such as sweep and calibration buffers.
//! - **RSS heap** the SDK allocates itself through the global allocator.
//!
//! How exact the numbers are depends on where they come from:
//!
//! | Memory | Source | Exactness |
//! |---|---|---|
//! | Detector buffers | `get_distance_buffer_size`, `get_static_result_buffer_size`, `PresenceDetector::get_buffer_size`, or `memory_requirements` on the detector | Exact, reported by the SDK |
//! | Session buffer | [`RadarConfig::config_buffer_size`](crate::config::RadarConfig::config_buffer_size) | Exact, reported by the SDK |
//! | Session memory | [`SessionMemoryCalculator`](crate::memory::SessionMemoryCalculator), [`calc_session_total`](crate::memory::calc_session_total) | Acconeer's memory model, which it states is within ±5% |
//! | Detector RSS heap | `heap-stats` feature, `sdk_heap_stats` | Measured at run time |
//!
//! The SDK has no query for the RSS heap of a detector, which depends on the configuration it
//! builds internally. To size the heap, enable the `heap-stats` feature and read
//! `sdk_heap_stats` after the detector has run a frame.
//!
//! # Session memory at compile time
//!
//! ```
//! use a121_rs::memory::{calc_session_external_heap, calc_session_rss_heap};
//!
//! const NUM_POINTS: u16 = 100;
//! const NUM_SUBSWEEPS: u8 = 1;
//! const SWEEPS_PER_FRAME: u16 = 16;
//!
//! const EXTERNAL_HEAP: usize = calc_session_external_heap(NUM_POINTS, NUM_SUBSWEEPS, SWEEPS_PER_FRAME);
//! const RSS_HEAP: usize = calc_session_rss_heap(NUM_SUBSWEEPS);
//! const TOTAL_MEMORY: usize = EXTERNAL_HEAP + RSS_HEAP;
//! ```
//!
//! Or with the macro:
//!
//! ```
//! use a121_rs::memory_for_session;
//!
//! const TOTAL_MEMORY: usize = memory_for_session!(
//!     num_points: 100,
//!     num_subsweeps: 1,
//!     sweeps_per_frame: 16
//! );
//! ```

use crate::config::RadarConfig;

// Constants from Acconeer's A121 memory model (exptool `a121/model/memory.py`)

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

// ============================================================================
// Compile-Time Memory Calculation Functions
// ============================================================================

/// Calculate external heap memory for a session at compile time
///
/// # Parameters
/// - `num_points_per_subsweep`: Number of points per subsweep. If the subsweeps differ, pass
///   their total divided by `num_subsweeps`, rounded up.
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
    external_heap_for_points(total_points)
}

/// External heap for a frame of `total_points` points, over all subsweeps and sweeps.
const fn external_heap_for_points(total_points: usize) -> usize {
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

// ============================================================================
// Runtime Memory Calculation
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
    fn new(external_heap: usize, rss_heap: usize) -> Self {
        Self {
            external_heap,
            rss_heap,
            total: external_heap + rss_heap,
        }
    }
}

/// Sum of the points of all subsweeps in `config`, for one sweep.
pub(crate) fn total_points(config: &RadarConfig) -> usize {
    (0..config.num_subsweep())
        .filter_map(|index| config.get_subsweep(index))
        .map(|subsweep| subsweep.num_points(config) as usize)
        .sum()
}

/// Session memory calculator for a [`Radar`](crate::radar::Radar) using a [`RadarConfig`]
/// directly, following Acconeer's memory model.
///
/// Detectors configure their own sessions; use their `memory_requirements` instead.
pub struct SessionMemoryCalculator<'a> {
    config: &'a RadarConfig,
}

impl<'a> SessionMemoryCalculator<'a> {
    /// Creates a new session memory calculator
    pub fn new(config: &'a RadarConfig) -> Self {
        Self { config }
    }

    /// Calculates external heap memory for a sweep operation
    pub fn external_heap(&self) -> usize {
        external_heap_for_points(
            total_points(self.config) * self.config.sweeps_per_frame() as usize,
        )
    }

    /// Calculates RSS heap memory for a sweep operation.
    ///
    /// This includes all RSS heap components: per-config, per-subsweep, and per-sensor overhead.
    /// Consistent with compile-time [`calc_session_rss_heap`].
    pub fn rss_heap(&self) -> usize {
        calc_session_rss_heap(self.config.num_subsweep())
    }

    /// Calculates total memory requirements
    pub fn memory_requirements(&self) -> MemoryRequirements {
        MemoryRequirements::new(self.external_heap(), self.rss_heap())
    }
}

/// Buffer sizes a detector needs, as reported by the SDK for its configuration.
///
/// The detector's RSS heap is not included: the SDK has no query for it. Measure it with the
/// `heap-stats` feature and `sdk_heap_stats`.
#[cfg(any(feature = "distance", feature = "presence"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DetectorMemory {
    /// Size of the buffer passed to calibration, prepare, measure and process calls (bytes)
    pub buffer: usize,
    /// Size of the static calibration result buffer (bytes). Zero for the presence detector.
    pub static_calibration: usize,
}

// ============================================================================
// SDK heap statistics
// ============================================================================

/// Memory the SDK has allocated through the HAL, counted when the `heap-stats` feature is on.
///
/// Sizes are those the SDK requested. Each allocation also costs a header of 8 bytes on Arm and
/// 16 bytes elsewhere, plus whatever overhead the global allocator adds.
#[cfg(feature = "heap-stats")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct HeapStats {
    /// Bytes allocated now
    pub current: usize,
    /// Most bytes allocated at once since start, or since [`reset_sdk_heap_peak`]
    pub peak: usize,
    /// Allocations alive now
    pub allocations: usize,
}

#[cfg(feature = "heap-stats")]
static HEAP_STATS: embassy_sync::blocking_mutex::Mutex<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    core::cell::Cell<HeapStats>,
> = embassy_sync::blocking_mutex::Mutex::new(core::cell::Cell::new(HeapStats {
    current: 0,
    peak: 0,
    allocations: 0,
}));

/// Returns the memory the SDK has allocated through the HAL.
#[cfg(feature = "heap-stats")]
pub fn sdk_heap_stats() -> HeapStats {
    HEAP_STATS.lock(core::cell::Cell::get)
}

/// Resets the peak to the memory allocated now, to measure the peak of a later phase.
#[cfg(feature = "heap-stats")]
pub fn reset_sdk_heap_peak() {
    update_heap_stats(|stats| stats.peak = stats.current);
}

#[cfg(feature = "heap-stats")]
fn update_heap_stats(f: impl FnOnce(&mut HeapStats)) {
    HEAP_STATS.lock(|cell| {
        let mut stats = cell.get();
        f(&mut stats);
        cell.set(stats);
    });
}

/// Counts an SDK allocation of `size` bytes.
#[cfg(feature = "heap-stats")]
pub(crate) fn record_alloc(size: usize) {
    update_heap_stats(|stats| {
        stats.current += size;
        stats.peak = stats.peak.max(stats.current);
        stats.allocations += 1;
    });
}

/// Counts the release of an SDK allocation of `size` bytes.
#[cfg(feature = "heap-stats")]
pub(crate) fn record_free(size: usize) {
    update_heap_stats(|stats| {
        stats.current = stats.current.saturating_sub(size);
        stats.allocations = stats.allocations.saturating_sub(1);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // Vectors computed with the formulas of exptool's `a121/model/memory.py`.

    #[test]
    fn small_frames_use_the_calibration_buffer() {
        assert_eq!(calc_session_external_heap(160, 1, 1), 2560);
        assert_eq!(calc_session_rss_heap(1), 1384);
    }

    #[test]
    fn large_frames_use_their_own_size() {
        assert_eq!(calc_session_external_heap(1990, 1, 1), 8028);
        assert_eq!(calc_session_total(1990, 1, 1), 8028 + 1384);
    }

    #[test]
    fn subsweeps_add_rss_heap() {
        // 3 subsweeps of 5, 20 and 10 points, 16 sweeps per frame: 560 points
        assert_eq!(external_heap_for_points((5 + 20 + 10) * 16), 2560);
        assert_eq!(calc_session_rss_heap(3), 1856);
        assert_eq!(
            memory_for_session!(num_points: 100, num_subsweeps: 2, sweeps_per_frame: 16),
            100 * 2 * 16 * 4 + 68 + 512 + 2 * 236 + 636
        );
    }
}
