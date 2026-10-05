#![allow(unused_doc_comments)]

use std::cell::RefCell;
use std::rc::Rc;

use seqr::builtins::symbols::Symbol;
use seqr::lex;
use seqr::list::List;
use seqr::parse;
use seqr::scalar::Scalar;
use seqr::scope::new_scope;

use typed_arena::Arena;

fn main() -> Result<(), seqr::utils::error::Error> {
    /// --- LEXING --- \\\
    let mut lexer = lex::Lexer::new("1 2 3 +* x 4 5 6\n'hello world'4");
    lexer.tokenize()?;
    let tokens = lexer.tokens();

    println!("-TOKENS-\n{:?}", tokens);

    /// --- LIST --- \\\
    let l = List::from_vec(vec![
        Scalar::Number(4),
        Scalar::Number(-4),
        Scalar::Number(2),
        Scalar::Float(1.3),
    ])
    .unwrap();
    let r = List::from_vec(vec![Scalar::Boolean(false), Scalar::Boolean(true)]).unwrap();

    println!("{l:?}");
    println!("{:?}", l.clone().with_rank(&[3, 3]));

    /// --- SCOPE --- \\\
    let scope = new_scope();
    println!("{}", scope.borrow().has("literal"));

    /// --- SYMBOLS --- \\\
    if let (lex::TokenTag::Symbol, sym) = tokens[3] {
        let plus_symbol = Symbol::get_symbol(sym).unwrap().getfn();
        println!("\n-M-\n{:?}", plus_symbol.monadic_call(l.clone()));
        println!("\n-D-\n{:?}", plus_symbol.dyadic_call(l, r));

        /// --- SPEED --- \\\
        let mut build = Vec::with_capacity(10_000_000);
        for i in 0..10_000_000 {
            build.push(Scalar::Number(i as i32));
        }
        let list = List::from_vec(build).unwrap();
        let one = List::scalar(Scalar::Number(1));

        let t1 = std::time::Instant::now();
        let result = plus_symbol.dyadic_call(list, one);
        let t2 = std::time::Instant::now();
        println!("{}: {:?}", result.is_ok(), t2 - t1);
    }

    /// --- PARSING --- \\\
    println!("\n-P-");
    let mut lexer = lex::Lexer::new("   1 _2 3 'hello' 4.7 'list' 'of' 'strings' x y BOXC. ");
    lexer.tokenize()?;

    let arena = Rc::new(RefCell::new(Arena::<List>::new()));
    let mut parser = parse::Parser::new(arena, scope.clone(), lexer.tokens());
    parser.parse()?;

    println!("{:?}", parser.nodes());

    Ok(())
}
