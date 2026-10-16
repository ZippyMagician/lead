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
    Lead_List* r = malloc(sizeof(Lead_List));
    size_t maximum = max(a->c, b->c);

    r->i = malloc(sizeof(Lead_Scalar) * maximum);
    r->c = maximum;

    for (uint64_t i = 0; i < maximum; i++) {
        (r->i + i)->t = INTEGER;
        (r->i + i)->v.n = 0;

        Lead_Scalar* ah = a->i + i;
        Lead_Scalar* bh = b->i + i;
        // if within and integer, add
        if (i < a->c && ah->t == INTEGER) (r->i + i)->v.n += ah->v.n;
        if (i < b->c && ah->t == INTEGER) (r->i + i)->v.n += bh->v.n;
    }

    if (r->c % 2 == 0) {
        r->d = 2;
        r->r = malloc(sizeof(size_t) * 2);
        r->r[0] = 2;
        r->r[1] = r->c / 2;
    } else {
        r->d = 1;
        r->r = malloc(sizeof(size_t) * 1);
        *r->r = r->c;
    }

    return r;
}

