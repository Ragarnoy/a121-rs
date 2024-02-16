// Copyright (c) Acconeer AB, 2021-2023
// All rights reserved

#ifndef ACC_CONFIG_SUBSWEEP_H_
#define ACC_CONFIG_SUBSWEEP_H_

#include <stdint.h>

#include "acc_config.h"
#include "acc_definitions_a121.h"

/**
 * @defgroup subsweep Subsweep
 * @ingroup config
 *
 * @brief Module to configure subsweeps
 *
 * @{
 */

/**
 * @brief Set the number of subsweeps to use
 *
 * @param[in] config The configuration
 * @param[in] num_subsweeps The number of subsweeps
 */
void acc_config_num_subsweeps_set(acc_config_t *config, uint8_t num_subsweeps);


/**
 * @brief Get the number of subsweeps to use
 *
 * @param[in] config The configuration
 * @return The number of subsweeps
 */
uint8_t acc_config_num_subsweeps_get(const acc_config_t *config);


/**
 * @brief Set the starting point of the sweep
 *
 * See @ref acc_config_start_point_set
 *
 * @param[in] config The configuration
 * @param[in] start_point The starting point of the sweep
 * @param[in] index The subsweep index
 */
void acc_config_subsweep_start_point_set(acc_config_t *config, int32_t start_point, uint8_t index);


/**
 * @brief Get the starting point of the sweep
 *
 * See @ref acc_config_start_point_get
 *
 * @param[in] config The configuration
 * @param[in] index The subsweep index
 * @return The starting point of the sweep
 */
int32_t acc_config_subsweep_start_point_get(const acc_config_t *config, uint8_t index);


/**
 * @brief Set the number of data points to measure
 *
 * See @ref acc_config_num_points_set
 *
 * @param[in] config The configuration
 * @param[in] num_points Number of data points to measure
 * @param[in] index The subsweep index
 */
void acc_config_subsweep_num_points_set(acc_config_t *config, uint16_t num_points, uint8_t index);


/**
 * @brief Get the number of data points to measure
 *
 * See @ref acc_config_num_points_get
 *
 * @param[in] config The configuration
 * @param[in] index The subsweep index
 * @return Number of data points to measure
 */
uint16_t acc_config_subsweep_num_points_get(const acc_config_t *config, uint8_t index);


/**
 * @brief Set the step length in a sweep
 *
 * See @ref acc_config_step_length_set
 *
 * @param[in] config The configuration
 * @param[in] step_length The step length
 * @param[in] index The subsweep index
 */
void acc_config_subsweep_step_length_set(acc_config_t *config, uint16_t step_length, uint8_t index);


/**
 * @brief Get the step length in a sweep
 *
 * See @ref acc_config_step_length_get
 *
 * @param[in] config The configuration
 * @param[in] index The subsweep index
 * @return The step length
 */
uint16_t acc_config_subsweep_step_length_get(const acc_config_t *config, uint8_t index);


/**
 * @brief Set a profile
 *
 * See @ref acc_config_profile_set
 *
 * @param[in] config The config to set a profile for
 * @param[in] profile The profile to set
 * @param[in] index The subsweep index
 */
void acc_config_subsweep_profile_set(acc_config_t *config, acc_config_profile_t profile, uint8_t index);


/**
 * @brief Get the currently used profile
 *
 * See @ref acc_config_profile_get
 *
 * @param[in] config The config to get a profile for
 * @param[in] index The subsweep index
 * @return The current profile, 0 if config is invalid
 */
acc_config_profile_t acc_config_subsweep_profile_get(const acc_config_t *config, uint8_t index);


/**
 * @brief Set the hardware accelerated average samples (HWAAS)
 *
 * See @ref acc_config_hwaas_set
 *
 * @param[in] config The config to set hwaas for
 * @param[in] hwaas Hardware accelerated average samples
 * @param[in] index The subsweep index
 */
void acc_config_subsweep_hwaas_set(acc_config_t *config, uint16_t hwaas, uint8_t index);


