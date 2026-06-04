/* bindgen entry point. QOI_NO_STDIO drops qoi_read/qoi_write so only the two
   in-memory functions (qoi_encode, qoi_decode) and qoi_desc are surfaced. The
   allowlist in build.rs narrows it further. */
#define QOI_NO_STDIO
#include "qoi.h"
