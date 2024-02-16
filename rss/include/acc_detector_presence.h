// Copyright (c) Acconeer AB, 2022-2023
// All rights reserved

#ifndef ACC_DETECTOR_PRESENCE_H_
#define ACC_DETECTOR_PRESENCE_H_

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "acc_definitions_a121.h"
#include "acc_definitions_common.h"
#include "acc_processing.h"
#include "acc_sensor.h"

/**
 * @defgroup Presence Presence Detector
 * @ingroup Detectors
 *
 * @brief Presence detector API description
 *
 * For a detailed description of the presence detector algorithm and its
 * configuration parameters, see
 * <a href="https://docs.acconeer.com/en/latest/exploration_tool/algo/a121/detectors/presence_detection.html">
 * docs.acconeer.com</a>
 *
 * @{
 */


/**
 * @brief Presence detector handle
 */
struct acc_detector_presence_handle;

typedef struct acc_detector_presence_handle acc_detector_presence_handle_t;


/**
 * @brief Presence detector configuration container
 */
struct acc_detector_presence_config;

typedef struct acc_detector_presence_config acc_detector_presence_config_t;


/**
 * @brief Presence detector results container
 */
typedef struct
{
	/**
	 * true if presence was detected, false otherwise
	 */
	bool presence_detected;
	/**
	 * A measure of the amount of fast motion detected
	 */
	float intra_presence_score;
	/**
	 * A measure of the amount of slow motion detected
	 */
	float inter_presence_score;
	/**
	 * The distance, in meters, to the detected object
	 */
	float presence_distance;
	/**
	 * An array of measures of the amount of fast motion detected per distance point.
	 * This will point to memory in the buffer supplied to @ref acc_detector_presence_process
	 */
	float *depthwise_intra_presence_scores;
	/**
	 * An array of measures of the amount of slow motion detected per distance point.
	 * This will point to memory in the buffer supplied to @ref acc_detector_presence_process
	 */
	float *depthwise_inter_presence_scores;
	/**
	 * The number of elements in the depthwise presence scores arrays
	 */
	uint32_t depthwise_presence_scores_length;
	/**
	 * Radar data that the presence detection is based on.
	 * This will point to memory in the buffer supplied to @ref acc_detector_presence_process
	 */
	acc_processing_result_t processing_result;
} acc_detector_presence_result_t;


/**
 * brief Metadata for presence detector
 */
typedef struct
{
	/**
	 * Actual start point of measurement in m.
	 * This can be useful to know the exact start point of the measurement in m.
	 * The resolution of each point is approximately 2.5mm
	 */
	float start_m;
	/**
	 * Actual step length between each data point of the measurement in m.
	 * This can be useful when automatic selection of step length based on the profile
	 * is enabled through @ref acc_detector_presence_config_auto_step_length_set
	 */
	float step_length_m;
	/**
	 * Number of data points in measurement.
	 * This is calculated from the requested start and end point and the resulting
	 * step length. This corresponds to the length of the depthwise inter/intra
	 * presence score results, which can be useful to know already at detector creation.
	 */
	uint16_t num_points;
	/**
	 * Profile used.
	 * This can be useful when automatic selection of profile based on start point
	 * is enabled through @ref acc_detector_presence_config_auto_profile_set
	 */
	acc_config_profile_t profile;
} acc_detector_presence_metadata_t;


/**
 * @brief Create a configuration for a presence detector
 *
 * @return Presence detector configuration, NULL if creation was not possible
 */
acc_detector_presence_config_t *acc_detector_presence_config_create(void);


/**
 * @brief Destroy a presence detector configuration
 *
 * @param[in] presence_config The configuration to destroy
 */
void acc_detector_presence_config_destroy(acc_detector_presence_config_t *presence_config);


/**
 * @brief Print a configuration to the log
 *
 * @param[in] presence_config The configuration to log
 */
