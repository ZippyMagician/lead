# lead
Redesign + rewrite of `ayr`, a toy array language intended for code golfing. The rewrite takes a more general purpose approach, although most of the symbols will be lifted from the previous version.

## Building
Clone the repository to a local location. Run
```sh
cargo install --path lead-cli
```
Use
```
cargo install --path lead-cli --features ffi
```
to compile with the supporting FFI linking internals `[]extern` and `[]link`.

For building the webserver, ensure you have `wasm-bindgen` installed and accessible on the path. Use
```
RUSTFLAGS='--cfg=getrandom_backend="wasm_js"' cargo build --package lead-wasm --target=wasm32-unknown-unknown --release
wasm-bindgen --out-dir ./docs/pkg ./target/wasm32-unknown-unknown/release/lead_wasm.wasm --target web
```
to build the files into the `./docs` directory
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
