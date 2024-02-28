// Copyright (c) Acconeer AB, 2020-2023
// All rights reserved

#ifndef ACC_CONFIG_H_
#define ACC_CONFIG_H_

#include <stdbool.h>
#include <stdint.h>

#include "acc_definitions_a121.h"
#include "acc_definitions_common.h"


/**
 * @defgroup config Config
 * @ingroup service
 *
 * @brief Module to configure sensor and processing
 *
 * @{
 */


struct acc_config;

typedef struct acc_config acc_config_t;


/**
 * @brief Create a configuration
 *
 * A configuration is created and populated with default values.
 *
 * @return A configuration instance
 */
acc_config_t *acc_config_create(void);


/**
 * @brief Destroy a configuration freeing any resources allocated
 *
 * Destroy a configuration that is no longer needed.
 *
 * @param[in] config The configuration to destroy, can be NULL
 */
void acc_config_destroy(acc_config_t *config);


/**
 * @brief Print a configuration to the log
 *
 * @param[in] config The configuration to log
 */
void acc_config_log(const acc_config_t *config);


/**
 * @brief Set the starting point of the sweep
 *
 * This sets the starting point of the sweep. The corresponding start
 * in millimeter is approximately start_point * 2.5 mm. For the exact
 * distance in meter, use the @ref acc_processing_points_to_meter function.
 *
 * @param[in] config The configuration
 * @param[in] start_point The starting point of the sweep
 */
void acc_config_start_point_set(acc_config_t *config, int32_t start_point);


/**
 * @brief Get the starting point of the sweep
 *
 * @see acc_config_start_point_set
 *
 * @param[in] config The configuration
 * @return The starting point of the sweep
 */
int32_t acc_config_start_point_get(const acc_config_t *config);


/**
 * @brief Set the number of data points to measure
 *
 * This sets the number of data points to measure in a sweep.
 *
 * @param[in] config The configuration
 * @param[in] num_points Number of data points to measure
 */
void acc_config_num_points_set(acc_config_t *config, uint16_t num_points);


/**
 * @brief Get the number of data points to measure
 *
 * @see acc_config_num_points_set
 *
 * @param[in] config The configuration
 * @return Number of data points to measure
 */
uint16_t acc_config_num_points_get(const acc_config_t *config);


/**
 * @brief Set the step length in a sweep
 *
 * This sets the number of steps to have between each data point.
 *
 * Sampling produces complex (IQ) data points with configurable distance spacing,
 * starting from ~2.5mm.
 *
 * @param[in] config The configuration
 * @param[in] step_length The step length
 */
void acc_config_step_length_set(acc_config_t *config, uint16_t step_length);


/**
 * @brief Get the step length in a sweep
 *
 * @see acc_config_step_length_set
 *
 * @param[in] config The configuration
 * @return The step length
 */
uint16_t acc_config_step_length_get(const acc_config_t *config);


/**
 * @brief Set a profile
 *
 * Each profile consists of a number of settings for the sensor that configures
 * the RX and TX paths. Lower profiles have higher depth resolution while
 * higher profiles have higher SNR.
 *
 * @param[in] config The config to set a profile for
 * @param[in] profile The profile to set
 */
void acc_config_profile_set(acc_config_t         *config,
                            acc_config_profile_t profile);


/**
 * @brief Get the currently used profile
 *
 * See @ref acc_config_profile_set
 *
 * @param[in] config The config to get a profile for
 * @return The profile currently used
 */
acc_config_profile_t acc_config_profile_get(const acc_config_t *config);


/**
 * @brief Set the hardware accelerated average samples (HWAAS)
 *
 * Each data point can be sampled several times and the sensor hardware then
 * produces an average value of those samples. The time needed to measure a sweep is roughly proportional
 * to the number of averaged samples. Hence, if there is a need to obtain a higher update rate, HWAAS
 * could be decreased but this leads to lower SNR.
 *
 * HWAAS must be between 1 and 511 inclusive
 *
 * @param[in] config The config to set HWAAS for
 * @param[in] hwaas Hardware accelerated average samples
 */
void acc_config_hwaas_set(acc_config_t *config, uint16_t hwaas);


/**
 * @brief Get the hardware accelerated average samples (HWAAS)
 *
 * @see acc_config_hwaas_set
 *
 * @param[in] config The config to get HWAAS from
 * @return Hardware accelerated average samples
 */
