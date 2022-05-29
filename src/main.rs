#[warn(
    missing_debug_implementations,
    missing_copy_implementations,
    unused_import_braces,
    unused_lifetimes
)]
use crate::program::Program;

mod lexer;
mod program;

fn main() {
    let program = Program::from_file("examples/bad_strs.fl");
    for token in program.tokens() {
        match token {
            Ok(t) => println!("{:?}", t),
            Err(e) => eprintln!("{}", e),
        }
    }
}
