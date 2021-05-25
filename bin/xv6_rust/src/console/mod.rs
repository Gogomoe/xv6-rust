use crate::console::uart::{uart_init, uart_put_char, uart_put_char_sync};
use crate::file_system::device::DEVICES;
use crate::memory::{either_copy_in, either_copy_out};
use crate::process::{CPU_MANAGER, PROCESS_MANAGER};
use crate::spin_lock::SpinLock;
use crate::file_control_lib::CONSOLE_ID;

pub mod uart;

pub fn console_put_char(c: u8) {
    uart_put_char_sync(c);
}

pub fn console_put_backspace() {
    uart_put_char_sync(0x08);
    uart_put_char_sync(b' ');
    uart_put_char_sync(0x08);
}

const INPUT_BUFFER: usize = 128;

struct Console {
    buffer: [u8; INPUT_BUFFER],
    read: usize,
    write: usize,
    edit: usize,
}

static CONSOLE: SpinLock<Console> = SpinLock::new(Console {
    buffer: [0; INPUT_BUFFER],
    read: 0,
    write: 0,
    edit: 0,
}, "console");

const CTRL_P: u8 = b'P' - b'@';
const CTRL_U: u8 = b'U' - b'@';
const CTRL_H: u8 = b'H' - b'@';
const CTRL_D: u8 = b'D' - b'@';

//
// user write()s to the console go here.
//
pub fn console_write(user_src: bool, src: usize, n: usize) -> usize {
    for i in 0..n {
        let char: u8 = 0;
        if !either_copy_in(user_src, &char as *const u8 as usize, src + i, 1) {
            return i;
        }
        uart_put_char(char);
    }
    return n;
}

//
// user read()s from the console go here.
// copy (up to) a whole input line to dst.
// user_dist indicates whether dst is a user
// or kernel address.
pub fn console_read(user_dst: bool, mut dst: usize, mut n: usize) -> usize {
    let target = n;

    let mut guard = CONSOLE.lock();
    let console = unsafe { (&mut *guard as *mut Console).as_mut() }.unwrap();
    while n > 0 {
        // wait until interrupt handler has put some
        // input into cons.buffer.
        while console.read == console.write {
            if CPU_MANAGER.my_proc().info().killed {
                drop(guard);
                return usize::max_value();
            }
            CPU_MANAGER.my_cpu().sleep(&console.read as *const _ as usize, guard);
            guard = CONSOLE.lock();
        }

        let c = console.buffer[console.read % INPUT_BUFFER];
        console.read += 1;

        if c == CTRL_D {
            // end-of-file
            if n < target {
                // Save ^D for next time, to make sure
                // caller gets a 0-byte result.
                console.read -= 1;
            }
            break;
        }

        // copy the input byte to the user-space buffer.
        let c_buf = c;
        if !either_copy_out(user_dst, dst, &c_buf as *const u8 as usize, 1) {
            break;
        }

        dst += 1;
        n -= 1;

        if c == b'\n' {
            // a whole line has arrived, return to
            // the user-level read().
            break;
        }
    }

    drop(guard);

    return target - n;
}

// the console input interrupt handler.
// uartintr() calls this for input character.
// do erase/kill processing, append to cons.buf,
// wake up consoleread() if a whole line has arrived.
pub fn console_intr(char: u8) {
    let mut lock = CONSOLE.lock();
    let console = &mut *lock;

    match char {
        CTRL_P => {
            PROCESS_MANAGER.print_processes();
        }
        CTRL_U => {
            while console.edit != console.write
                && console.buffer[(console.edit - 1) % INPUT_BUFFER] != b'\n' {
                console.edit -= 1;
                console_put_backspace();
            }
        }
        CTRL_H | b'\x7f' => { // Backspace
            if console.edit != console.write {
                console.edit -= 1;
                console_put_backspace();
            }
        }
        _ => {
            if char != 0 && console.edit - console.read < INPUT_BUFFER {
                let char = if char == b'\r' { b'\n' } else { char };
                console_put_char(char);
                console.buffer[console.edit % INPUT_BUFFER] = char;
                console.edit += 1;

                if char == b'\n' || char == CTRL_D || console.edit == console.read + INPUT_BUFFER {
                    console.write = console.edit;
                    PROCESS_MANAGER.wake_up(&console.read as *const _ as usize)
                }
            }
        }
    }
}

pub fn console_init() {
    uart_init();

    // connect read and write system calls
    // to consoleread and consolewrite.

    unsafe {
        DEVICES[CONSOLE_ID].read = Some(console_read);
        DEVICES[CONSOLE_ID].write = Some(console_write);
    }
}