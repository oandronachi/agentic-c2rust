#ifndef QOI_RS_H
#define QOI_RS_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct QoiRsDesc {
  uint32_t width;
  uint32_t height;
  uint8_t channels;
  uint8_t colorspace;
} QoiRsDesc;

/**
 * # Safety
 * `ptr` must point to `len` readable bytes. `desc` and `out_len` must be valid
 * non-null pointers. The returned pointer must be freed with `qoi_rs_free`.
 */
uint8_t *qoi_rs_encode(const uint8_t *ptr,
                       uintptr_t len,
                       const struct QoiRsDesc *desc,
                       uintptr_t *out_len);

/**
 * # Safety
 * `ptr` must point to `len` readable bytes. `out_desc` and `out_len` must be valid
 * non-null pointers. The returned pointer must be freed with `qoi_rs_free`.
 */
uint8_t *qoi_rs_decode(const uint8_t *ptr,
                       uintptr_t len,
                       uint8_t channels,
                       struct QoiRsDesc *out_desc,
                       uintptr_t *out_len);

/**
 * # Safety
 * `ptr` and `len` must be exactly the pointer and length returned by this library.
 */
void qoi_rs_free(uint8_t *ptr, uintptr_t len);

#endif  /* QOI_RS_H */
