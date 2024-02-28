// Copyright (c) Acconeer AB, 2016-2023
// All rights reserved

#ifndef ACC_ALG_BASIC_UTILS_H_
#define ACC_ALG_BASIC_UTILS_H_

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define ACC_ALG_SPEED_OF_LIGHT 299792458.0f

#ifndef M_PI
	#define M_PI 3.14159265358979323846
#endif

/*
 * The mathematical constant pi.
 */
#define ACC_ALG_BASIC_MATH_PI M_PI


/**
 * @brief Calculate CRC32 checksum on byte array
 *
 * @param[in] input byte array
 * @param[in] len Length of byte array
 *
 * @return CRC32 checksum
 */
uint32_t acc_alg_basic_util_crc32(const uint8_t *input, size_t len);


/**
 * @brief Calculate length of 32-bit array to contain size number of bits
 *
 * @param number_of_bits Number of bits to contain in bit array
 * @return Length of 32-bit array
 */
static inline size_t acc_alg_basic_utils_calculate_length_of_bitarray_uint32(size_t number_of_bits)
{
	return (number_of_bits + (32U - 1U)) / 32U;
}


/**
 * @brief Set bit in bit array
 *
 * @param[in, out] bitarray Array to set bit in
 * @param[in] bit_index Index of bit to set
 */
static inline void acc_alg_basic_utils_set_bit_bitarray_uint32(uint32_t *bitarray, size_t bit_index)
{
	bitarray[bit_index / 32U] |= (uint32_t)1U << (bit_index & 0x1FU);
}


/**
 * @brief Clear bit in bit array
 *
 * @param[in, out] bitarray Array to clear bit in
 * @param[in] bit_index Index of bit to clear
 */
static inline void acc_alg_basic_utils_clear_bit_bitarray_uint32(uint32_t *bitarray, size_t bit_index)
{
	bitarray[bit_index / 32U] &= ~((uint32_t)1U << (bit_index & 0x1FU));
}


/**
 * @brief Check if bit is set in bit array
 *
 * @param[in] bitarray Array to check bit in
 * @param[in] bit_index Index of bit to check
 * @return True if bit is set
 */
static inline bool acc_alg_basic_utils_is_bit_set_bitarray_uint32(const uint32_t *bitarray, size_t bit_index)
{
	return (bitarray[bit_index / 32U] & ((uint32_t)1U << (bit_index & 0x1FU))) != 0U;
}


#endif
