/* vec_ops.c — C reference for benchmarks/vec_ops.ark (push/sum/contains, 1k ints). */
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>

static bool contains(const int *v, size_t n, int x) {
    for (size_t i = 0; i < n; i++) {
        if (v[i] == x) {
            return true;
        }
    }
    return false;
}

int main(void) {
    size_t cap = 16;
    size_t len = 0;
    int *v = malloc(cap * sizeof *v);
    if (!v) {
        return 1;
    }
    for (int i = 0; i < 1000; i++) {
        if (len >= cap) {
            cap *= 2;
            int *nv = realloc(v, cap * sizeof *nv);
            if (!nv) {
                free(v);
                return 1;
            }
            v = nv;
        }
        v[len++] = i;
    }
    printf("%zu\n", len);

    int sum = 0;
    for (size_t j = 0; j < len; j++) {
        sum += v[j];
    }
    printf("%d\n", sum);

    if (contains(v, len, 500)) {
        printf("found 500\n");
    } else {
        printf("not found 500\n");
    }
    if (contains(v, len, 9999)) {
        printf("found 9999\n");
    } else {
        printf("not found 9999\n");
    }
    free(v);
    return 0;
}
