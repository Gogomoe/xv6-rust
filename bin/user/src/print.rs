#[macro_export]
macro_rules! print {
	($fd:tt, $fmt:tt, $($args:tt)*) => ({
		use core::convert::Infallible;
		use ufmt::{uWrite, uwriteln};
		use crate::write;

		struct UserPrinter;
		impl uWrite for UserPrinter {
			type Error = Infallible;

		    fn write_str(&mut self, s: &str) -> Result<(), Self::Error> {
				write($fd, s.as_ptr(), s.len());
		        Ok(())
		    }
		}

		uwriteln!(&mut UserPrinter, $fmt, $($args)*).ok();
	});
}

#[macro_export]
macro_rules! println {
	() => ({
		print!(1, "")
	});
	($fmt:tt) => ({
		print!(1, $fmt, )
	});
	($fmt:tt, $($args:tt)+) => ({
		print!(1, $fmt, $($args)+)
	});
}

#[macro_export]
macro_rules! fprintln {
	($fd:tt) => ({
		print!($fd, "")
	});
	($fd:tt, $fmt:tt) => ({
		print!($fd, $fmt, )
	});
	($fd:tt, $fmt:tt, $($args:tt)+) => ({
		print!($fd, $fmt, $($args)+)
	});
}
