use crate::syscall::write;
use core::fmt;

struct UserPrinter {
    pub fd: usize,
}

impl fmt::Write for UserPrinter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            write(self.fd, &byte, 1);
        }
        Ok(())
    }
}

static mut PRINTER: UserPrinter = UserPrinter { fd: 1 };

pub fn _print(fd: usize, args: fmt::Arguments<'_>) {
    use core::fmt::Write;
	unsafe {
		PRINTER.fd = fd;
		PRINTER.write_fmt(args).expect("_print: error");
	}
}

#[macro_export]
macro_rules! print {
	($($args:tt)+) => ({
		$crate::print::_print(1, format_args!($($args)+));
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

#[macro_export]
macro_rules! fprint {
	($fd:tt, $($args:tt)+) => ({
		$crate::print::_print($fd, format_args!($($args)+));
	});
}

#[macro_export]
macro_rules! fprintln {
	($fd:tt) => ({
		fprint!($fd, "\n")
	});
	($fd:tt, $fmt:expr) => ({
		fprint!($fd, concat!($fmt, "\n"))
	});
	($fd:tt, $fmt:expr, $($args:tt)+) => ({
		fprint!($fd, concat!($fmt, "\n"), $($args)+)
	});
}
