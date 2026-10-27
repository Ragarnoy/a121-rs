# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - Unreleased

### Changed
- Updated to a121-sys 0.8.0 (Acconeer A121 SDK v1.13.0). The SDK C API is unchanged since 1.12.
- The `distance` and `presence` features now enable the matching a121-sys features, so the detector
  libraries are only linked when requested. Previously both were always linked.
- The `std` feature now enables `a121-sys/std`.
- `embassy-sync/defmt` is only enabled by the `defmt` feature.
- **Breaking:** minimum supported Rust version is 1.99 (was 1.82). Variadic functions are stable since
  1.99, so the Rust logger no longer needs nightly.
- Updated embassy-sync to 0.8.
- The `nightly-logger` feature is deprecated in favour of `logger`, and is kept as an alias.
- docs.rs builds the crate against the a121-sys stub libraries. It could not build it before.
- **Breaking:** `RadarDistanceDetector::get_distance_buffer_size`, `get_static_result_buffer_size`
  and `PresenceDetector::get_buffer_size` return `Result<usize, SensorError>`. They no longer read
  uninitialized memory when the SDK fails to report a size.
- **Breaking:** `Hwaas::new` validates its argument and returns `Result<Hwaas, ConfigError>`.
- **Breaking:** `PeakSortingMethod::Amplitude` is renamed to `PeakSortingMethod::Closest`, matching
  the SDK sorting method it selects.
- **Breaking:** removed `impl From<PulseRepetitionFrequency> for u32`, which returned the frequency in
  Hz rather than the SDK enum value. Use `PulseRepetitionFrequency::value()` for the frequency.
- `RadarDistanceConfig::balanced()` uses a 0.5–2.1 m interval, matching the SDK
  example, instead of 15–17 m.
- `Radar::calibrate` leaves the radar in the `Enabled` state, since calibration power-cycles the
  sensor. A successful detector `prepare_detector` moves the radar to `Ready`.
- `PresenceConfig::set_profile` disables automatic profile selection, without which the SDK ignores
  the profile.
- **Breaking:** `Radar::config` and `Radar::processing` are no longer public fields. Use `config()`,
  `config_mut()` and `processing()`; `config_mut()` makes the radar leave the `Ready` state, since the
  sensor must be prepared again for a new configuration. `Radar::process` runs the processing.
- **Breaking:** `Radar::hibernate_on` and `hibernate_off` are `async`: they now disable and enable the
  sensor with the enable pin, as the SDK requires.
- **Breaking:** `Radar::reset_sensor`, `Processing::new` and `Processing::execute` return `Result`.
- **Breaking:** `Radar::measure`, `Processing::execute` and the checked detector functions
  (`calibrate_detector`, `update_calibration`, `prepare_detector`, `process_data`,
  `detect_presence`) require buffers aligned for any built-in type (see `RadarBuffer` and
  `AlignedBuffer`). `measure` also requires the buffer size of the prepared configuration or detector.
- **Breaking:** removed `PREPARE_BUFFER_SIZE`. The scratch buffer is checked against
  `RadarConfig::config_buffer_size()` at runtime instead.
- **Breaking:** `RadarDistanceConfig::balanced()` returns `Result`, and `RadarDistanceConfig` no longer
  implements `Default`. `PresenceConfig::new()` replaces `PresenceConfig::default()`. Both fail with
  `SensorError::InitFailed` instead of panicking when the SDK cannot allocate the configuration.
- **Breaking:** `ProcessingMetaData::subsweep_data_offset` and `subsweep_data_length` return
  `Option<usize>` instead of panicking on an out-of-range index.
- **Breaking:** `SensorError` and `ProcessDataError` have new variants (see Added), so exhaustive
  matches on them need new arms.
- **Breaking:** `Radar::check_status` and `RadarDistanceDetector::print_status` return
  `Result<(), SensorError>`, failing with `SpiTransferFailed` when the status read fails.
- `Radar::hibernate_on` leaves the radar unprepared if an SPI transfer fails, since the sensor may be
  half-hibernated. `Radar::reset_sensor` recovers it.
- The xe125 example links newlib's `sinf`, `cosf` and `tanf` instead of the compiler_builtins ones,
  which saves about 10 KB of flash, and logs at `info` level by default.
