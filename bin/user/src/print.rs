use core::convert::Infallible;
use ufmt::uWrite;
pub use ufmt::uwriteln;
use crate::write;

pub struct UserPrinter;

impl UserPrinter {
    fn print(&self, c: u8) {
        unsafe { write(0, &c, 1); }
    }
}

impl uWrite for UserPrinter {
	type Error = Infallible;

    fn write_str(&mut self, s: &str) -> Result<(), Self::Error> {
        for byte in s.bytes() {
            self.print(byte);
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
	($fmt:expr, $($args:tt),*) => ({
		uwriteln!(&mut UserPrinter, $fmt, $($args)*).ok();
	});
}

#[macro_export]
macro_rules! println {
	() => ({
		print!()
	});
	($fmt:expr) => ({
		print!($fmt, )
	});
	($fmt:expr, $($args:tt)+) => ({
		print!($fmt, $($args)+)
	});
}