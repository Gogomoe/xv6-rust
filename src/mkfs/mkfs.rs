use std::env;
use std::process::exit;

use xv6_rust::file_system;
use xv6_rust::file_system::BLOCK_SIZE;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: mkfs fs.img files..");
        exit(1);
    }

    assert!(BLOCK_SIZE == 0);
}