uint16_t acc_config_hwaas_get(const acc_config_t *config);


/**
 * @brief Set receiver gain setting
 *
 * Must be a value between 0 and 23 inclusive where 23 is the highest gain and 0 the lowest.
 *
 * Lower gain gives higher SNR. However, too low gain may result in quantization, lowering SNR.
 * Too high gain may result in saturation, corrupting the data.
 *
 * @param[in] config The configuration
 * @param[in] gain Receiver gain setting
 */
void acc_config_receiver_gain_set(acc_config_t *config, uint8_t gain);


/**
 * @brief Get receiver gain setting
 *
 * See @ref acc_config_receiver_gain_set
 *
 * @param[in] config The configuration
 * @return Receiver gain setting
 */
uint8_t acc_config_receiver_gain_get(const acc_config_t *config);


/**
 * @brief Set sweeps per frame
 *
 * Sets the number of sweeps that will be captured in each frame (measurement).
 * Can be set to 0 if e.g. only temperature measurement is wanted.
 *
 * @param[in] config The configuration
 * @param[in] sweeps Sweeps per frame
 */
void acc_config_sweeps_per_frame_set(acc_config_t *config, uint16_t sweeps);


/**
 * @brief Get the number of sweeps per frame
 *
 * See @ref acc_config_sweeps_per_frame_set
 *
 * @param[in] config The configuration
 * @return Sweeps per frame
 */
uint16_t acc_config_sweeps_per_frame_get(const acc_config_t *config);


/**
 * @brief Set the sweep rate
 *
 * Sets the sweep rate for sweeps in a frame (measurement).
 *
 * @param[in] config The configuration
 * @param[in] sweep_rate Sweep rate in Hz. Must be >= 0, 0 is interpreted as max sweep rate
 */
void acc_config_sweep_rate_set(acc_config_t *config, float sweep_rate);


/**
 * @brief Get the sweep rate
 *
 * See @ref acc_config_sweep_rate_set
 *
 * @param[in] config The configuration
 * @return Sweep rate in Hz
 */
float acc_config_sweep_rate_get(const acc_config_t *config);


/**
 * @brief Set continuous sweep mode
 *
 * In continuous sweep mode the timing will be identical over all sweeps, not
 * just the sweeps in a frame.
 *
 * Constraints:
 * - Frame rate must be set to unlimited (0.0)
 * - Sweep rate must be set (> 0)
 * - Inter frame idle state must be set equal to inter sweep idle state
 *
 * @param[in] config The configuration
 * @param[in] enabled true if continuous sweep mode should be enabled, false otherwise
 */
void acc_config_continuous_sweep_mode_set(acc_config_t *config, bool enabled);


/**
 * @brief Get continuous sweep mode
 *
 * See @ref acc_config_continuous_sweep_mode_set
 *
 * @param[in] config The configuration
 * @return true if continuous sweep mode is enabled, false otherwise
 */
bool acc_config_continuous_sweep_mode_get(const acc_config_t *config);


/**
 * @brief Set the frame rate
 *
 * Sets the frame rate.
 *
 * Setting the frame rate to unlimited (0) means that the rate is not limited by the
 * sensor but the rate that the host acknowledge and reads out the measurement data.
 *
 * @param[in] config The configuration
 * @param[in] frame_rate Frame rate in Hz. Must be >= 0, 0 is interpreted as unlimited
 */
void acc_config_frame_rate_set(acc_config_t *config, float frame_rate);


/**
 * @brief Get the frame rate
 *
 * See @ref acc_config_frame_rate_set
 *
 * @param[in] config The configuration
 * @return Frame rate
 */
float acc_config_frame_rate_get(const acc_config_t *config);


/**
 * @brief Enable or disable the transmitter
 *
 * If set to true, TX is enabled. This will enable the radio transmitter.
 * By turning the transmitter off the RX noise floor can be measured.
 *
 * @param[in] config The configuration
 * @param[in] enable true to enable the transmitter, false to disable it
 */
void acc_config_enable_tx_set(acc_config_t *config, bool enable);


/**
 * @brief Get transmitter enable configuration
 *
 * See @ref acc_config_enable_tx_set
 *
 * @param[in] config The configuration
 * @return true if the transmitter is enabled, false if it is disabled
 */
bool acc_config_enable_tx_get(const acc_config_t *config);


