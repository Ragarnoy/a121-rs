// Copyright (c) Acconeer AB, 2022-2023
// All rights reserved

#ifndef ACC_DETECTOR_DISTANCE_DEFINITIONS_H_
#define ACC_DETECTOR_DISTANCE_DEFINITIONS_H_


#include <stdint.h>


/**
 * The size of the result from a completed calibration update.
 */
#define ACC_DETECTOR_CAL_RESULT_DYNAMIC_DATA_SIZE (8U)

/**
 * The result from a completed calibration update.
 */
typedef struct
{
	uint8_t data[ACC_DETECTOR_CAL_RESULT_DYNAMIC_DATA_SIZE];
} acc_detector_cal_result_dynamic_t;


/**
 * @brief Enum for peak sorting algorithms
 */
typedef enum
{
	/*! Return peaks with the closest detection first. */
	ACC_DETECTOR_DISTANCE_PEAK_SORTING_CLOSEST,
	/*! Return peaks with the peak with the highest RCS first. */
	ACC_DETECTOR_DISTANCE_PEAK_SORTING_STRONGEST,
} acc_detector_distance_peak_sorting_t;


/**
 * @brief Enum for threshold methods
 */
typedef enum
{
	/*! Compares processed data against a fixed amplitude value */
	ACC_DETECTOR_DISTANCE_THRESHOLD_METHOD_FIXED_AMPLITUDE,
	/*! Compares processed data against a fixed strength value */
	ACC_DETECTOR_DISTANCE_THRESHOLD_METHOD_FIXED_STRENGTH,
	/*! Compares processed data against a recorded threshold */
	ACC_DETECTOR_DISTANCE_THRESHOLD_METHOD_RECORDED,
	/*! Uses the CFAR algorithm as a threshold */
	ACC_DETECTOR_DISTANCE_THRESHOLD_METHOD_CFAR
} acc_detector_distance_threshold_method_t;


/**
 * @brief Enum for reflector shapes
 */
typedef enum
{
	/*! Use a generic reflector shape for RCS calculation */
	ACC_DETECTOR_DISTANCE_REFLECTOR_SHAPE_GENERIC,
	/*! Use a planar reflector shape for RCS calculation */
	ACC_DETECTOR_DISTANCE_REFLECTOR_SHAPE_PLANAR,
} acc_detector_distance_reflector_shape_t;


#endif
