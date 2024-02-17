// Copyright (c) Acconeer AB, 2022
// All rights reserved
// This file is subject to the terms and conditions defined in the file
// 'LICENSES/license_acconeer.txt', (BSD 3-Clause License) which is part
// of this source code package.

#ifndef ACC_CONTROL_HELPER_H_
#define ACC_CONTROL_HELPER_H_
#include <stdint.h>

#include "acc_config.h"
#include "acc_definitions_a121.h"
#include "acc_definitions_common.h"
#include "acc_processing.h"
#include "acc_sensor.h"

/** \example acc_control_helper.c
 * @brief This is a simplified API that can be used to easier get started
 * The implementation of this API is provided as source code which can
 * be examined and modified in order to suit your needs.
 */

typedef struct
{
	acc_config_t              *config;
	acc_sensor_t              *sensor;
	acc_sensor_id_t           sensor_id;
	acc_processing_t          *processing;
	void                      *buffer;
	uint32_t                  buffer_size;
	acc_cal_result_t          cal_result;
	acc_processing_metadata_t proc_meta;
	acc_processing_result_t   proc_result;
} acc_control_helper_t;


/**
 * @brief Create a helper instance
 *
 * After a successful call to this function all members of the the acc_control_helper_t
 * are initialized to default values and the config member is created.
 *
 * @param radar A pointer to an acc_control_helper_t struct. The members in this struct will be initialized.
 * @param sensor_id The sensor id
 * @return true if successful, false otherwise
 */
bool acc_control_helper_create(acc_control_helper_t *radar, acc_sensor_id_t sensor_id);


/**
 * @brief Destroy a helper instance
 *
 * @param radar A pointer to an acc_control_helper_t struct
 */
void acc_control_helper_destroy(acc_control_helper_t *radar);


/**
 * @brief Activate the sensor
 *
 * After a successful call to this function the following members of the
 * acc_control_helper_t struct are updated:
 *
 * buffer_size: The size of the allocated buffer
 * sensor: Pointer to sensor instance
 * processing: Pointer to processing instance
 * cal_result: The calibration data
 *
 * @param radar A pointer to an acc_control_helper_t struct
 * @return true if successful, false otherwise
 *
 */
bool acc_control_helper_activate(acc_control_helper_t *radar);


/**
 * @brief Perform a radar measurement and wait for the result.
 *
 * After each call to this function the "proc_result" member of the acc_control_helper_t
 * is updated.
 *
 * @param radar A pointer to an acc_control_helper_t struct
 * @return true if successful, false otherwise
 */
bool acc_control_helper_get_next(acc_control_helper_t *radar);


#endif
