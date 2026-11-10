#ifndef _H_LIBLEAD_DEF
#define _H_LIBLEAD_DEF

#include <stdint.h>
#include <stdio.h>
#include <stdbool.h>

enum SCALARS {
    INTEGER = 'I',
    FLOAT   = 'F',
    BOOLEAN = 'B',
    CHAR    = 'C',
    BOX     = 'X',
};

typedef struct _Lead_List {
    char t;
    unsigned char* i;
    size_t c;
    size_t d;
    size_t* r;
} Lead_List;

#endif

