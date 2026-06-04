# Migration notes

## API inventory

The public C++ class being wrapped is `re2::RE2` from `re2/re2.h`.

Relevant C++ ownership facts from the pinned header:

- `RE2(const RE2&) = delete`
- `RE2& operator=(const RE2&) = delete`
- `RE2(RE2&&) = delete`
- `RE2& operator=(RE2&&) = delete`
- `RE2` is documented as thread-safe and logically immutable.

The workspace adds a tiny C ABI facade over the C++ object:

| Function | Direction and semantics |
| --- | --- |
| `re2_handle_new(pattern, len)` | Allocates a C++ `Re2Handle` containing `std::unique_ptr<re2::RE2>`. Returns null only for invalid input/allocation failure. Invalid regex syntax still returns a handle so the caller can read the RE2 error string. |
| `re2_handle_free(handle)` | Deletes the C++ handle. Must be called exactly once for each successful allocation. |
| `re2_handle_ok(handle)` | Reports RE2 compile success. Null-safe and returns false for null. |
| `re2_handle_error(handle, len)` | Borrows RE2's copied error string from the handle and returns its length. The pointer becomes invalid after `re2_handle_free`. |
| `re2_handle_partial_match(handle, text, len)` | Calls `RE2::PartialMatch` on a valid compiled regex. Null-safe and returns false for null/invalid handles. |

## Ownership

| Pointer/resource | Owner | Rule |
| --- | --- | --- |
| `Re2Handle*` | C++ facade | Created by `re2_handle_new`; destroyed only by `re2_handle_free`. |
| `re2::RE2` | `std::unique_ptr` inside `Re2Handle` | Never exposed by value to Rust; never copied or moved. |
| `const char* pattern/text` | Caller | Borrowed only for the duration of the call with an explicit length. |
| Error string pointer | `Re2Handle` | Borrowed; Rust copies it immediately into an owned `String`. |

## Edge cases

- Empty pattern is valid.
- Invalid pattern returns an owned handle with `ok=false` and a copied error string.
- Empty text is valid; null text with nonzero length is rejected by the facade.
- Patterns/text are `&str` in Rust; arbitrary fuzz bytes are coerced with UTF-8
  lossy conversion for no-panic testing.
- The behavioral comparison only uses escaped ASCII literals to stay inside the
  documented common syntax subset of RE2 and Rust `regex`.
