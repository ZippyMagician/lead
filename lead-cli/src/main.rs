#![allow(unused_doc_comments)]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

use lead_core::exec::Exec;
use lead_core::lex::Lexer;
use lead_core::parse::Parser;
use lead_core::scope::new_scope;

use typed_arena::Arena;

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut buf = String::with_capacity(100);

    let arena = Rc::new(RefCell::new(Arena::new()));
    let scope = new_scope();

    let mut exec = Exec::new(arena.clone(), scope.clone());

    println!("lead (ayr-deriv) v0.1.0:");
    loop {
        print!("    ");
        stdout.flush()?;

        buf.clear();
        if stdin.read_line(&mut buf)? == 0 {
            break;
        }

        let line = buf.trim_end_matches('\r');

        const DEBUG: bool = false;
        let mut lexer = Lexer::new(&line);
        if let Err(e) = lexer.tokenize() {
            println!("{e}");
            continue;
        } else if DEBUG {
            println!("{:?}", lexer.tokens());
        }

        let mut parser = Parser::new(arena.clone(), scope.clone(), lexer.tokens());
        if let Err(e) = parser.parse() {
            println!("{e}");
            continue;
        } else if DEBUG {
            println!("{:?}", &parser.borrow_nodes());
        }

        if let Err(e) = exec.exec(parser.nodes()) {
            println!("{e}");
            continue;
        }
    }
    Ok(())
}
