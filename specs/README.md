# lead specs
WIP general documentation + specs for lead and it's differences with `ayr`.

## Table of Contents
* [Array model](#array-model)
* [Basic syntax](#basic-syntax)
* [User functions](#user-functions)
* [Trains](#trains)
* [Symbols](#symbols)
* [Binders](#binders)
* [Library builtins](#library-builtins)
* [FFI](#ffi)

## Array Model
`lead` uses the boxed array model, similar to [J](https://jsoftware.com), although there are a few implementation specific differences.

A **Value** can be either a **Primitive** (i.e. a **List**), or a **Map**. Primitive Lists can contain *Numerics*, *Booleans*, *Chars*, or *Boxes*. Boxes are treated internally
as pointers to a **Type**, which is either a **Value** or **Function**. A Numeric list contains *Integers* and/or *Floats* interchangeably. A *Char* List is treated as a String.

Similarly, **Maps** contain keys and values, which are Strings and *Boxes* respectively.

**Lists** can be of multiple dimensions. For instance, a 3-length array, a 2 x 2 matrix, etc.

### Example Code
```
    <2 3 6
[ 2 3 6 ]
    1
1
    1 2 3 ; _4 5 11
 1  2  3
_4  5 11
```

## Basic syntax
General types come in the form of signed numerics and strings, which may or may not be scalar values. A list, similar to **APL**, is denoted with a series of these general values separated by spaces.

Incomplete operations within groups **or** assigned to a variable are called [Trains](#trains). Binders/Operators are left-assosciative, while Symbols/Functions are right-assosciative.

lead also supports block syntax, which can take one or two variables. The right variable is `y`, and the left is `x`. For example:
```
    {{ 3 * y }} 4
12
```
Additionally, blocks can recurse via the variable `v`, which refers to themselves. These can be used in any place any symbol or group could also be used.

A trailing, unmatched `)` can denote a newline, while currently `--` is used to denote comments.

lead supports a feature called **Tags**, which are marked with square brackets surrounding a single token. For instance:
```
    [`B] 1 0 0 0 2 7 0    -- Cast primitive to boolean list
1 0 0 0 1 1 0
    [~] 1 + 10            -- Silent, discards value
    add: [{] x + y        -- Anonymous, one line block
```
As can be seen, some tags are used to denote syntactic meaning, while others are similar to functions, albeit not usable in trains.

Another use case of this is for the definition of maps. For example:
```
    map: [:] { a: 'aardvark'; word: 'stout' }
    map.word
stout
```
A trailing dot token following a literal is marked as an accessor, which allows one to index into maps.
Currently this feature is not variadic. Similarly, some functions will also work on maps, although not all do accept one as an argument.

### Data Types
#### Numbers
`_` represents a negative sign. Literal floats and scientific notation are supported. For example,
```
    e2
100
    _3.76
_3.76
```

#### Strings
As stated earlier, strings are simply primitive lists of chars. `'hello world' + 3` will print a string where each character is offset from 3 by the string `hello world`. 
Character matrices and higher rank data is also supported. To display a string in numeric form, you can use the `+` prefix, which monadically will cast chars to integers.

Additionally, the **`** character can denote char literals, specifically when the next character is alphanumeric. For instance,
```
    `C
C
```

#### General Lists
Each list has a rank, which denotes its shape. For instance, the 3 x 2 primitive list:
```
0 0 0
0 0 0
```
As list gains dimensions, these are represented by a new number being appended to the rank, representing the size in that dimension. This array has `3` rows and `2` columns<br>
These could also be referred to as **axes**: the first axes is the newest dimension; in this case being the *rows*. A one dimensional array's first axes would be its *columns*.

As in J, the rank grows outwards. The first value will always be the first axes, the second will always be the second, and so on. In pure mathematics, the above list would be considered 2 x 3, for example.

### Symbols and Operators
A symbol refers to a built in ascii symbol that denotes a monadic/dyadic function. An operator, or binder, refers to a symbol that monadically and/or dyadically takes a symbol and does something with them. 
For example, take the `+` (add) symbol, `-` (subtract/negate) symbol, `>` (first) symbol, and the `&` (atop) operator:

```
    1 2 + 3
4 5
    > 1 2 3
1
    1 2 3 -&+ 4 5 6
_5 _7 _9
```
Some symbols feature overloads dependent on the types passed. For instance, monadic `+` is both absolute value, and a numeric cast.
`-` monadically is negation, boolean negation, and swaps the case of characters.

The same is true for operators, `&` is *bind* when provided an instant as argument (List or Map).

### Control Flow
Basic control flow can be achieved in ayr through the use of `:`. In general,
```
(COND) : IF
ELSE
```
is equivalent to JS'
```
if (COND) { IF }
else { ELSE }
```

## User Functions
User functions refer to, as the name implies, functions defined by the user. For instance,
```
    add: +
    4 add 5
9
```
They are defined with the syntax `<name> : <expression>`. These can be constant values, incomplete trains, blocks, etc.
All function definitions are one line by default, so a block is necessary to create larger function bodies.

## Trains
There are two types of trains: partial trains and J-style trains. First the default, J-style:<br>
*Key: lowercase letters represent symbols and uppercase represent immediates*

```
(f g) A          -> f g A
A (f g) B        -> A f g B
(A f) B          -> A f B
(f g h) A        -> (f A) g h A
A (f g h) B      -> (A f B) g A h B
(f g h i) A      -> f (g A) h i A
..etc
```
Note that this is not an exhaustive list, there may be some edge cases not included.

Partial trains are also present in lead, and are usable by prefixing a train with `:`. For example:
```
(: f g h) A      -> f g h A
A (: f g h) B    -> f g A h B
```
Partial trains, of course, support constants within them too.

## Symbols
|  Symbol  |   Monadic  | Rank |    Dyadic    |  Rank | Notes |
|:--------:|:----------:|:----:|:------------:|:-----:|:-----:|
|  ```+``` |Abs / Cast(`I)| 0  |      Add     |  0 0  |       |
|  ```-``` |\[`B]Neg / Swp case| 0  |Sub      |  0 0  |\`N: negate, \`B: boolean not, \`C: swap cases|
|  ```*``` |Sign / Case |   0  | Mul / Bool AND|  0 0 |Cases: (`1 _1 0 = *'Ab '`)|
|  ```%``` | Reciprocal |   0  |    Div       |  0 0  |       |
|   `\|`   | Boolean Not|   0  |    Residue   |  0 0  |TODO: Update monadic?|
|  ```!``` |  Factorial |   0  |   Bit OR     |  0 0  |Binomial?|
|  ```<``` |    Box     |  99  |   Less Than  |  0 0  |       |
|  ```>``` |   Unbox    |  99  | Greater Than |  0 0  |       |
|  ```^``` |     Exp    |   0  |    Bit AND   |  0 0  |Replace with pow?|
|  ```$``` |    Shape   |  99  |    Reshape   | 1 99  |       |
|  ```[``` |  Identity  |  99  |     Left     | 99 99 |       |
|  ```]``` |  Identity  |  99  |     Right    | 99 99 |       |
|  ```?``` | Dedup Sieve|  99  |     Group    | 99 1  |Keys on right repr. indices of corresponding buckets. Leading axis|
|  ```=``` | Transpose/Keys|2  |   Equality   |  0 0  |Yields list of keys of map|
|  ```~``` |  One-Range |   0  |     Index    | 99 0  |\`C (monad): alphabet upto arg|
|  ```,``` |Ravel/Values|  99  |  Concatenate |  1 1  |Yields values of map|
|  ```;``` |    Mold    |   1  |     Table    | 99 99 |Uses scalar fill char|
|  ```#``` |   Tally    |  99  |    Replicate | 99 1  |Also tallies maps|
|  ```{``` |    Inc     |   0  |      Take    |  1 99 |Uses scalar fill char for out of bounds|
|  ```}``` |    Dec     |   0  |      Drop    |  1 99 |Drops from leading axis descending, negative starts from back|

## Binders
|  Binder |   Usage   | Monadic | Rank | Dyadic | Rank |        Notes       |
|:-------:|:---------:|:-------:|:----:|:------:|:----:|:------------------:|
| ```&``` | ```u&v``` | Compose |  N/A |  Atop  |  N/A | Binds if imm.      |
| ```&:```| ```u&:v```|   Hook  |  N/A |  Hook  |  N/A | Binds as default if imm. |
| ```/``` |  ```u/``` |   Fold  |   1  | N-wise Reduce | 0 99 |              |
| ```\```|  ```u\```|   Scan  |  99  |  Table |99 99 | TODO: Move monad `/` to leading axis?|
| ```@``` | ```u@v``` | Compose |  N/A |  Over  |  N/A | Rank if right arg is immediate |
| ``` ` ```| ```u` ```|   Tie   |  N/A | Commute|  N/A | Flips operands if attached to dyadic operator|
| ```"``` |  ```u"``` |   Each  |  99  |  Each  |99 99 | Each of leading axis |
| ```":```|  ```u":```|  Atoms  |   0  |  Atoms |   0  | Equivalent to `u@0` |

## Library Builtins
TODO. Old(er):

    DS. Matrix multiplication
    mmu:+/&*&:=@1 2@2

    DS. Monadic binary encode ('#:')
    enc:{{(y):(x,`2|y)v 0!`y%2)x}}@1 0&`E.

    DS. Join list of strings on separator
    jn:' '&:{{>(<[,x,])@>/y}}

    DS. Maximum
    max:{{(0>y-x):x)y}}@0

    DS. Minimum
    min:{{(0>y-x):y)x}}@0

    DS. Binary Encode+ (from ayr.js)
    ENC::(0&#,>&[)"&:(-`"&:(max/)#&>")@1#:

## FFI
lead supports a rudimentary FFI interop with C. Link C files with the provided `ffi/liblead.h`. Functions are defined as
```rs
fn(*mut Window, *mut Window) -> *mut Window
```
to be compatible with lead. Currently, one would import, for example:
```c
// liblinkingtest.so
#include "liblead.h"
#include <stdlib.h>

// If monadic b is NULL
Lead_List* do_things(Lead_List* a, Lead_List* b) {
    if (a->c == 0) return a;
    switch (a->i->t) {
    CHAR: 
    BOX: 
        break;
    NUMBER:
        a->i->v.c = abs(a->i->v.n);
        break;
    FLOAT:
        a->i->v.c = abs(a->i->v.f | 0);
        break;
    BOOLEAN:
        a->i->v.c = 0;
        break;
    }
    a->i->t = CHAR;
    return a;
}
```
And then in lead
```    
    []extern'liblinkingtest.so'
    test:[:]{
        do_things: []link'do_things'
    }

    test.do_things 65
A
```
Is an example usage. `[]extern` and `[]link` serve as semi pre-processed internals, allowing them to be passed as values despite yielding functions.

## Quick examples
As I think of them

    -- equiv. to J's dyadic `;`.
    -- First separator between if/else is literal tab, since it is within a group.
    laminate:{{((`X=[?]x):x <x),(`Y=[?]y):y)<y}}

    -- comparison
    enc:{{(y):(x,`2|y)v y%2)x}}@1 0&`()
    enc2:()&{{(*/0=,y):x)(x,`2|y)v y%2}}@=@1

enc is a version of `#:` I implemented in `ayr`. Not only does `enc2` represent `B:` / `ENC` (see above), which is a matrix output of binary representations of
each inputted integer, it is also ~*15x* faster.. thought that was an interesting example to include. Sometimes I forget how useful array languages can be for
certain calculations
