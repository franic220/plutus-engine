use std::process;
use crate::reader::run;

mod mapper;
mod test_helpers;
mod reader;

fn main() {
    if let Err(err) = run() {
        eprintln!("Error executing run! {}", err);
        process::exit(1);
    }
}
