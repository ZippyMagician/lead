#![allow(unused_doc_comments)]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::borrow::Cow;
use std::cell::RefCell;
use std::ffi::{OsStr, OsString};
use std::io::{self, Write};
use std::rc::Rc;

use lead_core::exec::Exec;
use lead_core::lex::Lexer;
use lead_core::parse::Parser;
use lead_core::scope::{Scope, new_scope};
use lead_core::utils::{Arena, error::Status, heap::Heap};

#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Cli,
    File(OsString),
}

#[derive(Debug)]
struct Args {
    pub zero: bool,
    pub debug: bool,
    pub io: i64,

    pub mode: Mode,
    // ...
}

#[cfg(any(unix, target_os = "wasi"))]
fn os_str_to_bytes(s: &OsStr) -> Cow<'_, [u8]> {
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(target_os = "wasi")]
    use std::os::wasi::ffi::OsStrExt;

    Cow::Borrowed(s.as_bytes())
}

#[cfg(not(any(unix, target_os = "wasi")))]
fn os_str_to_bytes(s: &OsStr) -> Cow<'_, [u8]> {
    Cow::Owned(s.to_string_lossy().into_owned().into_bytes())
}

fn help() -> ! {
    println!("{HELP}");
    std::process::exit(1)
}

fn parse_args() -> Args {
    let mut args = Args { zero: false, debug: false, io: 0, mode: Mode::Cli }; // default

    let mut os_args = std::env::args_os().skip(1);
    while let Some(arg) = os_args.next() {
        let bytes = os_str_to_bytes(arg.as_os_str());
        match bytes.as_ref() {
            b"-0" => args.zero = true,
            b"-debug" => args.debug = true,
            b"-io" => {
                let Some(Some(num)) = os_args.next().map(|s| s.to_str()?.parse::<i64>().ok())
                else {
                    help()
                };
                args.io = num;
            }
            _ if bytes[0] != b'-' => args.mode = Mode::File(arg),
            _ => help(),
        }
    }

    if args.debug {
        println!("{args:?}");
    }

    args
}

fn eval(code: &str, args: &Args, exec: &mut Exec, scope: &Scope, arena: &Arena) -> Status {
    let mut lexer = Lexer::new(code);
    let e = lexer.tokenize();
    if args.debug {
        println!("{:?}", lexer.tokens());
    }

    if let Err(e) = e {
        return Err(e);
    }

    let mut parser = Parser::new(arena.clone(), scope.clone(), lexer.tokens());
    let e = parser.parse();
    if args.debug {
        println!("{:?}", &parser.borrow_nodes());
    }

    if let Err(e) = e {
        return Err(e);
    }

    if let Err(e) = exec.exec(parser.nodes()) {
        exec.right = None;
        return Err(e);
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let arena = Rc::new(RefCell::new(Heap::new()));
    let scope = new_scope();
    let mut exec = Exec::new(arena.clone(), scope.clone());

    let args = parse_args();

    // Pass argument values
    let s = scope.borrow();
    if args.zero {
        _ = s.write_internal("RANGEI", false.into());
    }
    if args.io != 0 {
        _ = s.write_internal("IO", args.io.into());
    }
    drop(s);

    if let Mode::File(f) = &args.mode {
        if let Ok(program) = std::fs::read_to_string(f) {
            if let Err(e) = eval(&program, &args, &mut exec, &scope, &arena) {
                println!("{e}");
                std::process::exit(1);
            }

            std::process::exit(0);
        } else {
            println!("File {f:?} not found.");
            help()
        }
    }

    println!(concat!("lead-cli (ayr-deriv) ", env!("CARGO_PKG_VERSION"), ":"));
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut buf = String::with_capacity(100);
    loop {
        print!("    ");
        stdout.flush()?;

        buf.clear();
        if stdin.read_line(&mut buf)? == 0 {
            break;
        }

        let line = buf.trim_end_matches('\r');
        if let Err(e) = eval(&line, &args, &mut exec, &scope, &arena) {
            println!("{e}");
        }
    }
    Ok(())
}

const HELP: &str = concat!(
    "lead-cli (ayr-deriv) ",
    env!("CARGO_PKG_VERSION"),
    r#"
Usage:
    lead <file> [flags] - Runs file
    lead [flags]        - Runs CLI

Flags:
    -debug              - Debug information
    -0                  - Swaps 0 & 1 range symbols"#
);
