use std::cell::RefCell;
use std::rc::Rc;

use typed_arena::Arena;
use wasm_bindgen::prelude::*;
use web_sys::*;

use lead_core::*;

thread_local! {
    static SCOPE: RefCell<scope::ScopeInternal> = scope::new_scope().take().into();
    static ARENA: RefCell<Arena<types::Type>> = RefCell::new(Arena::new());
}

#[wasm_bindgen]
pub fn exec_lead(input: &str) -> String {
    let scope = SCOPE.take();
    let arena = ARENA.take();
    let s = Rc::new(RefCell::new(scope));
    let a = Rc::new(RefCell::new(arena));

    let mut lexer = lex::Lexer::new(input);
    if let Err(e) = lexer.tokenize() {
        let scope = s.take();
        let arena = a.take();
        SCOPE.set(scope);
        ARENA.set(arena);
        return e.to_string();
    }
    let mut parser = parse::Parser::new(a.clone(), s.clone(), lexer.tokens());
    if let Err(e) = parser.parse() {
        drop(parser);
        let scope = s.take();
        let arena = a.take();
        SCOPE.set(scope);
        ARENA.set(arena);
        return e.to_string();
    }
    let mut exec = exec::Exec::new(a.clone(), s.clone());
    exec.silent = true;

    let res = exec.exec_wasm(parser.nodes()).map_or_else(|e| e.to_string(), |v| v);

    drop(exec);
    let scope = s.take();
    let arena = a.take();
    SCOPE.set(scope);
    ARENA.set(arena);
    res
}

#[wasm_bindgen]
extern "C" {}
