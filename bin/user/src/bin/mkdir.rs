#![no_std]
#![no_main]

use user::*;

#[no_mangle]
pub fn main(_args: Vec<&str>) {
    if _args.is_empty() {
        println!("Usage: mkdir files...");
        exit(1);
    } else {
        for i in 0.._args.len() {
            if mkdir(_args[i]) < 0 {
                eprintln!("mkdir: {} failed to create", _args[i]);
                break;
            }
        }
    }
}
