use core::fmt;
use spin::Mutex;
use crate::console;
use core::fmt::Write;

struct ConsolePrinter {}

impl ConsolePrinter {
    fn print(&self, c: u8) {
        console::console_put_char(c);
    }
}

impl fmt::Write for ConsolePrinter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.print(byte);
        }
        Ok(())
    }
}

static PRINTER: Mutex<ConsolePrinter> = Mutex::new(ConsolePrinter {});

pub fn _print(args: fmt::Arguments<'_>) {
    use core::fmt::Write;
    PRINTER.lock().write_fmt(args).expect("_print: error");
}

#[macro_export]
macro_rules! print {
	($($args:tt)+) => ({
		$crate::print::_print(format_args!($($args)+));
	});
}

#[macro_export]
macro_rules! println {
	() => ({
		print!("\n")
	});
	($fmt:expr) => ({
		print!(concat!($fmt, "\n"))
	});
	($fmt:expr, $($args:tt)+) => ({
		print!(concat!($fmt, "\n"), $($args)+)
	});
}