void acc_detector_presence_config_log(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Get the buffer size needed for the provided presence detector handle
 *
 * This buffer size can be used to allocate a memory buffer in the
 * application, which is needed for several functions in the detector library.
 * This size will also include memory for holding the depthwise inter/intra presence
 * score arrays that will be part of the result, see @ref acc_detector_presence_result_t
 *
 * @param[in] presence_handle The presence detector handle to to get the buffer size for
 * @param[out] buffer_size The buffer size
 * @return true if successful, false otherwise
 */
bool acc_detector_presence_get_buffer_size(const acc_detector_presence_handle_t *presence_handle, uint32_t *buffer_size);


/**
 * @brief Create a presence detector with the provided configuration
 *
 * @param[in] presence_config The presence detector configuration to create a presence detector with
 * @param[out] metadata Metadata for the presence detector given the presence_config
 * @return Presence detector handle, NULL if presence detector was not possible to create
 */
acc_detector_presence_handle_t *acc_detector_presence_create(acc_detector_presence_config_t   *presence_config,
                                                             acc_detector_presence_metadata_t *metadata);


/**
 * @brief Destroy a presence detector identified with the provided handle
 *
 * Destroy the context of a presence detector allowing another presence detector to be created using the
 * same resources.
 * If NULL is sent in, nothing happens.
 *
 * @param[in] presence_handle A reference to the presence detector handle to destroy
 */
void acc_detector_presence_destroy(acc_detector_presence_handle_t *presence_handle);


/**
 * @brief Prepare the detector to do a measurement
 *
 * @param[in] presence_handle The presence detector handle to prepare for
 * @param[in] presence_config The configuration to prepare with
 * @param[in] sensor The sensor instance to prepare
 * @param[in] cal_result The calibration result to prepare with
 * @param[in] buffer Memory used by the detector to prepare the sensor for measurements
 *            The buffer will only be used during the duration of this call
 * @param[in] buffer_size The size in bytes of the buffer, should be at least buffer_size
 *            from @ref acc_detector_presence_get_buffer_size
 * @return true if successful, false otherwise
 */
bool acc_detector_presence_prepare(acc_detector_presence_handle_t *presence_handle, acc_detector_presence_config_t *presence_config,
                                   acc_sensor_t *sensor, const acc_cal_result_t *cal_result, void *buffer, uint32_t buffer_size);


/**
 * @brief Process the data according to the configuration used in @ref acc_detector_presence_config_create
 *
 * @param[in] presence_handle The presence detector handle for the presence detector to get the next result for
 * @param[in] buffer  A reference to the buffer (populated by @ref acc_sensor_read) containing the
 *                    data to be processed.
 *                    After this function returns, the depthwise inter/intra presence that is part of the
 *                    result (@ref acc_detector_presence_result_t) will point to memory located in this buffer.
 *                    If these arrays are of interest for the application they need to be processed
 *                    before the buffer is used in any other function.
 * @param[out] result Presence detector results
 * @return true if successful, otherwise false
 */
bool acc_detector_presence_process(acc_detector_presence_handle_t *presence_handle, void *buffer,
                                   acc_detector_presence_result_t *result);


/**
 * @brief Set the start point of measurement interval in meters
 *
 * @param[in] presence_config The configuration
 * @param[in] start The start point of measurement interval in meters
 */
void acc_detector_presence_config_start_set(acc_detector_presence_config_t *presence_config, float start);


/**
 * @brief Get the start point of measurement interval in meters
 *
 * @param[in] presence_config The configuration
 * @return The start point of measurement interval in meters
 */
float acc_detector_presence_config_start_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set the end point of measurement interval in meters
 *
 * @param[in] presence_config The configuration
 * @param[in] end The end point of measurement interval in meters
 */
void acc_detector_presence_config_end_set(acc_detector_presence_config_t *presence_config, float end);


/**
 * @brief Get the end point of measurement interval in meters
 *
 * @param[in] presence_config The configuration
 * @return The end point of measurement interval in meters
 */
float acc_detector_presence_config_end_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set the step length in points
 *
 * This sets the number of steps between each data point.
 *
 * The set step length will only be used if step length auto selection was disabled
 * through @ref acc_detector_presence_config_auto_step_length_set
 *
 * Sampling produces complex (IQ) data points with configurable distance spacing,
 * starting from ~2.5mm.
 *
 * @param[in] presence_config The configuration
 * @param[in] step_length The step length
 */
void acc_detector_presence_config_step_length_set(acc_detector_presence_config_t *presence_config, uint16_t step_length);


/**
 * @brief Get the step length in points
 *
 * @see acc_detector_presence_config_step_length_set
 *
 * @param[in] presence_config The configuration
 * @return The step length
 */
uint16_t acc_detector_presence_config_step_length_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Enable automatic selection of step length based on the profile
 *
 * The highest possible step length based on the fwhm of the set profile
 * with the goal to achieve detection on the complete range with minimum number
 * of sampling points
 *
 * @param[in] presence_config The configuration
 * @param[in] enable true to enable auto selection, false to disable
 */
void acc_detector_presence_config_auto_step_length_set(acc_detector_presence_config_t *presence_config, bool enable);


/**
 * @brief Get if automatic selection of step length based on the profile is enabled
 *
 * See @ref acc_detector_presence_config_auto_step_length_set
 *
 * @param[in] presence_config The configuration
 * @return true if automatic selection of step length is enabled, false if disabled
 */
bool acc_detector_presence_config_auto_step_length_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set a profile
 *
 * Each profile consists of a number of settings for the sensor that configures
 * the RX and TX paths. Lower profiles have higher depth resolution while
 * higher profiles have higher SNR.
 *
 * The set profile will only be used if profile auto selection was disabled
 * through @ref acc_detector_presence_config_auto_profile_set
 *
 * @param[in] presence_config The configuration
 * @param[in] profile The profile to set
 */
void acc_detector_presence_config_profile_set(acc_detector_presence_config_t *presence_config, acc_config_profile_t profile);


/**
 * @brief Get the currently set profile
 *
 * See @ref acc_detector_presence_config_profile_set
 *
 * @param[in] presence_config The configuration
 * @return The profile currently used
 */
acc_config_profile_t acc_detector_presence_config_profile_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Enable automatic selection of profile based on start point of measurement
 *
 * The highest possible profile without interference of direct leakage will used to maximize SNR
 *
 * @param[in] presence_config The configuration
 * @param[in] enable true to enable auto selection, false to disable
 */
void acc_detector_presence_config_auto_profile_set(acc_detector_presence_config_t *presence_config, bool enable);


/**
 * @brief Get if automatic selection of profile based on start point of measurement is enabled
 *
 * See @ref acc_detector_presence_config_auto_profile_set
 *
 * @param[in] presence_config The configuration
 * @return true if automatic selection of profile is enabled, false if disabled
 */
bool acc_detector_presence_config_auto_profile_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set inter frame idle state
 *
 * The 'inter-frame idle state' is the state the sensor idles in between each frame.
 *
 * See also @ref acc_config_idle_state_t.
 *
 * @param[in] presence_config The configuration
 * @param[in] idle_state The idle state to use between frames
 */
void acc_detector_presence_config_inter_frame_idle_state_set(acc_detector_presence_config_t *presence_config,
                                                             acc_config_idle_state_t        idle_state);


/**
 * @brief Get inter frame idle state
 *
 * See @ref acc_detector_presence_config_inter_frame_idle_state_set
 *
 * @param[in] presence_config The configuration
 * @return The idle state to use between frames
 */
acc_config_idle_state_t acc_detector_presence_config_inter_frame_idle_state_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set the hardware accelerated average samples (HWAAS)
 *
 * See @ref acc_config_hwaas_set for more details
 *
 * @param[in] presence_config The configuration
 * @param[in] hwaas Hardware accelerated average samples
 */
void acc_detector_presence_config_hwaas_set(acc_detector_presence_config_t *presence_config, uint16_t hwaas);


/**
 * @brief Get the hardware accelerated average samples (HWAAS)
 *
 * See @ref acc_detector_presence_config_hwaas_set
 *
 * @param[in] presence_config The configuration
 * @return Hardware accelerated average samples
 */
uint16_t acc_detector_presence_config_hwaas_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set the number of sweeps per frame
 *
 * Sets the number of sweeps that will be captured in each frame (measurement).
 *
 * @param[in] presence_config The configuration
 * @param[in] sweeps_per_frame Sweeps per frame, must be at least 6
 */
void acc_detector_presence_config_sweeps_per_frame_set(acc_detector_presence_config_t *presence_config, uint16_t sweeps_per_frame);


/**
 * @brief Get the number of sweeps per frame
 *
 * See @ref acc_detector_presence_config_sweeps_per_frame_set
 *
 * @param[in] presence_config The configuration
 * @return Sweeps per frame
 */
uint16_t acc_detector_presence_config_sweeps_per_frame_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set the frame rate
 *
 * This frame rate is maintained by the sensor if @ref acc_detector_presence_config_frame_rate_app_driven_set
 * is invoked with false (default) and the application must maintain the given frame rate if invoked with true.
 * If the application maintains the frame rate it is important that it doesn't deviate more than 10%
 * from the set value for the presence algorithm to work optimally.
 * See @ref acc_config_frame_rate_set for details
 *
 * @param[in] presence_config The configuration
 * @param[in] frame_rate Frame rate in Hz. Must be > 0
 */
void acc_detector_presence_config_frame_rate_set(acc_detector_presence_config_t *presence_config, float frame_rate);


/**
 * @brief Get the frame rate
 *
 * See @ref acc_detector_presence_config_frame_rate_set
 *
 * @param[in] presence_config The configuration
 * @return Frame rate in Hz
 */
float acc_detector_presence_config_frame_rate_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set if the application should maintain the requested frame rate
 *
 * If set to true, the application must maintain the frame rate set using
 * @ref acc_detector_presence_config_frame_rate_set
 * If set to false, the frame rate is maintained by the sensor at the frame rate given by
 * @ref acc_detector_presence_config_frame_rate_set.
 *
 * @param[in] presence_config The configuration
 * @param[in] enable true to enable application driven frame rate, false to disable
 */
void acc_detector_presence_config_frame_rate_app_driven_set(acc_detector_presence_config_t *presence_config, bool enable);


/**
 * @brief Get if the application should maintain the requested frame rate
 *
 * See @ref acc_detector_presence_config_frame_rate_app_driven_set
 *
 * @param[in] presence_config The configuration
 * @return true if application driven frame rate is enabled, false if disabled
 */
bool acc_detector_presence_config_frame_rate_app_driven_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set sensor ID
 *
 * @param[in] presence_config The configuration to set the sensor ID for
 * @param[in] sensor_id The sensor ID
 */
void acc_detector_presence_config_sensor_set(acc_detector_presence_config_t *presence_config, acc_sensor_id_t sensor_id);


/**
 * @brief Get sensor ID
 *
 * @param[in] presence_config The configuration to get the sensor ID for
 * @return sensor ID
 */
acc_sensor_id_t acc_detector_presence_config_sensor_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set if the presence filters should reset on prepare
 *
 * If set to true, the presence filters will be reset when
 * @ref acc_detector_presence_prepare is invoked.
 *
 * @param[in] presence_config The configuration
 * @param[in] enable true to reset the filters on prepare, false to not reset
 */
void acc_detector_presence_config_reset_filters_on_prepare_set(acc_detector_presence_config_t *presence_config, bool enable);


/**
 * @brief Get if the presence filters should reset on prepare
 *
 * See @ref acc_detector_presence_config_reset_filters_on_prepare_set
 *
 * @param[in] presence_config The configuration
 * @return true if filters should reset on prepare, false otherwise
 */
bool acc_detector_presence_config_reset_filters_on_prepare_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set the inter-frame presence timeout in seconds
 *
 * Number of seconds the inter-frame presence score needs to decrease before exponential
 * scaling starts for faster decline. Should be between 0 and 30 where 0 means no timeout
 *
 * @param[in] presence_config The configuration
 * @param[in] inter_frame_presence_timeout Timeout in seconds between 0 and 30
 */
void acc_detector_presence_config_inter_frame_presence_timeout_set(acc_detector_presence_config_t *presence_config,
                                                                   uint16_t                       inter_frame_presence_timeout);


/**
 * @brief Get the inter-frame presence timeout in seconds
 *
 * See @ref acc_detector_presence_config_inter_frame_presence_timeout_set
 *
 * @param[in] presence_config The configuration
 * @return Inter-frame presence timeout in s
 */
uint16_t acc_detector_presence_config_inter_frame_presence_timeout_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set inter-frame phase boost
 *
 * Used to increase detection of slow motions by utilizing the phase information in the Sparse IQ data.
 *
 * @param[in] presence_config The configuration to set inter phase boost for
 * @param[in] enable true if inter phase boost should be enabled
 */
void acc_detector_presence_config_inter_phase_boost_set(acc_detector_presence_config_t *presence_config, bool enable);


/**
 * @brief Get if inter-frame phase boost is enabled
 *
 * See @ref acc_detector_presence_config_inter_phase_boost_set
 *
 * @param[in] presence_config The configuration to get inter phase boost for
 * @return true if inter-frame phase boost is enabled, false otherwise
 */
bool acc_detector_presence_config_inter_phase_boost_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set intra-frame presence detection
 *
 * This is used for detecting faster movements inside frames
 *
 * @param[in] presence_config The configuration to set intra-frame detection for
 * @param[in] enable true if intra-frame detection should be enabled
 */
void acc_detector_presence_config_intra_detection_set(acc_detector_presence_config_t *presence_config, bool enable);


/**
 * @brief Get if frame intra-frame presence detection is enabled
 *
 * See @ref acc_detector_presence_config_intra_detection_set
 *
 * @param[in] presence_config The configuration to get intra detection for
 * @return true if intra-frame detection is enabled, false otherwise
 */
bool acc_detector_presence_config_intra_detection_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set the detection threshold for the intra-frame presence detection
 *
 * This is the threshold for detecting faster movements inside frames
 *
 * @param[in] presence_config The configuration to set the detection threshold for
 * @param[in] intra_detection_threshold The intra-frame detection threshold to set
 */
void acc_detector_presence_config_intra_detection_threshold_set(acc_detector_presence_config_t *presence_config,
                                                                float                          intra_detection_threshold);


/**
 * @brief Get the detection threshold for the intra-frame presence detection
 *
 * See @ref acc_detector_presence_config_intra_detection_threshold_set
 *
 * @param[in] presence_config The configuration to get the detection threshold for
 * @return The intra-frame detection threshold
 */
float acc_detector_presence_config_intra_detection_threshold_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set inter-frame presence detection
 *
 * This is used for detecting slower movements between frames
 *
 * @param[in] presence_config The configuration to set inter-frame detection for
 * @param[in] enable true if inter-frame presence detection should be enabled
 */
void acc_detector_presence_config_inter_detection_set(acc_detector_presence_config_t *presence_config, bool enable);


/**
 * @brief Get if inter-frame presence detection is enabled
 *
 * See @ref acc_detector_presence_config_inter_detection_set
 *
 * @param[in] presence_config The configuration to get inter-frame presence detection for
 * @return true if inter-frame presence detection is enabled, false otherwise
 */
bool acc_detector_presence_config_inter_detection_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set the detection threshold for the inter-frame presence detection
 *
 * This is the threshold for detecting slower movements between frames
 *
 * @param[in] presence_config The configuration to set the detection threshold for
 * @param[in] inter_detection_threshold The threshold
 */
void acc_detector_presence_config_inter_detection_threshold_set(acc_detector_presence_config_t *presence_config,
                                                                float                          inter_detection_threshold);


/**
 * @brief Get the detection threshold for the inter-frame presence detection
 *
 * See @ref acc_detector_presence_config_inter_detection_threshold_set
 *
 * @param[in] presence_config The configuration to get the detection threshold for
 * @return detection threshold
 */
float acc_detector_presence_config_inter_detection_threshold_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set the time constant of the low pass filter for the inter-frame deviation between fast and slow
 *
 * @param[in] presence_config The configuration
 * @param[in] inter_frame_deviation_time_const Time constant to set
 */
void acc_detector_presence_config_inter_frame_deviation_time_const_set(acc_detector_presence_config_t *presence_config,
                                                                       float                          inter_frame_deviation_time_const);


/**
 * @brief Get the time constant of the low pass filter for the inter-frame deviation between fast and slow
 *
 * @param[in] presence_config The configuration to get the time constant for
 * @return time constant in s
 */
float acc_detector_presence_config_inter_frame_deviation_time_const_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set the cutoff frequency of the low pass filter for the fast filtered absolute sweep mean
 *
 * No filtering is applied if the cutoff is set over half the frame rate (Nyquist limit).
 *
 * @param[in] presence_config The configuration
 * @param[in] inter_frame_fast_cutoff Cutoff frequency to set
 */
void acc_detector_presence_config_inter_frame_fast_cutoff_set(acc_detector_presence_config_t *presence_config,
                                                              float                          inter_frame_fast_cutoff);


/**
 * @brief Get the cutoff frequency of the low pass filter for the fast filtered absolute sweep mean
 *
 * @param[in] presence_config The configuration to get the cutoff frequency for
 * @return the cutoff frequency in Hz
 */
float acc_detector_presence_config_inter_frame_fast_cutoff_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set the cutoff frequency of the low pass filter for the slow filtered absolute sweep mean
 *
 * @param[in] presence_config The configuration
 * @param[in] inter_frame_slow_cutoff Cutoff frequency to set
 */
void acc_detector_presence_config_inter_frame_slow_cutoff_set(acc_detector_presence_config_t *presence_config,
                                                              float                          inter_frame_slow_cutoff);


/**
 * @brief Get the cutoff frequency of the low pass filter for the slow filtered absolute sweep mean
 *
 * @param[in] presence_config The configuration to get the cutoff frequency for
 * @return the cutoff frequency in Hz
 */
float acc_detector_presence_config_inter_frame_slow_cutoff_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set the time constant for the depthwise filtering in the intra-frame part
 *
 * @param[in] presence_config The configuration
 * @param[in] intra_frame_time_const Time constant to set
 */
void acc_detector_presence_config_intra_frame_time_const_set(acc_detector_presence_config_t *presence_config,
                                                             float                          intra_frame_time_const);


/**
 * @brief Get the time constant for the depthwise filtering in the intra-frame part
 *
 * @param[in] presence_config The configuration to get the time constant for
 * @return time constant in s
 */
float acc_detector_presence_config_intra_frame_time_const_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set the time constant for the output in the intra-frame part
 *
 * @param[in] presence_config The configuration
 * @param[in] intra_output_time_const Time constant to set
 */
void acc_detector_presence_config_intra_output_time_const_set(acc_detector_presence_config_t *presence_config,
                                                              float                          intra_output_time_const);


/**
 * @brief Get the time constant for the output in the intra-frame part
 *
 * @param[in] presence_config The configuration to get the time constant for
 * @return time constant in s
 */
float acc_detector_presence_config_intra_output_time_const_get(const acc_detector_presence_config_t *presence_config);


/**
 * @brief Set the time constant for the output in the inter-frame part
 *
 * @param[in] presence_config The configuration
 * @param[in] inter_output_time_const Time constant to set
 */
void acc_detector_presence_config_inter_output_time_const_set(acc_detector_presence_config_t *presence_config,
                                                              float                          inter_output_time_const);


/**
 * @brief Get the time constant for the output in the inter-frame part
 *
 * @param[in] presence_config The configuration to get the time constant for
 * @return time constant in s
 */
float acc_detector_presence_config_inter_output_time_const_get(const acc_detector_presence_config_t *presence_config);


/**
 * @}
 */

#endif
