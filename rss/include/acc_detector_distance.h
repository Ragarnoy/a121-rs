// Copyright (c) Acconeer AB, 2022-2023
// All rights reserved

#ifndef ACC_DETECTOR_DISTANCE_H_
#define ACC_DETECTOR_DISTANCE_H_


#include <stdbool.h>
#include <stdint.h>

#include "acc_definitions_a121.h"
#include "acc_definitions_common.h"
#include "acc_detector_distance_definitions.h"
#include "acc_processing.h"
#include "acc_sensor.h"

/**
 * @defgroup Distance Distance Detector
 * @ingroup Detectors
 *
 * @brief Distance detector API description
 *
 * For a detailed description of the algorithm and its parameters, see
 * <a href="https://docs.acconeer.com/en/latest/exploration_tool/algo/a121/detectors/distance_detection.html">docs.acconeer.com</a>
 * @{
 */

#define ACC_DETECTOR_DISTANCE_RESULT_MAX_NUM_DISTANCES (10U)


/**
 * @brief Distance detector handle
 */
struct acc_detector_distance_handle;

typedef struct acc_detector_distance_handle acc_detector_distance_handle_t;


/**
 * @brief Configuration of the distance detector
 */
struct acc_detector_distance_config;

typedef struct acc_detector_distance_config acc_detector_distance_config_t;


/**
 * @brief Distance detector result
 */
typedef struct
{
	/**
	 * The detected distances in meters
	 */
	float distances[ACC_DETECTOR_DISTANCE_RESULT_MAX_NUM_DISTANCES];
	/**
	 * The reflective strengths of each distance
	 */
	float strengths[ACC_DETECTOR_DISTANCE_RESULT_MAX_NUM_DISTANCES];
	/**
	 * The number of detected distances. If 0, no distances where detected
	 */
	uint8_t num_distances;
	/**
	 * Indicating that there might be an object near the start point of the measured range
	 */
	bool near_start_edge_status;
	/**
	 * Indication of calibration needed.
	 *
	 * The sensor calibration needs to be redone if this indication is set.
	 *
	 * A detector calibration update should then be done after the new sensor calibration.
	 * A detector calibration update is done by calling @ref acc_detector_distance_update_calibration
	 */
	bool calibration_needed;
	/** Temperature in sensor during measurement (in degree Celsius).
	 *  Note that it has poor absolute accuracy and should only be used
	 *  for relative temperature measurements.
	 */
	int16_t temperature;
	/**
	 * Radar data that the distance detection is based on.
	 * This will point to memory in the buffer supplied to @ref acc_detector_distance_process
	 *
	 * Note: The processing result is only valid until the next time
	 *        @ref acc_detector_distance_process is called.
	 */
	acc_processing_result_t *processing_result;
	/**
	 * The metadata for the processing result
	 *
	 * Note: The processing metedata is only valid until the next time
	 *        @ref acc_detector_distance_process is called.
	 */
	acc_processing_metadata_t *processing_metadata;
	/**
	 * The sensor_config used for the processing result
	 *
	 * Note: The sensor_config is only valid until the next time
	 *        @ref acc_detector_distance_process is called.
	 */
	const acc_config_t *sensor_config;
} acc_detector_distance_result_t;


/**
 * @brief Create a configuration for a distance detector
 *
 * @return Distance detector configuration, NULL in case of error
 */
acc_detector_distance_config_t *acc_detector_distance_config_create(void);


/**
 * @brief Destroy a configuration for a distance detector
 *
 * @param[in] config The configuration to destroy
 */
void acc_detector_distance_config_destroy(acc_detector_distance_config_t *config);


/**
 * @brief Print a configuration to the log
 *
 * @param[in] handle The distance detector handle, if NULL only distance config will be logged
 * @param[in] config The configuration to log
 */
void acc_detector_distance_config_log(const acc_detector_distance_handle_t *handle, const acc_detector_distance_config_t *config);


/**
 * @brief Get the sizes needed given the provided detector handle
 *
 * buffer_size is the size of memory needed by the detector for proper operation. This includes memory
 * for sensor handling and detector calculations. This memory can be reused between instances.
 *
 * detector_cal_result_static_size is the size of the static part of the detector calibration result.
 *
 * Both size are dependent on the configuration used which is contained in the provided handle.
 *
 * @param[in] handle The distance detector handle
 * @param[out] buffer_size The buffer size
 * @param[out] detector_cal_result_static_size The calibration result size
 * @return true if successful, false otherwise
 */
bool acc_detector_distance_get_sizes(const acc_detector_distance_handle_t *handle,
                                     uint32_t                             *buffer_size,
                                     uint32_t                             *detector_cal_result_static_size);


/**
 * @brief Create a distance detector with the provided configuration
 *
 * @param[in] config The configuration to create a distance detector with
 * @return Distance detector handle, NULL if distance detector was not possible to create
 */
