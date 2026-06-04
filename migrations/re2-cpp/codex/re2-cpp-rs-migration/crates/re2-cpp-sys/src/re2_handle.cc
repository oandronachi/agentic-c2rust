#include "re2_handle.h"

#include <memory>
#include <new>
#include <string>

#include <re2/re2.h>

struct Re2Handle {
  std::unique_ptr<re2::RE2> re;
  std::string error;
};

Re2Handle *re2_handle_new(const char *pattern, size_t pattern_len) {
  if (pattern == nullptr && pattern_len != 0) {
    return nullptr;
  }

  try {
    auto handle = std::make_unique<Re2Handle>();
    const char *pattern_ptr = pattern == nullptr ? "" : pattern;
    handle->re = std::make_unique<re2::RE2>(
        re2::StringPiece(pattern_ptr, pattern_len), re2::RE2::Quiet);
    if (!handle->re->ok()) {
      handle->error = handle->re->error();
    }
    return handle.release();
  } catch (const std::bad_alloc &) {
    return nullptr;
  }
}

void re2_handle_free(Re2Handle *handle) { delete handle; }

bool re2_handle_ok(const Re2Handle *handle) {
  return handle != nullptr && handle->re != nullptr && handle->re->ok();
}

const char *re2_handle_error(const Re2Handle *handle, size_t *len) {
  if (len == nullptr) {
    return nullptr;
  }
  if (handle == nullptr) {
    *len = 0;
    return nullptr;
  }

  *len = handle->error.size();
  return handle->error.data();
}

bool re2_handle_partial_match(const Re2Handle *handle, const char *text,
                              size_t text_len) {
  if (!re2_handle_ok(handle)) {
    return false;
  }
  if (text == nullptr && text_len != 0) {
    return false;
  }

  const char *text_ptr = text == nullptr ? "" : text;
  return re2::RE2::PartialMatch(re2::StringPiece(text_ptr, text_len),
                                *handle->re);
}
