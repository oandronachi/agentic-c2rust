/* The single C translation unit for the reference implementation.
   QOI_IMPLEMENTATION pulls in the code; QOI_NO_STDIO omits the file-I/O helpers
   (qoi_read/qoi_write) so the only exported symbols are qoi_encode/qoi_decode. */
#define QOI_IMPLEMENTATION
#define QOI_NO_STDIO
#include "qoi.h"
