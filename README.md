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

  - Some symbol builtins will have more optimized rank support
	  - General rank functions will be applied otherwise
	  - Create some sort of distinction via flags? `@` will note and wrap when necessary

  - How to handle multiple scalar types within list? Should I support?
  - Should I overload operators more, like in some K impls?
    - Harder to support if lists can hold multiple types..
  - Keep `Scalar` types copyable for speed. Should I overload most arithmetic ops?
  - Dicts?

### Syntax (tentative)
```
[~] 2 * ~e9    -- Don't print
mean: +/%#     -- Define literal
mean ~25       -- Mean 1..=25
'hello world'  -- String
map: [:]{ a: 'first'; b: 'second' }
[]map.a        -- 'first'
[]map.('b')    -- 'second'
I.             -- STDIN
```

### TODO
- [x] Non-instant groups (mainly trains)
- [x] Operators
- [x] Proper `@` rank support
- [x] Recursive blocks via `v`
- [ ] Port symbols + operators
- [ ] Maps
- [ ] Accessors
- [ ] Redesign/remove Internals
- [ ] Tables?
