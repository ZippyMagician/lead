#![allow(unused_doc_comments)]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::cell::RefCell;
use std::io::{self, Write};
#[cfg(feature = "ffi")]
use std::path::{Path, absolute};
use std::rc::Rc;
use std::time::Instant;

use lead_core::exec::Exec;
use lead_core::lex::Lexer;
use lead_core::parse::Parser;
use lead_core::scope::new_scope;

use lead_core::builtins::symbols::Symbol;
use lead_core::types::list::List;
use lead_core::types::scalar::Scalar;

use typed_arena::Arena;

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut buf = String::with_capacity(100);

    let arena = Rc::new(RefCell::new(Arena::new()));
    let scope = new_scope();

    let s = scope.clone();
    s.borrow_mut().set_function(
        "time",
        lead_core::function!(|a, b, o| {
                let iterations = a
                    .as_list()?
                    .list()
                    .get(0)
                    .map(|n| n.to_num())
                    .unwrap_or(Ok(1))? as usize;

                let b = b.unwrap_or(a);

                let arena = Rc::new(RefCell::new(Arena::new()));
                let line = format!("{b}");
                let mut times = Vec::with_capacity(iterations);
                let mut exec = Exec::new(arena.clone(), o.s.clone());
                exec.silent = true;
                let mut l = Lexer::new(&line);
                l.tokenize()?;

                for _ in 0..iterations {
                    let t0 = Instant::now();
                    let mut p = Parser::new(arena.clone(), o.s.clone(), l.tokens());
                    p.parse()?;
                    exec.exec(p.nodes())?;
                    let t1 = Instant::now();
                    times.push(t1 - t0);
                }

                drop(arena);
                let mut acc = times[0];
                for i in 1..iterations {
                    acc += times[i];
                }
                acc /= iterations as u32;
                Ok(List::string(&format!("{acc:?}")).into())
            }; arena.clone(), scope.clone()
        ),
    );

    // TODO: Move to internals once those are implemented
    #[cfg(feature = "ffi")]
    s.borrow_mut().set_function(
        "extern",
        lead_core::function!(|a, b, _| {
                if b.is_some() {
                    return lead_core::err!(Domain);
                }
                let name = format!("{a}");
                let path = absolute(Path::new(&name)).unwrap();
                lead_core::ffi::internal::load_library(&path)?;
                Ok(a)
            }; arena.clone(), scope.clone()
        ),
    );
    #[cfg(feature = "ffi")]
    s.borrow_mut().set_function(
        "link",
        lead_core::function!(|a, b, o| {
                if b.is_some() {
                    return lead_core::err!(Domain);
                }
                let name = format!("{a}");
                let sym = lead_core::ffi::internal::link_symbol(&name)?;
                let s = o.s.clone();
                let ar = o.a.clone();
                s.borrow_mut().set_function(
                    &name,
                    lead_core::function!(move |a, b, _| {
                        lead_core::ffi::internal::call_symbol(
                            sym.clone(), &ar, a.to_list()?,
                            b.map(|n| n.to_list()).transpose()?,
                        ).map(lead_core::types::Value::from)
                    }; o.a.clone(), o.s.clone()
                ));
                Ok(List::scalar(Scalar::Boolean(true)).into())
            }; arena.clone(), scope.clone()
        ),
    );

    let mut exec = Exec::new(arena.clone(), scope.clone());

    println!("lead_core (ayr-deriv) v0.0.1:");
    loop {
        print!("    ");
        stdout.flush()?;

        buf.clear();
        if stdin.read_line(&mut buf)? == 0 {
            break;
        }

        let line = buf.trim_end_matches('\r');
        if line == "test speed\n" {
            speedtest(arena.clone(), scope.clone());
            continue;
        }

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

fn speedtest(arena: Rc<RefCell<Arena<lead_core::types::Type>>>, scope: lead_core::scope::Scope) {
    // --- SPEED TEST --- \\
    let mut build = Vec::with_capacity(10_000_000);
    for i in 0..10_000_000 {
        build.push(Scalar::Number(i));
    }
    let list = List::from_vec(build).unwrap();
    let one = List::scalar(Scalar::Number(1));
    let plus_symbol = Symbol::get_symbol("+").unwrap().getfn(arena, scope);

    let t1 = Instant::now();
    let result = plus_symbol.dyadic_call(list.into(), one.into());
    let t2 = Instant::now();
    println!("{}: {:?}", result.is_ok(), t2 - t1);
    // --- END --- \\\
}
