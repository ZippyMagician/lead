#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

enum ScalarExtern_Tag
#if __STDC_VERSION__ >= 202311L
  : uint8_t
#endif // __STDC_VERSION__ >= 202311L
 {
  Number,
  Float,
  Boolean,
  Char,
  Box,
};
#if __STDC_VERSION__ >= 202311L
typedef enum ScalarExtern_Tag ScalarExtern_Tag;
#else
typedef uint8_t ScalarExtern_Tag;
#endif // __STDC_VERSION__ >= 202311L

typedef struct ScalarExtern {
  ScalarExtern_Tag tag;
  union {
    struct {
      int64_t number;
    };
    struct {
      double float_;
    };
    struct {
      bool boolean;
    };
    struct {
      uint32_t char_;
    };
    struct {
      struct Window *box;
    };
  };
} ScalarExtern;

typedef struct Window {
  struct ScalarExtern *i;
  uintptr_t c;
  uintptr_t d;
  uintptr_t *r;
} Window;

#define BOXES_TL { U'\U0000250C', U'\U0000251C', U'\U0000250C', U'\U0000251C', U'\U0000250C', U'\U0000251C', U'\U0000250C', U'\U0000251C', U'\U0000252C', U'\U0000253C', U'\U0000252C', U'\U0000253C', U'\U0000252C', U'\U0000253C', U'\U0000252C', U'\U0000253C', }

#define BOXES_TR { U'\U00002510', U'\U00002524', U'\U0000252C', U'\U0000253C', U'\U00002510', U'\U00002524', U'\U0000252C', U'\U0000253C', U'\U00002510', U'\U00002524', U'\U0000252C', U'\U0000253C', U'\U00002510', U'\U00002524', U'\U0000252C', U'\U0000253C', }

#define BOXES_BR { U'\U00002518', U'\U00002518', U'\U00002534', U'\U00002534', U'\U00002524', U'\U00002524', U'\U0000253C', U'\U0000253C', U'\U00002518', U'\U00002518', U'\U00002534', U'\U00002534', U'\U00002524', U'\U00002524', U'\U0000253C', U'\U0000253C', }

#define BOXES_BL { U'\U00002514', U'\U00002514', U'\U00002514', U'\U00002514', U'\U0000251C', U'\U0000251C', U'\U0000251C', U'\U0000251C', U'\U00002534', U'\U00002534', U'\U00002534', U'\U00002534', U'\U0000253C', U'\U0000253C', U'\U0000253C', U'\U0000253C', }

void lead_free_list(struct Window *a);
