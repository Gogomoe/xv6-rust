use core::fmt;
use crate::syscall::write;

struct UserPrinter {}

impl UserPrinter {
    fn print(&self, c: u8) {
        write(0, &c, 1);
    }
}

impl fmt::Write for UserPrinter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.print(byte);
        }
        Ok(())
    }
}

static mut PRINTER: UserPrinter = UserPrinter {};

pub fn _print(args: fmt::Arguments<'_>) {
    use core::fmt::Write;
    unsafe { &mut PRINTER }.write_fmt(args).expect("_print: error");
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