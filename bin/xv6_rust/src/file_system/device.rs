use param_lib::MAX_DEV_NUMBER;

pub struct Device {
    pub read: Option<fn(usize, usize) -> usize>,
    pub write: Option<fn(usize, usize) -> usize>,
}

impl Device {
    pub const fn new() -> Device {
        Device {
            read: None,
            write: None,
        }
    }
}

pub static mut DEVICES: [Device; MAX_DEV_NUMBER] = array![_ => Device::new(); MAX_DEV_NUMBER];

pub const CONSOLE: usize = 1;