acc_detector_distance_handle_t *acc_detector_distance_create(const acc_detector_distance_config_t *config);


/**
 * @brief Destroy the distance detector handle, freeing its resources
 *
 * @param[in] handle The handle to destroy
 */
void acc_detector_distance_destroy(acc_detector_distance_handle_t *handle);


/**
 * @brief Do a detector calibration
 *
 * The calibration is dependent on the config used. This means that the duration of the
 * calibration is dependent on the config used. For example, a config with a fixed threshold
 * will not need to record the background as opposed to a config with a recorded threshold.
 *
 * The calibration needs a valid sensor calibration result for proper operation.
 *
 * The calibration produces two results, one static and one dynamic. The static result is not
 * temperature dependent and thus can be used in all temperatures. The dynamic result is
 * temperature dependent and needs to be updated if the temperature changes, which is indicated
 * by the 'calibration_needed' indication.
 *
 * @param[in] sensor The sensor instance to use for calibration
 * @param[in] handle The detector handle
 * @param[in] sensor_cal_result Sensor calibration result
 * @param[in] buffer Working memory buffer needed by function
 * @param[in] buffer_size The size of buffer. Needs to be at least
 *            the result of @ref acc_detector_distance_get_sizes
 * @param[out] detector_cal_result_static Static result of calibration
 * @param[in] detector_cal_result_static_size The size of detector_cal_result_static.
 *            Needs to be at least the result of @ref acc_detector_distance_get_sizes
 * @param[out] detector_cal_result_dynamic Dynamic result of calibration
 * @param[out] calibration_complete Will be set to true when the calibration is complete.
 *             If false; at least one more call to this function is needed.
 *             Note that it's necessary to wait for interrupt between calls.
 * @return true if successful, false otherwise
 */
bool acc_detector_distance_calibrate(acc_sensor_t                      *sensor,
                                     acc_detector_distance_handle_t    *handle,
                                     const acc_cal_result_t            *sensor_cal_result,
                                     void                              *buffer,
                                     uint32_t                          buffer_size,
                                     uint8_t                           *detector_cal_result_static,
                                     uint32_t                          detector_cal_result_static_size,
                                     acc_detector_cal_result_dynamic_t *detector_cal_result_dynamic,
                                     bool                              *calibration_complete);


/**
 * @brief Update the calibration
 *
 * This function should be called if the 'calibration_needed' indication is set,
 * after a new sensor calibration has been done.
 *
 * The calibration update needs a valid sensor calibration result for proper operation.
 *
 * @param[in] sensor The sensor instance to use for calibration update
 * @param[in] handle The detector handle
 * @param[in] sensor_cal_result Sensor calibration result
 * @param[in] buffer Working memory buffer needed by function
 * @param[in] buffer_size The size of buffer. Needs to be at least
 *            the result of @ref acc_detector_distance_get_sizes
 * @param[out] detector_cal_result_dynamic Result of the calibration update
 * @param[out] calibration_complete Will be set to true when the calibration update is complete.
 *             If false; at least one more call to this function is needed.
 *             Note that it's necessary to wait for interrupt between calls.
 * @return true if successful, false otherwise
 */
bool acc_detector_distance_update_calibration(acc_sensor_t                      *sensor,
                                              acc_detector_distance_handle_t    *handle,
                                              const acc_cal_result_t            *sensor_cal_result,
                                              void                              *buffer,
                                              uint32_t                          buffer_size,
                                              acc_detector_cal_result_dynamic_t *detector_cal_result_dynamic,
                                              bool                              *calibration_complete);


/**
 * @brief Prepare the detector for measurements
 *
 * This should to be done before every measure/wait for interrupt/read, as it reconfigures the sensor.
 *
 * @param[in, out] handle The distance detector handle
 * @param[in] config The distance detector config
 * @param[in] sensor The sensor instance to prepare
 * @param[in] sensor_cal_result The sensor calibration result to prepare with
 * @param[in] buffer Memory used by the detector. Should be at least buffer_size bytes
 * @param[in] buffer_size The buffer size received by @ref acc_detector_distance_get_sizes
 * @return true if successful, false otherwise
 */
bool acc_detector_distance_prepare(const acc_detector_distance_handle_t *handle,
                                   const acc_detector_distance_config_t *config,
                                   acc_sensor_t                         *sensor,
                                   const acc_cal_result_t               *sensor_cal_result,
                                   void                                 *buffer,
                                   uint32_t                             buffer_size);


/**
 * @brief Process the data according to the configuration used in @ref acc_detector_distance_config_create
 *
 * @param[in] handle The distance detector handle
 * @param[in] buffer A reference to the buffer (populated by @ref acc_sensor_read) containing the
 *                    data to be processed.
 * @param[in] detector_cal_result_static The result from @ref acc_detector_distance_calibrate
 * @param[in] detector_cal_result_dynamic The result from @ref acc_detector_distance_calibrate or @ref acc_detector_distance_update_calibration
 * @param[out] result_available Whether result will contain a new result
 * @param[out] result Distance detector result
 * @return true if successful, false otherwise
 */
