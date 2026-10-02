# seqr

Current WIP repository.

### Notes
Based off some of my ideas when rewriting `ayr` in typescript.

Intention is for this to be relatively fast. TODO: Mark notes on more optimized operations here when possible.

Handling ranked operations?
Ideas:
  - Stick with what I did for ayr, generally function that applies to rank
  - Create more specialized functions for some ranks + generic for higher rank operations
  - No generic function, fully specialize?


  - How to handle multiple scalar types within list? Should I support?
  - Should I overload operators more, like in some K impls?
    - Harder to support if lists can hold multiple types..
  - Keep `Scalar` types copyable for speed. Should I overload most arithmetic ops?
  - Dicts?
