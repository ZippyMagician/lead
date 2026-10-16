// gcc -c -fpic test/external.c -Iffi -o test/external.o
// gcc -shared -o test/libexternal.so external.o

#include "liblead.h"
#include <stdio.h>

void test_list(Lead_List* a, Lead_List* b) {
    printf("Hello from FFI\n");
    printf("NUMS: %d %d\n", a->i->v, b->i->v);

    Lead_Scalar* first_l = a->i;
    Lead_Scalar* first_r = b->i;

    *(a->i) = *first_r;
    b->i->t = FLOAT;
    b->i->v.f = -3.4;
}