bool acc_detector_distance_process(acc_detector_distance_handle_t    *handle,
                                   void                              *buffer,
                                   uint8_t                           *detector_cal_result_static,
                                   acc_detector_cal_result_dynamic_t *detector_cal_result_dynamic,
                                   bool                              *result_available,
                                   acc_detector_distance_result_t    *result);


/**
 * @brief Set the sensor ID
 *
 * @param[out] config The distance detector config
 * @param[in] sensor Sensor ID
 */
void acc_detector_distance_config_sensor_set(acc_detector_distance_config_t *config, acc_sensor_id_t sensor);


/**
 * @brief Get the sensor ID
 *
 * @param[in] config The distance detector config
 * @return Sensor ID
 */
acc_sensor_id_t acc_detector_distance_config_sensor_get(const acc_detector_distance_config_t *config);


/**
 * @brief Set the start of measured interval in meters.
 *
 * @param[out] config The distance detector config
 * @param[in] start_m Starting point in meters.
 */
void acc_detector_distance_config_start_set(acc_detector_distance_config_t *config, float start_m);


/**
 * @brief Get the start of measured interval in meters.
 *
 * @param[in] config The distance detector config
 * @return the start point in meters
 */
float acc_detector_distance_config_start_get(const acc_detector_distance_config_t *config);


/**
 * @brief Set the end of measured interval in meters.
 *
 * @param[out] config The distance detector config
 * @param[in] end_m End point in meters.
 */
void acc_detector_distance_config_end_set(acc_detector_distance_config_t *config, float end_m);


/**
 * @brief Get the end of measured interval in meters.
 *
 * @param[in] config The distance detector config
 * @return the end point in meters
 */
float acc_detector_distance_config_end_get(const acc_detector_distance_config_t *config);


/**
 * @brief Set the maximum step length
 *
 * Used to limit step length. If set to 0 (default), the step length is calculated
 * based on profile.
 *
 * @param[out] config The distance detector config
 * @param[in] max_step_length The maximum step length
 */
void acc_detector_distance_config_max_step_length_set(acc_detector_distance_config_t *config, uint16_t max_step_length);


/**
 * @brief Get the maximum step length
 *
 * @param[in] config The distance detector config
 * @return the maximum step length
 */
uint16_t acc_detector_distance_config_max_step_length_get(const acc_detector_distance_config_t *config);


/**
 * @brief Enable the close range leakage cancellation logic
 *
 * Close range leakage cancellation refers to the process of measuring close to the
 * sensor(<100mm) by first characterizing the direct leakage, and then subtracting it
 * from the measured sweep in order to isolate the signal component of interest.
 * The close range leakage cancellation process requires the sensor to be installed in its
 * intended geometry with free space in front of the sensor during detector calibration.
 *
 * @param[out] config The distance detector config
 * @param[in] enable true to enable close range leakage cancellation logic, false to disable
 */
void acc_detector_distance_config_close_range_leakage_cancellation_set(acc_detector_distance_config_t *config, bool enable);


/**
 * @brief Get if the close range leakage cancellation logic is enabled
 *
 * @param[in] config The distance detector config
 * @return true if close range leakage cancellation logic is enabled, false if disabled
 */
bool acc_detector_distance_config_close_range_leakage_cancellation_get(const acc_detector_distance_config_t *config);


/**
 * @brief Set the signal quality
 *
 * High signal quality results in a better SNR (because of higher HWAAS) and higher power consumption.
 * Signal quality can be set within the interval [-10, 35].
 *
 * @param[out] config The distance detector config
 * @param[in] signal_quality The signal quality
 */
void acc_detector_distance_config_signal_quality_set(acc_detector_distance_config_t *config, float signal_quality);


/**
 * @brief Get the signal quality
 *
 * @param[in] config The distance detector config
 * @return the signal quality
 */
float acc_detector_distance_config_signal_quality_get(const acc_detector_distance_config_t *config);


/**
 * @brief Set the max profile
 *
 * Specifies the highest allowed profile (the default is the highest, Profile 5).
 * A higher profile yields better SNR but worse distance resolution.
 *
 * @param[out] config The distance detector config
 * @param[in] max_profile The max profile
 */
void acc_detector_distance_config_max_profile_set(acc_detector_distance_config_t *config, acc_config_profile_t max_profile);


/**
 * @brief Get the max profile
 *
 * @param[in] config The distance detector config
 * @return the max profile
 */
acc_config_profile_t acc_detector_distance_config_max_profile_get(const acc_detector_distance_config_t *config);


