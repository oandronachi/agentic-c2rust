/* hyperfine subject — C side. Compiled at -O3 against the vendored reference.
 * Does identical work to crates/qoi-rs/examples/bench_bin.rs (same image, same
 * encode+decode loop, same checksum) so the two are a fair whole-process race and
 * the printed checksums must match (also defeats dead-code elimination).
 *
 * Usage: qoi_cbench <width> <height> <channels> <iters>
 */
#define QOI_IMPLEMENTATION
#define QOI_NO_STDIO
#include "qoi.h"

#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    unsigned width = argc > 1 ? (unsigned)strtoul(argv[1], NULL, 10) : 512;
    unsigned height = argc > 2 ? (unsigned)strtoul(argv[2], NULL, 10) : 512;
    int channels = argc > 3 ? atoi(argv[3]) : 4;
    long iters = argc > 4 ? strtol(argv[4], NULL, 10) : 50;

    size_t npx = (size_t)width * height;
    size_t n = npx * (size_t)channels;
    unsigned char *img = (unsigned char *)malloc(n);
    if (!img) { fprintf(stderr, "alloc failed\n"); return 2; }
    for (size_t i = 0; i < npx; i++) {
        img[i * channels + 0] = (unsigned char)(i * 3);
        img[i * channels + 1] = (unsigned char)(i * 7);
        img[i * channels + 2] = (unsigned char)(i * 11);
        if (channels == 4) img[i * channels + 3] = 255;
    }

    qoi_desc desc;
    desc.width = width;
    desc.height = height;
    desc.channels = (unsigned char)channels;
    desc.colorspace = 0;

    unsigned long long acc = 0;
    int enc_len = 0;
    for (long it = 0; it < iters; it++) {
        void *enc = qoi_encode(img, &desc, &enc_len);
        if (!enc) { fprintf(stderr, "encode failed\n"); free(img); return 1; }
        unsigned char *e = (unsigned char *)enc;
        for (int i = 0; i < enc_len; i++) acc += e[i];

        qoi_desc d2;
        void *dec = qoi_decode(enc, enc_len, &d2, 0);
        if (!dec) { fprintf(stderr, "decode failed\n"); free(enc); free(img); return 1; }
        unsigned char *p = (unsigned char *)dec;
        for (size_t i = 0; i < n; i++) acc += p[i];

        free(enc);
        free(dec);
    }
    free(img);
    printf("checksum=%llu enc_len=%d\n", acc, enc_len);
    return 0;
}
