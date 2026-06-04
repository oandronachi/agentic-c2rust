#define QOI_IMPLEMENTATION
#define QOI_NO_STDIO
#include "../vendor/qoi/qoi.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static uint64_t checksum(const unsigned char *data, int len) {
    uint64_t h = 0xcbf29ce484222325ULL;
    for (int i = 0; i < len; i++) {
        h ^= data[i];
        h *= 0x100000001b3ULL;
    }
    return h;
}

int main(int argc, char **argv) {
    const char *mode = argc > 1 ? argv[1] : "encode";
    int w = argc > 2 ? atoi(argv[2]) : 256;
    int h = argc > 3 ? atoi(argv[3]) : 256;
    int ch = argc > 4 ? atoi(argv[4]) : 4;
    int iters = argc > 5 ? atoi(argv[5]) : 20;
    int len = w * h * ch;
    unsigned char *pixels = (unsigned char *)malloc((size_t)len);
    if (!pixels) return 2;
    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            uint32_t v = (uint32_t)x * 31u + (uint32_t)y * 17u;
            int p = (y * w + x) * ch;
            pixels[p + 0] = (unsigned char)v;
            pixels[p + 1] = (unsigned char)(v >> 3);
            pixels[p + 2] = (unsigned char)(v >> 7);
            if (ch == 4) pixels[p + 3] = 255;
        }
    }

    qoi_desc desc = { (unsigned int)w, (unsigned int)h, ch, 0 };
    int encoded_len = 0;
    void *encoded = qoi_encode(pixels, &desc, &encoded_len);
    if (!encoded) return 3;
    uint64_t sum = 0;
    int out_len = 0;
    for (int i = 0; i < iters; i++) {
        if (strcmp(mode, "roundtrip") == 0) {
            qoi_desc out_desc;
            void *decoded = qoi_decode(encoded, encoded_len, &out_desc, 0);
            if (!decoded) return 4;
            out_len = out_desc.width * out_desc.height * out_desc.channels;
            sum ^= checksum((const unsigned char *)decoded, out_len);
            free(decoded);
        } else {
            int n = 0;
            void *out = qoi_encode(pixels, &desc, &n);
            if (!out) return 5;
            out_len = n;
            sum ^= checksum((const unsigned char *)out, n);
            free(out);
        }
    }
    printf("checksum=%016llx out_len=%d\n", (unsigned long long)sum, out_len);
    free(encoded);
    free(pixels);
    return 0;
}
