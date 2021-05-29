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
	($fmt:expr) => ({
		fprint!(1, $fmt);
	});
	($fmt:expr, $($args:tt)+) => ({
		fprint!(1, $fmt, $($args)+);
	});
}

#[macro_export]
macro_rules! println {
	() => ({
		print!("\n");
	});
	($fmt:expr) => ({
		print!(concat!($fmt, "\n"));
	});
	($fmt:expr, $($args:tt)+) => ({
		print!(concat!($fmt, "\n"), $($args)+);
	});
}

#[macro_export]
macro_rules! fprint {
	($fd:tt, $fmt:expr) => ({
		if $fd == 2 {
			$crate::print::_print($fd, format_args!(concat!("\x1b[31m", $fmt, "\x1b[0m")));
		} else {
			$crate::print::_print($fd, format_args!($fmt));
		}
	});
	($fd:tt, $fmt:expr, $($args:tt)+) => ({
		if $fd == 2 {
			$crate::print::_print($fd, format_args!(concat!("\x1b[31m", $fmt, "\x1b[0m"), $($args)+));
		} else {
			$crate::print::_print($fd, format_args!($fmt, $($args)+));
		}
	});
}

#[macro_export]
macro_rules! fprintln {
	($fd:tt) => ({
		fprint!($fd, "\n");
	});
	($fd:tt, $fmt:expr) => ({
		fprint!($fd, concat!($fmt, "\n"));
	});
	($fd:tt, $fmt:expr, $($args:tt)+) => ({
		fprint!($fd, concat!($fmt, "\n"), $($args)+);
	});
}