/**
 * @brief Get the hardware accelerated average samples (HWAAS)
 *
 * See @ref acc_config_hwaas_get
 *
 * @param[in] config The config to get hwaas from
 * @param[in] index The subsweep index
 * @return Hardware accelerated average samples
 */
uint16_t acc_config_subsweep_hwaas_get(const acc_config_t *config, uint8_t index);


/**
 * @brief Set receiver gain setting
 *
 * See @ref acc_config_receiver_gain_set
 *
 * @param[in] config The configuration
 * @param[in] gain Receiver gain setting
 * @param[in] index The subsweep index
 */
void acc_config_subsweep_receiver_gain_set(acc_config_t *config, uint8_t gain, uint8_t index);


/**
 * @brief Get receiver gain setting
 *
 * See @ref acc_config_receiver_gain_get
 *
 * @param[in] config The configuration
 * @param[in] index The subsweep index
 * @return Receiver gain setting
 */
uint8_t acc_config_subsweep_receiver_gain_get(const acc_config_t *config, uint8_t index);


/**
 * @brief Enable or disable the transmitter
 *
 * See @ref acc_config_enable_tx_set
 *
 * @param[in] config The configuration
 * @param[in] enable true to enable the transmitter
 * @param[in] index The subsweep index
 */
void acc_config_subsweep_enable_tx_set(acc_config_t *config, bool enable, uint8_t index);


/**
 * @brief Get transmitter enable mode
 *
 * See @ref acc_config_enable_tx_get
 *
 * @param[in] config The configuration
 * @param[in] index The subsweep index
 * @return true if the transmitter is enabled
 */
bool acc_config_subsweep_enable_tx_get(const acc_config_t *config, uint8_t index);


/**
 * @brief Set Pulse Repetition Frequency
 *
 * See @ref acc_config_prf_t for details.
 *
 * @param[in] config The configuration
 * @param[in] prf The Pulse Repetition Frequency to use
 * @param[in] index The subsweep index
 */
void acc_config_subsweep_prf_set(acc_config_t *config, acc_config_prf_t prf, uint8_t index);


/**
 * @brief Get Pulse Repetition Frequency
 *
 * See @ref acc_config_prf_t for details.
 *
 * @param[in] config The configuration
 * @return Pulse Repetition Frequency
 * @param[in] index The subsweep index
 */
acc_config_prf_t acc_config_subsweep_prf_get(const acc_config_t *config, uint8_t index);


/**
 * @brief Set the phase enhancement enabled configuration
 *
 * See @ref acc_config_phase_enhancement_set
 *
 * @param[in] config The configuration
 * @param[in] enable true if phase enhancement to be enabled, false otherwise
 * @param[in] index The subsweep index
 */
void acc_config_subsweep_phase_enhancement_set(acc_config_t *config, bool enable, uint8_t index);


/**
 * @brief Get the phase enhancement enabled configuration
 *
 * See @ref acc_config_phase_enhancement_get
 *
 * @param[in] config The configuration
 * @param[in] index The subsweep index
 * @return true if phase enhancement is enabled, false otherwise
 */
bool acc_config_subsweep_phase_enhancement_get(const acc_config_t *config, uint8_t index);


/**
 * @brief Set the loopback enabled configuration
 *
 * See @ref acc_config_enable_loopback_set
 *
 * @param[in] config The configuration
 * @param[in] enable true if loopback to be enabled, false otherwise
 * @param[in] index The subsweep index
 */
void acc_config_subsweep_enable_loopback_set(acc_config_t *config, bool enable, uint8_t index);


/**
 * @brief Get the enable loopback configuration
 *
 * See @ref acc_config_enable_loopback_get
 *
 * @param[in] config The configuration
 * @param[in] index The subsweep index
 * @return true if loopback is enabled, false otherwise
 */
bool acc_config_subsweep_enable_loopback_get(const acc_config_t *config, uint8_t index);


/**
 * @}
 */


#endif
