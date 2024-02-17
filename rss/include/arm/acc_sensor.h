// Copyright (c) Acconeer AB, 2020-2022
// All rights reserved

#ifndef ACC_SENSOR_H_
#define ACC_SENSOR_H_

#include <stdbool.h>
#include <stdint.h>

#include "acc_config.h"
#include "acc_definitions_a121.h"
#include "acc_definitions_common.h"


/**
 * @defgroup service Service
 *
 * @brief Service API
 *
 * @defgroup sensor Sensor
 * @ingroup service
 *
 * @brief Module to control the sensor
 *
 * @{
 */


struct acc_sensor;

typedef struct acc_sensor acc_sensor_t;


/**
 * @brief Create a sensor instance
 *
 * A sensor instance represents a physical radar sensor and handles the communication
 * with it.
 *
 * Before this function is called the sensor must be powered on and not used
 * in another sensor instance without a power or reset cycle between.
 *
 * @param[in] sensor_id The sensor id to be used to communicate with
 *
 * @return Sensor instance, NULL if sensor instance was not possible to create
 */
acc_sensor_t *acc_sensor_create(acc_sensor_id_t sensor_id);


/**
 * @brief Destroy a sensor instance freeing any resources allocated.
 *
 * @param[in] sensor The sensor instance to destroy, can be NULL
 */
void acc_sensor_destroy(acc_sensor_t *sensor);


/**
 * @brief Calibrate a sensor
 *
 * Note that the sensor must be powered on before calling this function.
 * To calibrate the sensor, call this function and wait for sensor interrupt,
 * repeat until calibration is complete (or fails).
 *
 * @param[in]  sensor The sensor instance to calibrate
 * @param[out] cal_complete True if calibration is complete
               False if caller should wait for interrupt and
               then call again
 * @param[out] cal_result The result after a completed calibration
 * @param[in]  buffer Memory used during calibration.
 *             A larger buffer might mean fewer transactions between host and sensor.
 *             The buffer will only be used during the calibration.
 *             The client has to make sure this buffer is suitably aligned for
 *             any built-in type.
 * @param[in]  buffer_size The size in bytes of the buffer, should be at least buffer_size
               from @ref acc_rss_get_buffer_size
 * @return true if successful, false otherwise
 */
bool acc_sensor_calibrate(acc_sensor_t *sensor, bool *cal_complete, acc_cal_result_t *cal_result,
                          void *buffer, uint32_t buffer_size);


/**
 * @brief Gets calibration information from a calibration result
 *
 * @param[in]  cal_result The calibration result
 * @param[out] cal_info The calibration information
 * @return true if successful, false otherwise
 */
bool acc_sensor_get_cal_info(const acc_cal_result_t *cal_result, acc_cal_info_t *cal_info);


/**
 * @brief Prepare a sensor to do a measurement
 *
 * It's possible to reconfigure the sensor by calling the function multiple times.
 *
 * Note:
 * - The sensor must be powered on when calling this function.
 * - The sensor must not be measuring when calling this function, if previous call was
 *   @ref acc_sensor_measure use @ref acc_hal_integration_wait_for_sensor_interrupt to
 *   wait for measurement to complete.
 * - Reconfiguring is not supported when double buffering is active, however enabling
 *   double buffering through reconfiguration is.
 *
 * @param[in] sensor The sensor instance to prepare
 * @param[in] config The configuration to prepare for
 * @param[in] cal_result The calibration result to prepare for
 * @param[in] buffer Memory used during preparation.
 *            A larger buffer might mean fewer transactions between host and sensor.
 *            The buffer will only be used during the duration of this call.
 *            The client has to make sure this buffer is suitably aligned for
 *            any built-in type.
 * @param[in] buffer_size The size in bytes of the buffer, should be at least buffer_size
 *            from @ref acc_rss_get_buffer_size
 * @return true if successful, false otherwise
 */
bool acc_sensor_prepare(acc_sensor_t *sensor, const acc_config_t *config, const acc_cal_result_t *cal_result,
                        void *buffer, uint32_t buffer_size);


/**
 * @brief Start a radar measurement with previously prepared configuration
 *
 * Note that the following preconditions apply
 *  - The sensor must be powered on
 *  - @ref acc_sensor_calibrate must have been called
 *  - @ref acc_sensor_prepare must have been called
 *
 * @param[in] sensor The sensor instance to measure with
 * @return true if successful, false otherwise
 */
bool acc_sensor_measure(acc_sensor_t *sensor);


/**
 * @brief Read out radar data
 *
 * Note that the following preconditions apply
 *  - The sensor must be powered on
 *  - @ref acc_sensor_measure must be called before each call to this function
 *  - The sensor interrupt must be active
 *
 * @param[in] sensor The sensor to read the radar data from
 * @param[in] buffer The buffer to read radar data into.
 *            The buffer will only be used during the duration of this call.
 *            The client has to make sure this buffer is suitably aligned for
 *            any built-in type.
 * @param[in] buffer_size The size in bytes of the buffer, should be at least buffer_size
 *            from @ref acc_rss_get_buffer_size
 * @return true if successful, false otherwise
 */
bool acc_sensor_read(const acc_sensor_t *sensor, void *buffer, uint32_t buffer_size);


/**
 * @brief Check if a sensor is connected and responsive
 *
 * Note that the sensor must be powered on before calling this function.
 *
 * @param[in] sensor_id The sensor id to be used to communicate with
 * @return true if it is possible to communicate with the sensor
 */
bool acc_sensor_connected(acc_sensor_id_t sensor_id);


/**
 * @brief Check the status of the sensor
 *
 * This function reads out the internal status from the sensor and prints it for debugging purposes.
 * It can for example be called when the function @ref acc_hal_integration_wait_for_sensor_interrupt()
 * fails. Note that the sensor must be powered on before calling this function.
 *
 * @param[in] sensor The sensor instance to get status from
 */
void acc_sensor_status(const acc_sensor_t *sensor);


/**
 * @brief Prepare sensor for entering hibernation
 *
 * Prepare sensor for entering hibernation.
 * Should be invoked prior to calling @ref acc_hal_integration_sensor_disable()
 *
 * @param[in] sensor The sensor to prepare for hibernation
 * @return True if prepare was successful
 */
bool acc_sensor_hibernate_on(acc_sensor_t *sensor);


/**
 * @brief Restore sensor after exiting hibernation
 *
 * Restore sensor after exiting hibernation.
 * Should be invoked after calling @ref acc_hal_integration_sensor_enable()
 *
 * @param[in] sensor The sensor to unprepare for hibernation
 * @return True if unprepare was successful
 */
bool acc_sensor_hibernate_off(const acc_sensor_t *sensor);


/**
 * @brief Validate calibration result
 *
 * @param[in] cal_result Result of a calibration
 *
 * @return True if calibration is valid
 */
bool acc_sensor_validate_calibration(const acc_cal_result_t *cal_result);


/**
 * @}
 */


#endif
