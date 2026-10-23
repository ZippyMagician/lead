# lead
Redesign + rewrite of `ayr`, a toy array language intended for code golfing. The rewrite takes a more general purpose approach, although most of the symbols will be lifted from the previous version.

## Building
Clone the repository to a local location. Run
```sh
cargo vendor
cargo install --path lead-cli
```
Use
```
cargo install --path lead-cli --features ffi
```
to compile with the supporting FFI linking internals `[]extern` and `[]link`.

Alternatively, you can comment out the vendor specifications in `.cargo/config.toml` and simply use the second command.

### Syntax (tentative)
```
[~] 2 * ~e9    -- Don't print
mean: +/%#     -- Define literal
mean ~25       -- Mean 1..=25
'hello world'  -- String
map: [:]{ a: 'first'; b: 'second' }
map.a        -- 'first'
map.('b')    -- 'second'
[]I          -- STDIN?
```

### TODO
- [x] Non-instant groups (mainly trains)
- [x] Operators
- [x] Proper `@` rank support
- [x] Recursive blocks via `v`
- [ ] Port symbols + operators
- [x] Support maps in some functions?
- [x] Maps
- [ ] Support variadic accessors?
- [x] Accessors
- [ ] Tables?
