// gcc -c -fpic test/external.c -Iffi -o test/external.o
// gcc -shared -o test/libexternal.so external.o

#include "liblead.h"
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>

#define max(a,b) \
    ({ __typeof__ (a) _a = (a); \
           __typeof__ (b) _b = (b); \
         _a > _b ? _a : _b; })

Lead_List* test_list(Lead_List* a, Lead_List* b) {
    if (a->t != INTEGER || b->t != INTEGER) {
        printf("Not integers.\n");
        return NULL;
    }

    Lead_List* r = malloc(sizeof(Lead_List));
    size_t maximum = max(a->c, b->c);

    r->t = INTEGER;
    r->i = calloc(maximum, sizeof(int64_t));
    r->c = maximum;

    const int64_t* as = (int64_t*)a->i;
    const int64_t* bs = (int64_t*)b->i;
    int64_t* out = (int64_t*)r->i;
    for (uint64_t i = 0; i < maximum; i++) {
        if (i < a->c) out[i] += as[i];
        if (i < b->c) out[i] += bs[i];
    }

    r->d = 1;
    r->r = malloc(sizeof(size_t) * 1);
    *r->r = r->c;
    return r;
}

