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
use lead_core::function::Function;
use lead_core::list::List;
use lead_core::scalar::Scalar;

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
        Function::new(
            Rc::new(|a, mut b, o| {
                let mut iterations = 1;
                if b.is_some() {
                    if !a.is_empty() {
                        iterations = a.to_list()?.list()[0].to_num()? as usize;
                    }
                } else {
                    b = Some(a);
                }
                let b = b.take().unwrap();

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
            }),
            99,
            [99, 99],
            arena.clone(),
            scope.clone(),
            None,
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

        #[cfg(feature = "ffi")]
        if line.starts_with("extern") {
            let name = line.split(' ').nth(1).unwrap_or("");
            let path = absolute(Path::new(name)).unwrap();
            if let Err(e) =
                lead_core::ffi::internal::load_library(&path)
            {
                println!("{e}");
            }
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

/*
fn main() -> Result<(), lead_core::utils::error::Error> {
    /// --- LEXING --- \\\
    let mut lexer = lex::Lexer::new("1 2 3 +* x 4 5 6\n'hello world'4");
    lexer.tokenize()?;
    let tokens = lexer.tokens();

    //println!("-TOKENS-\n{:?}", tokens);

    /// --- LIST --- \\\
    let l = List::from_vec(vec![
        Scalar::Number(4),
        Scalar::Number(-4),
        Scalar::Number(2),
        Scalar::Float(1.3),
    ])
    .unwrap();
    let r = List::from_vec(vec![Scalar::Boolean(false), Scalar::Boolean(true)]).unwrap();

    println!("{l}");
    println!("{}", l.clone().with_rank(&[3, 3])?);

    let string_mat = List::string("helloworld");
    let string_mat = string_mat.with_rank(&[5, 2])?;

    /// --- SCOPE --- \\\
    let scope = new_scope();
    println!("{}", scope.borrow().has("literal"));

    /// --- SYMBOLS --- \\\
    if let (lex::TokenTag::Symbol, sym) = tokens[3] {
        let plus_symbol = Symbol::get_symbol(sym).unwrap().getfn(scope.clone());
        println!("\n-M-\n{}", plus_symbol.monadic_call(l.clone())?);
        println!("\n-D-\n{}", plus_symbol.dyadic_call(l, r)?);

        /// --- SPEED --- \\\
        let mut build = Vec::with_capacity(10_000_000);
        for i in 0..10_000_000 {
            build.push(Scalar::Number(i));
        }
        let list = List::from_vec(build).unwrap();
        let one = List::scalar(Scalar::Number(1));

        let t1 = std::time::Instant::now();
        let result = plus_symbol.dyadic_call(list, one);
        let t2 = std::time::Instant::now();
        println!("{}: {:?}", result.is_ok(), t2 - t1);
    }

    /// --- PARSING --- \\\
    const P: &str = "f: [{] 1 2 3 + y * 2\n f (54)"; //"x: 4 5 6 + ~3\n2 * x";
    // println!("\n-P-\n\"\"\" {P} \"\"\"");
    let mut lexer = lex::Lexer::new(P);
    lexer.tokenize()?;

    let arena = Rc::new(RefCell::new(Arena::<List>::new()));
    let mut parser = parse::Parser::new(arena.clone(), scope.clone(), lexer.tokens());
    parser.parse()?;

    // println!("{:?}", parser.nodes());
    let mut evaluator = exec::Exec::new(arena.clone(), scope);
    let t1 = std::time::Instant::now();
    evaluator.exec(parser.nodes())?;
    let t2 = std::time::Instant::now();
    println!("{:?}", t2 - t1);

    /// --- PRINTING --- \\\
    let boxed = List::scalar(Scalar::boxed(arena.clone(), string_mat)?);
    println!("{}", boxed.with_rank(&[2, 2])?);

    Ok(())
}*/

fn speedtest(arena: Rc<RefCell<Arena<lead_core::scope::Type>>>, scope: lead_core::scope::Scope) {
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
