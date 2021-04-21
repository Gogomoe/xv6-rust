pub mod uart;

pub fn console_put_char(c: u8) {
    uart::uart_put_char_sync(c);
}