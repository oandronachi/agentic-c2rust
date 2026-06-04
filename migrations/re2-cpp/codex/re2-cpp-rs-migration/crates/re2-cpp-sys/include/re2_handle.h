#pragma once

#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct Re2Handle Re2Handle;

Re2Handle *re2_handle_new(const char *pattern, size_t pattern_len);
void re2_handle_free(Re2Handle *handle);
bool re2_handle_ok(const Re2Handle *handle);
const char *re2_handle_error(const Re2Handle *handle, size_t *len);
bool re2_handle_partial_match(const Re2Handle *handle, const char *text, size_t text_len);

#ifdef __cplusplus
}
#endif
