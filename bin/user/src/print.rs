use core::convert::Infallible;
use ufmt::uWrite;
use crate::write;

pub struct UserPrinter;

impl UserPrinter {
    fn print(&self, fd: usize, c: u8) {
        write(fd, &c, 1);
    }
}

impl uWrite for UserPrinter {
	type Error = Infallible;

    fn write_str(&mut self, s: &str) -> Result<(), Self::Error> {
        for byte in s.bytes() {
            self.print(1, byte);
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
	($fmt:expr, $($args:tt),*) => ({
		use ufmt::uwriteln;
		use print::UserPrinter;
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