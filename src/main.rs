#![feature(try_from)]

pub mod dwarf;

use dwarf::{parse_dwarf_file};

fn main() {
    let dwarf = parse_dwarf_file(String::from("./ref/php.dwz"));
    println!("{:?}", dwarf);
}
