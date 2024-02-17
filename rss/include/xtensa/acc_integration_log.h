// Copyright (c) Acconeer AB, 2016-2021
// All rights reserved

#ifndef ACC_INTEGRATION_LOG_H_
#define ACC_INTEGRATION_LOG_H_

#include "acc_definitions_common.h"
#include "acc_integration.h"

#ifdef ACC_LOG_RSS_H_
#error "acc_integration_log.h and acc_log_rss.h cannot coexist"
#endif

#define ACC_LOG(level, ...) acc_integration_log(level, MODULE, __VA_ARGS__)

#define ACC_LOG_ERROR(...)   ACC_LOG(ACC_LOG_LEVEL_ERROR, __VA_ARGS__)
#define ACC_LOG_WARNING(...) ACC_LOG(ACC_LOG_LEVEL_WARNING, __VA_ARGS__)
#define ACC_LOG_INFO(...)    ACC_LOG(ACC_LOG_LEVEL_INFO, __VA_ARGS__)
#define ACC_LOG_VERBOSE(...) ACC_LOG(ACC_LOG_LEVEL_VERBOSE, __VA_ARGS__)
#define ACC_LOG_DEBUG(...)   ACC_LOG(ACC_LOG_LEVEL_DEBUG, __VA_ARGS__)

#define ACC_LOG_SIGN(a)      (((a) < 0.0f) ? (-1.0f) : (1.0f))
#define ACC_LOG_FLOAT_INT(a) ((unsigned long int)((a) + 0.0000005f))
#define ACC_LOG_FLOAT_DEC(a) (unsigned long int)((1000000.0f * (((a) + 0.0000005f) - ((unsigned int)((a) + 0.0000005f)))))

#define ACC_LOG_FLOAT_TO_INTEGER(a) (((a) < 0.0f) ? "-" : ""), ACC_LOG_FLOAT_INT((a) * ACC_LOG_SIGN(a)), ACC_LOG_FLOAT_DEC((a) * ACC_LOG_SIGN(a))

/**
 * @brief Specifier for printing float type using integers.
 */
#define PRIfloat "s%lu.%06lu"


#if defined(__GNUC__)
#define PRINTF_ATTRIBUTE_CHECK(a, b) __attribute__((format(printf, a, b)))
#else
#define PRINTF_ATTRIBUTE_CHECK(a, b)
#endif


/**
 * @brief Log function
 *
 * This log function can be used as a complement to for example printf.
 * It adds useful information to the log such as time and log level
 *
 * @param[in] level The severity level for the log
 * @param[in] module The name of the SW module from where the log is called
 * @param[in] format The information to be logged, same format as for printf
 */
void acc_integration_log(acc_log_level_t level, const char *module, const char *format, ...) PRINTF_ATTRIBUTE_CHECK(3, 4);


#endif
