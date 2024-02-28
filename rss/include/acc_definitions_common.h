// Copyright (c) Acconeer AB, 2018-2023
// All rights reserved

#ifndef ACC_DEFINITIONS_COMMON_H_
#define ACC_DEFINITIONS_COMMON_H_

#include <inttypes.h>
#include <stdint.h>


/**
 * @brief Type representing a sensor ID
 */
typedef uint32_t acc_sensor_id_t;

/**
 * @brief Macro for printing sensor id
 */
#define PRIsensor_id PRIu32


/**
 * @brief This enum represents the different log levels for RSS
 */
typedef enum
{
	/*! ERROR log level. */
	ACC_LOG_LEVEL_ERROR,
	/*! WARNING log level. */
	ACC_LOG_LEVEL_WARNING,
	/*! INFO log level. */
	ACC_LOG_LEVEL_INFO,
	/*! VERBOSE log level. */
	ACC_LOG_LEVEL_VERBOSE,
	/*! DEBUG log level. */
	ACC_LOG_LEVEL_DEBUG
} acc_log_level_t;


/**
 * @brief Data type for interger-based representation of complex numbers
 */
typedef struct
{
	int16_t real;
	int16_t imag;
} acc_int16_complex_t;


#endif