/**
 * @brief Set inter frame idle state
 *
 * The 'inter-frame idle state' is the state the sensor idles in between each frame.
 *
 * See also @ref acc_config_idle_state_t.
 *
 * The inter frame idle state of the frame must be deeper or the same as the inter sweep idle state.
 *
 * @param[in] config The configuration
 * @param[in] idle_state The idle state to use between frames
 */
void acc_config_inter_frame_idle_state_set(acc_config_t *config, acc_config_idle_state_t idle_state);


/**
 * @brief Get inter frame idle state
 *
 * See @ref acc_config_inter_frame_idle_state_set
 *
 * @param[in] config The configuration
 * @return The idle state to use between frames
 */
acc_config_idle_state_t acc_config_inter_frame_idle_state_get(const acc_config_t *config);


/**
 * @brief Set inter sweep idle state
 *
 * The 'inter-sweep idle state' is the state the sensor idles in between each sweep in a frame.
 *
 * See also @ref acc_config_idle_state_t.
 *
 * @param[in] config The configuration
 * @param[in] idle_state The idle state to use between sweeps within a frame
 */
void acc_config_inter_sweep_idle_state_set(acc_config_t *config, acc_config_idle_state_t idle_state);


/**
 * @brief Get inter sweep idle state
 *
 * See @ref acc_config_inter_sweep_idle_state_set
 *
 * @param[in] config The configuration
 * @return The idle state to use between sweeps within a frame
 */
acc_config_idle_state_t acc_config_inter_sweep_idle_state_get(const acc_config_t *config);


/**
 * @brief Set Pulse Repetition Frequency
 *
 * See @ref acc_config_prf_t for details.
 *
 * @param[in] config The configuration
 * @param[in] prf The Pulse Repetition Frequency to use
 */
void acc_config_prf_set(acc_config_t *config, acc_config_prf_t prf);


/**
 * @brief Get Pulse Repetition Frequency
 *
 * See @ref acc_config_prf_t for details.
 *
 * @param[in] config The configuration
 * @return Pulse Repetition Frequency
 */
acc_config_prf_t acc_config_prf_get(const acc_config_t *config);


/**
 * @brief Enable or disable phase enhancement
 *
 * If enabled, the data phase will be enhanced such that coherent distance filtering can be applied.
 * Given a single reflection from an object, the phase will appear as "flat" around the amplitude peak.
 *
 * Enabling the phase enhancement increases the processing execution time.
 *
 * @param[in] config The configuration
 * @param[in] enable true if phase enhancement should be enabled, false otherwise
 */
void acc_config_phase_enhancement_set(acc_config_t *config, bool enable);


/**
 * @brief Get the phase enhancement configuration
 *
 * See @ref acc_config_phase_enhancement_set
 *
 * @param[in] config The configuration
 * @return true if phase enhancement is enabled, false otherwise
 */
bool acc_config_phase_enhancement_get(const acc_config_t *config);


/**
 * @brief Enable or disable loopback
 *
 * Constraints:
 * - Loopback can't be enabled together with profile 2.
 *
 * @param[in] config The configuration
 * @param[in] enable true if loopback should be enabled, false otherwise
 */
void acc_config_enable_loopback_set(acc_config_t *config, bool enable);


/**
 * @brief Get the enable loopback configuration
 *
 * See @ref acc_config_enable_loopback_set
 *
 * @param[in] config The configuration
 * @return true if loopback is enabled, false otherwise
 */
bool acc_config_enable_loopback_get(const acc_config_t *config);


/**
 * @brief Enable or disable double buffering
 *
 * If enabled, the sensor buffer will be split in two halves reducing the
 * maximum number of samples. A frame can be read using @ref acc_sensor_read while
 * sampling is done into the other buffer. Switching of buffers is done automatically
 * by @ref acc_sensor_measure.
 *
 * When using double buffering, measurements coinciding with SPI activity may have distorted phase.
 * To mitigate this issue, applying a median filter is recommended.
 *
 * @param[in] config The configuration
 * @param[in] enable true if double buffering should be enabled, false otherwise
 */
void acc_config_double_buffering_set(acc_config_t *config, bool enable);


/**
 * @brief Get the double buffering configuration
 *
 * See @ref acc_config_double_buffering_set
 *
 * @param[in] config The configuration
 * @return true if double buffering is enabled, false otherwise
 */
bool acc_config_double_buffering_get(const acc_config_t *config);


/**
 * @}
 */


#endif
