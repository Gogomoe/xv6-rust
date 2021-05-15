#![allow(dead_code)]

// Format of an ELF executable file
pub const ELF_MAGIC: u32 = 0x464C457F;  // "\x7FELF" in little endian

// File header
#[repr(C)]
pub struct ElfHeader {
    // must equal ELF_MAGIC
    pub magic: u32,
    pub elf: [u8; 12],
    pub types: u16,
    pub machine: u16,
    pub version: u32,
    pub entry: u64,
    pub phoff: u64,
    pub shoff: u64,
    pub flags: u32,
    pub ehsize: u16,
    pub phentsize: u16,
    pub phnum: u16,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
}

impl ElfHeader {
    pub const fn new() -> ElfHeader {
        ElfHeader {
            magic: 0,
            elf: [0; 12],
            types: 0,
            machine: 0,
            version: 0,
            entry: 0,
            phoff: 0,
            shoff: 0,
            flags: 0,
            ehsize: 0,
            phentsize: 0,
            phnum: 0,
            shentsize: 0,
            shnum: 0,
            shstrndx: 0,
        }
    }
}

// Program section header
#[repr(C)]
pub struct ProgramHeader {
    pub types: u32,
    pub flags: u32,
    pub off: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}

impl ProgramHeader {
    pub const fn new() -> ProgramHeader {
        ProgramHeader {
            types: 0,
            flags: 0,
            off: 0,
            vaddr: 0,
            paddr: 0,
            filesz: 0,
            memsz: 0,
            align: 0,
        }
    }
}

// Values for Proghdr type
pub const ELF_PROG_LOAD: u32 = 1;

// Flag bits for Proghdr flags
pub const ELF_PROG_FLAG_EXEC: usize = 1;
pub const ELF_PROG_FLAG_WRITE: usize = 2;
pub const ELF_PROG_FLAG_READ: usize = 4;
