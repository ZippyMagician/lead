#ifndef _H_LIBLEAD_DEF
#define _H_LIBLEAD_DEF

#include <stdint.h>
#include <stdio.h>
#include <stdbool.h>

enum SCALARS {
    INTEGER = 0,
    FLOAT   = 1,
    BOOLEAN = 2,
    CHAR    = 3,
    BOX     = 4,
};

typedef struct _Lead_Scalar {
    uint8_t t;
    union {
        int64_t n;
        double f;
        bool b;
        uint32_t c;
        struct _Lead_List* bx;
    } v;
} Lead_Scalar;

typedef struct _Lead_List {
    Lead_Scalar* i;
    size_t c;
    size_t d;
    size_t* r;
} Lead_List;

#endif