/**
 * @brief Set the threshold method
 *
 * See @ref acc_detector_distance_threshold_method_t for details
 *
 * @param[out] config The distance detector config
 * @param[in] threshold_method The threshold method
 */
void acc_detector_distance_config_threshold_method_set(acc_detector_distance_config_t           *config,
                                                       acc_detector_distance_threshold_method_t threshold_method);


/**
 * @brief Get the threshold method
 *
 * @param[in] config The distance detector config
 * @return the threshold method
 */
acc_detector_distance_threshold_method_t acc_detector_distance_config_threshold_method_get(const acc_detector_distance_config_t *config);


/**
 * @brief Set the peak sorting method
 *
 * See @ref acc_detector_distance_peak_sorting_t for details
 *
 * @param[out] config The distance detector config
 * @param[in] peak_sorting The peak sorting method
 */
void acc_detector_distance_config_peak_sorting_set(acc_detector_distance_config_t *config, acc_detector_distance_peak_sorting_t peak_sorting);


/**
 * @brief Get the peak sorting method
 *
 * See @ref acc_detector_distance_config_peak_sorting_set
 *
 * @param[in] config The distance detector config
 * @return The peak sorting method
 */
acc_detector_distance_peak_sorting_t acc_detector_distance_config_peak_sorting_get(const acc_detector_distance_config_t *config);


/**
 * @brief Set the number frames to use for recorded threshold
 *
 * @param[out] config The distance detector config
 * @param[in] num_frames Number of frames
 */
void acc_detector_distance_config_num_frames_recorded_threshold_set(acc_detector_distance_config_t *config, uint16_t num_frames);


/**
 * @brief Get the number of frames to use for recorded threshold
 *
 * @param[in] config The distance detector config
 * @return Number of frames
 */
uint16_t acc_detector_distance_config_num_frames_recorded_threshold_get(const acc_detector_distance_config_t *config);


/**
 * @brief Set fixed amplitude threshold value
 *
 * This value is used when the threshold method is set to @ref ACC_DETECTOR_DISTANCE_THRESHOLD_METHOD_FIXED_AMPLITUDE
 *
 * @param[out] config The distance detector config
 * @param[in] fixed_threshold_value The fixed threshold value
 */
void acc_detector_distance_config_fixed_amplitude_threshold_value_set(acc_detector_distance_config_t *config, float fixed_threshold_value);


/**
 * @brief Get fixed amplitude threshold value
 *
 * See @ref acc_detector_distance_config_fixed_amplitude_threshold_value_set
 *
 * @param[in] config The distance detector config
 * @return The fixed threshold value
 */
float acc_detector_distance_config_fixed_amplitude_threshold_value_get(const acc_detector_distance_config_t *config);


/**
 * @brief Set fixed strength threshold value
 *
 * This value is used when the threshold method is set to @ref ACC_DETECTOR_DISTANCE_THRESHOLD_METHOD_FIXED_STRENGTH
 *
 * @param[out] config The distance detector config
 * @param[in] fixed_threshold_value The fixed threshold value
 */
void acc_detector_distance_config_fixed_strength_threshold_value_set(acc_detector_distance_config_t *config, float fixed_threshold_value);


/**
 * @brief Get fixed strength threshold value
 *
 * See @ref acc_detector_distance_config_fixed_strength_threshold_value_set
 *
 * @param[in] config The distance detector config
 * @return The fixed threshold value
 */
float acc_detector_distance_config_fixed_strength_threshold_value_get(const acc_detector_distance_config_t *config);


/**
 * @brief Set threshold sensitivity
 *
 * High sensitivity yields a low detection threshold, low sensitivity yields a high detection threshold.
 * Threshold sensitivity can be set within the interval [0, 1].
 *
 * @param[out] config The distance detector config
 * @param[in] threshold_sensitivity The threshold sensitivity
 */
void acc_detector_distance_config_threshold_sensitivity_set(acc_detector_distance_config_t *config, float threshold_sensitivity);


/**
 * @brief Get threshold sensitivity
 *
 * @param[in] config The distance detector config
 * @return The threshold sensitivity
 */
float acc_detector_distance_config_threshold_sensitivity_get(const acc_detector_distance_config_t *config);


/**
 * @brief Set reflector shape
 *
 * @param[out] config The distance detector config
 * @param[in] reflector_shape The reflector shape
 */
void acc_detector_distance_config_reflector_shape_set(acc_detector_distance_config_t          *config,
                                                      acc_detector_distance_reflector_shape_t reflector_shape);


/**
 * @brief Get reflector shape
 *
 * @param[in] config The distance detector config
 * @return The reflector shape
 */
acc_detector_distance_reflector_shape_t acc_detector_distance_config_reflector_shape_get(const acc_detector_distance_config_t *config);


/**
 * @}
 */

#endif
