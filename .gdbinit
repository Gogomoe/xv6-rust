set confirm off
set architecture riscv:rv64
target remote 127.0.0.1:26000
symbol-file target/riscv64gc-unknown-none-elf/debug/xv6_rust
set disassemble-next-line auto
