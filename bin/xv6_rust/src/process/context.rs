#[repr(C)]
pub struct Context {
    pub ra: u64,
    pub sp: u64,

    // callee-saved
    pub s0: u64,
    pub s1: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64,
    pub s10: u64,
    pub s11: u64,
}

impl Context {
    pub const fn new() -> Context {
        Self {
            ra: 0,
            sp: 0,
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
        }
    }

    pub fn clear(&mut self) {
        self.ra = 0;
        self.sp = 0;
        self.s0 = 0;
        self.s1 = 0;
        self.s2 = 0;
        self.s3 = 0;
        self.s4 = 0;
        self.s5 = 0;
        self.s6 = 0;
        self.s7 = 0;
        self.s8 = 0;
        self.s9 = 0;
        self.s10 = 0;
        self.s11 = 0;
    }
}