- **Breaking:** narrowed the public API to what users of the crate need. These are no longer public:
  the `hal` module (`AccHalImpl`, `RadarSpi`, `RefRadarSpi`; `Radar::new` sets up the HAL), the
  `libm` module (its `extern "C"` math functions are still linked), `RadarConfig::ptr`,
  `CalibrationResult::new` (use `Default`), `CalibrationInfo::mut_ptr`, `Processing::new` and `execute` (use `Radar::process`),
  `ProcessingMetaData::new` and `mut_ptr`, `RssVersion::new`, `MemoryRequirements::new`,
  `DistanceResult::new`, `PresenceResult::new` (use `Default`), `PresenceConfig::inner`, and the
  `radar` and `config` fields of `RadarDistanceDetector` and `PresenceDetector`. Removed the unused
  `ProcessingMetaData::ptr` and `PresenceMetadata::new`.

### Added
- `logger` feature: log SDK messages through a Rust variadic function (defmt) instead of the C logging
  bridge, on stable Rust. A small built-in printf formatter renders the SDK messages, so newlib's
  `vsnprintf` is not needed. Messages longer than 256 bytes are truncated, and floats are rendered
  with 7 significant digits.
- `stub_library` feature, which forwards to `a121-sys/stub_library` so the crate can be built and
  tested on the host without the Acconeer libraries.
- `SensorError`, `ProcessDataError`, `CalibrationResult` and `CalibrationInfo` are re-exported from the
  crate root. They appear in public signatures but were not nameable before.
- Host unit tests, run against the stub libraries.
- `RadarBuffer<N>` and `AlignedBuffer`: byte buffers aligned as the SDK requires.
- `SensorError::SpiTransferFailed`: an SPI transfer failed during the SDK call. The SDK's transfer
  callback cannot return an error, so the failure is recorded and checked after each SDK call that
  talks to the sensor, which then fails even if the SDK reported success.
- `config::Hwaas` and `config::ConfigError` are re-exported. They appear in public `RadarConfig`
  signatures but were not nameable before.
- `SensorError::BufferMisaligned`, `SensorError::EnablePinFailed` and
  `ProcessDataError::BufferMisaligned`.

### Fixed
- `libm` feature: `cabsf` and `cexpf` had the wrong signatures. They took a single `f32` instead of a
  `float complex`, so `cabsf` returned `|re|` instead of the magnitude.
- `Subsweep::set_prf` passed the frequency in Hz to the SDK instead of the PRF enum value.
- Undefined behaviour in the SPI transfer callback when the SPI device returns an error.
- `mem_alloc` returned pointers aligned to only 4 bytes on 32-bit targets. The SDK requires alignment
  for any built-in type: 8 bytes on Arm, 16 bytes on RISC-V.
- `RadarConfig::set_sweep_mode(SweepMode::Continuous { .. })` always failed because continuous mode
  was enabled before the sweep rate and frame rate were set.
- The compile-time check that `SCRATCH_SIZE >= 2560` in `Radar` was never evaluated.
- `Radar::prepare_sensor` creates the processing from the prepared configuration. It used to be created
  once from the default configuration, so its metadata did not match a reconfigured sensor.
- Hibernation never powered the sensor down: the enable pin was not toggled.
- The radar scratch buffer was not aligned for any built-in type, and preparation used a fixed
  2560-byte slice instead of the configuration's buffer size.
- `Processing::execute` did not check the buffer length.
- An enable pin or interrupt error panicked instead of returning an error, including in the distance
  detector's calibration.
- A failed detector prepare left the radar ready with its previous preparation.
- Creating a distance or presence detector panicked when the SDK returned NULL. It returns
  `SensorError::InitFailed`.
- `RadarConfig::hwaas()` and distance results could panic on unexpected SDK values.

- `Hwaas` accepted 0. The SDK range is 1–511.
- `RadarDistanceDetector::update_calibration` ignored SDK failures after the first step and could wait
  forever.
- Broken module-level doctest in `memory`.
- Docs: `RadarConfig::set_start_point` takes points, not millimetres, and PRF measurement time is
  inversely proportional to the PRF (corrected in SDK 1.13).
- Examples build against the current API (xe125, xe121; the esp32 examples are updated but unverified),
  use the new sensor calibration after recalibrating, and the xe125 allocator returns 8-byte aligned
  memory.
- CI: tests now run (they were gated on a matrix that did not exist), the example job checks the
  example instead of the library, and host jobs build against the stub libraries.

### Removed
- The unused 16-bit SPI transfer callback, which contained `todo!()`.
