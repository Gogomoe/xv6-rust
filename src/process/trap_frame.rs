#[repr(C)]
pub struct TrapFrame {
    /*   0 */ kernel_satp: u64,
    /* kernel page table */
    /*   8 */ kernel_sp: u64,
    /* top of process's kernel stack */
    /*  16 */ kernel_trap: u64,
    /* usertrap() */
    /*  24 */ epc: u64,
    /* saved user program counter */
    /*  32 */ kernel_hartid: u64,
    /* saved kernel tp */
    /*  40 */ ra: u64,
    /*  48 */ sp: u64,
    /*  56 */ gp: u64,
    /*  64 */ tp: u64,
    /*  72 */ t0: u64,
    /*  80 */ t1: u64,
    /*  88 */ t2: u64,
    /*  96 */ s0: u64,
    /* 104 */ s1: u64,
    /* 112 */ a0: u64,
    /* 120 */ a1: u64,
    /* 128 */ a2: u64,
    /* 136 */ a3: u64,
    /* 144 */ a4: u64,
    /* 152 */ a5: u64,
    /* 160 */ a6: u64,
    /* 168 */ a7: u64,
    /* 176 */ s2: u64,
    /* 184 */ s3: u64,
    /* 192 */ s4: u64,
    /* 200 */ s5: u64,
    /* 208 */ s6: u64,
    /* 216 */ s7: u64,
    /* 224 */ s8: u64,
    /* 232 */ s9: u64,
    /* 240 */ s10: u64,
    /* 248 */ s11: u64,
    /* 256 */ t3: u64,
    /* 264 */ t4: u64,
    /* 272 */ t5: u64,
    /* 280 */ t6: u64,
}
