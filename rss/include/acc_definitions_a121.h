// Copyright (c) Acconeer AB, 2022-2024
// All rights reserved

#ifndef ACC_DEFINITIONS_A121_H_
#define ACC_DEFINITIONS_A121_H_

#include <stdint.h>


/**
 * @defgroup definitions Definitions
 *
 * @brief Various definitions and types used in the RSS API
 *
 * @{
 */


/**
 * @brief The size of a sensor calibration result
 */
#define ACC_CAL_RESULT_DATA_SIZE (192)

/**
 * @brief The maximum number of subsweeps in a configuration.
 */
#define ACC_MAX_NUM_SUBSWEEPS (4U)

/**
 * @brief The result from a completed calibration.
 */
typedef struct
{
	uint8_t data[ACC_CAL_RESULT_DATA_SIZE];
} acc_cal_result_t;

/**
 * @brief Information about calibration.
 */
typedef struct
{
	int16_t temperature;
} acc_cal_info_t;


/**
 * @brief Profile
 *
 * Each profile consists of a number of settings for the sensor that configures the RX and TX paths.
 * Lower profiles have higher depth resolution while higher profiles have higher radar loop gain.
 */
typedef enum
{
	/*! The profile with the highest depth resolution and lowest radar loop gain. */
	ACC_CONFIG_PROFILE_1 = 1,
	ACC_CONFIG_PROFILE_2,
	ACC_CONFIG_PROFILE_3,
	ACC_CONFIG_PROFILE_4,
	/*! The profile with the lowest depth resolution and highest radar loop gain. */
	ACC_CONFIG_PROFILE_5,
} acc_config_profile_t;


/**
 * @brief Idle state
 *
 * Idle state 'DEEP_SLEEP' is the deepest state where as much of the sensor hardware as
 * possible is shut down and idle state 'READY' is the shallowest state where most of the sensor
 * hardware is kept on.
 *
 * DEEP_SLEEP is the slowest to transition from while READY is the fastest.
 *
 */
typedef enum
{
	/*! The deepest state where as much of the sensor hardware is shut down. */
	ACC_CONFIG_IDLE_STATE_DEEP_SLEEP,
	ACC_CONFIG_IDLE_STATE_SLEEP,
	/*! The shallowest state where most of the sensor hardware is kept on. */
	ACC_CONFIG_IDLE_STATE_READY,
} acc_config_idle_state_t;


/**
 * @brief Pulse Repetition Frequency
 *
 * Pulse Repetition Frequency, PRF, is the frequency at
 * which pulses are sent out from the radar system. The
 * measurement time is approximately proportional to the
 * PRF. The higher the PRF, the shorter the measurement time.
 *
 * This parameter sets the Maximum Measurable Distance, MMD,
 * that can be achieved. MMD is the maximum value for the end point,
 * i.e.,the start point + (number of points * step length).
 * For example, an MMD of 7.0 m means that the range cannot
 * be set further out than 7.0 m.
 *
 * It also sets the Maximum Unambiguous Range, MUR, that can be achieved.
 * MUR is the maximum distance at which an object can be located to guarantee
 * that its reflection corresponds to the most recent transmitted pulse.
 * Objects farther away than the MUR may fold into the measured range.
 * For example, with a MUR of 11.5 m, an object at 13.5 m could become
 * visible at 2 m.
 *
 * | PRF Setting              |      PRF |    MMD |    MUR |
 * |-------------------------:|---------:|-------:|-------:|
 * | ACC_CONFIG_PRF_19_5_MHZ* | 19.5 MHz |  3.1 m |  7.7 m |
 * | ACC_CONFIG_PRF_15_6_MHZ  | 15.6 MHz |  5.1 m |  9.6 m |
 * | ACC_CONFIG_PRF_13_0_MHZ  | 13.0 MHz |  7.0 m | 11.5 m |
 * | ACC_CONFIG_PRF_8_7_MHZ   |  8.7 MHz | 12.7 m | 17.3 m |
 * | ACC_CONFIG_PRF_6_5_MHZ   |  6.5 MHz | 18.5 m | 23.1 m |
 * | ACC_CONFIG_PRF_5_2_MHZ   |  5.2 MHz | 24.3 m | 28.8 m |
 *
 * *19.5MHz is only available for profile 1.
 */
typedef enum
{
	/*! 19.5 MHz */
	ACC_CONFIG_PRF_19_5_MHZ,
	/*! 15.6 MHz */
	ACC_CONFIG_PRF_15_6_MHZ,
	/*! 13.0 MHz */
	ACC_CONFIG_PRF_13_0_MHZ,
	/*! 8.7 MHz */
	ACC_CONFIG_PRF_8_7_MHZ,
	/*! 6.5 MHz */
	ACC_CONFIG_PRF_6_5_MHZ,
	/*! 5.2 MHz */
	ACC_CONFIG_PRF_5_2_MHZ,
} acc_config_prf_t;


/**
 * @}
 */


#endif
