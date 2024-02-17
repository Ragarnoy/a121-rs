// Copyright (c) Acconeer AB, 2021-2022
// All rights reserved

#ifndef ACC_HAL_INTEGRATION_A121_H_
#define ACC_HAL_INTEGRATION_A121_H_

#include <stdbool.h>
#include <stdint.h>

#include "acc_definitions_common.h"
#include "acc_hal_definitions_a121.h"


/**
 * @brief Get hal implementation reference
 */
const acc_hal_a121_t *acc_hal_rss_integration_get_implementation(void);


/**
 * @brief Power on sensor supply
 *
 * @param[in] sensor_id The id of the sensor to power on
 */
void acc_hal_integration_sensor_supply_on(acc_sensor_id_t sensor_id);


/**
 * @brief Power off sensor supply
 *
 * @param[in] sensor_id The id of the sensor to power off
 */
void acc_hal_integration_sensor_supply_off(acc_sensor_id_t sensor_id);


/**
 * @brief Enable sensor
 *
 * Any pending sensor interrupts should be cleared before returning from function.
 * The sensor supply needs to be enabled by invoking @ref acc_hal_integration_sensor_supply_on
 * before calling this function.
 *
 * @param[in] sensor_id The id of the sensor to enable
 */
void acc_hal_integration_sensor_enable(acc_sensor_id_t sensor_id);


/**
 * @brief Disable sensor
 *
 * @param[in] sensor_id The id of the sensor to disable
 */
void acc_hal_integration_sensor_disable(acc_sensor_id_t sensor_id);


/**
 * @brief Wait for a sensor interrupt
 *
 * @param[in] sensor_id The sensor to wait for the interrupt on
 * @param[in] timeout_ms The maximum time to wait in milliseconds
 * @return True if an interrupt has occurred within timeout, false if timeout occurred
 */
bool acc_hal_integration_wait_for_sensor_interrupt(acc_sensor_id_t sensor_id, uint32_t timeout_ms);


/**
 * @brief Get the max number of sensors the integration supports
 *
 * @return The max sensor count
 */
uint16_t acc_hal_integration_sensor_count(void);


#endif
