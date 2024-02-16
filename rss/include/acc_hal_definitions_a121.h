// Copyright (c) Acconeer AB, 2021-2023
// All rights reserved

#ifndef ACC_HAL_DEFINITIONS_A121_H_
#define ACC_HAL_DEFINITIONS_A121_H_

#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#include "acc_definitions_common.h"


/**
 * @brief Specifies the minimal size in bytes that SPI transfers must be able to handle
 */
#define ACC_HAL_SPI_TRANSFER_SIZE_REQUIRED 16U

/**
 * @brief Definition of a memory allocation function
 *
 * Allocated memory should be suitably aligned for any built-in type. Returning NULL is seen as failure.
 */
typedef void *(*acc_hal_mem_alloc_function_t)(size_t);


/**
 * @brief Definition of a memory free function
 *
 * Free memory which is previously allocated.
 */
typedef void (*acc_hal_mem_free_function_t)(void *);


/**
 * @brief Definition of a sensor transfer function
 *
 * This function shall transfer data to and from the sensor over spi. It's beneficial from a performance perspective
 * to use dma if available.
 * The buffer is naturally aligned to a maximum of 4 bytes.
 *
 */
typedef void (*acc_hal_sensor_transfer8_function_t)(acc_sensor_id_t sensor_id, uint8_t *buffer, size_t buffer_size);


/**
 * @brief Definition of an optimized 16-bit sensor transfer function
 *
 * This function shall transfer data to and from the sensor over spi with 16 bits data size.
 * It's beneficial from a performance perspective to use dma if available.
 * The buffer is naturally aligned to a minimum of 4 bytes.
 *
 * If defined it will supersede the normal 8-bit function @ref acc_hal_sensor_transfer8_function_t
 *
 */
typedef void (*acc_hal_sensor_transfer16_function_t)(acc_sensor_id_t sensor_id, uint16_t *buffer, size_t buffer_length);


/**
 * @brief This struct contains function pointers that are optional to support different optimizations
 *
 * Optional
 *
 * This struct contains function pointers to support different optimizations.
 * These optimizations can be utilized for some integrations.
 * If they are defined, they will override the corresponding non-optimized function.
 *
 * For example, if the transfer16 function is implemented, it will be used instead of the transfer function.
 */
typedef struct
{
	acc_hal_sensor_transfer16_function_t transfer16;
} acc_hal_optimization_t;


/**
 * @brief Definition of a log function
 */
typedef void (*acc_hal_log_function_t)(acc_log_level_t level, const char *module, const char *format, ...);

typedef struct
{
	uint16_t max_spi_transfer_size;

	acc_hal_mem_alloc_function_t        mem_alloc;
	acc_hal_mem_free_function_t         mem_free;
	acc_hal_sensor_transfer8_function_t transfer;
	acc_hal_log_function_t              log;
	acc_hal_optimization_t              optimization;
} acc_hal_a121_t;

#endif
