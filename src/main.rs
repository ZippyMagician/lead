use std::rc::Rc;

use seqr::function::Function;
use seqr::lex;
use seqr::list::List;
use seqr::scalar::Scalar;
use seqr::scope::new_scope;

fn main() {
    let mut toks = lex::Lexer::new("1 2 3 +* x 4 5 6\n'hello world'4");
    toks.tokenize();

    println!("{:?}", toks.tokens());

    let l = List::from_vec(vec![
        Scalar::Number(4),
        Scalar::Number(4),
        Scalar::Number(2),
        Scalar::Float(1.3),
    ]);

    println!("{l:?}");
    println!("{:?}", l.with_rank(&[3, 3]));

    let scope = new_scope();
    println!("{}", scope.borrow().has("literal"));

    let _left = Function::new(Rc::new(|a, _| a), Rc::new(|a, _, _| a), 99, [99, 99]);
}
