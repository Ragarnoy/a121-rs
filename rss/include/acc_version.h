// Copyright (c) Acconeer AB, 2019-2023
// All rights reserved

#ifndef ACC_VERSION_H_
#define ACC_VERSION_H_

#include <stdint.h>

/**
 * @brief Get the version of the Acconeer software
 *
 * @return A string describing the software version.
 */
const char *acc_version_get(void);


/**
 * @brief Get the version of the Acconeer software as a hex number
 *
 * @return An uint32 number, 0xMMMMmmPP where M is major, m is minor and P is patch
 */
uint32_t acc_version_get_hex(void);


#endif
