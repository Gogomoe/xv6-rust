use spin::Mutex;

use crate::process::PROCESS_MANAGER;

pub mod uart;

pub fn console_put_char(c: u8) {
    uart::uart_put_char_sync(c);
}

pub fn console_put_backspace() {
    uart::uart_put_char_sync(0x08);
    uart::uart_put_char_sync(b' ');
    uart::uart_put_char_sync(0x08);
}

const INPUT_BUFFER: usize = 128;

struct Console {
    buffer: [u8; INPUT_BUFFER],
    read: usize,
    write: usize,
    edit: usize,
}

static CONSOLE: Mutex<Console> = Mutex::new(Console {
    buffer: [0; INPUT_BUFFER],
    read: 0,
    write: 0,
    edit: 0,
});

// the console input interrupt handler.
// uartintr() calls this for input character.
// do erase/kill processing, append to cons.buf,
// wake up consoleread() if a whole line has arrived.
pub fn console_intr(char: u8) {
    let mut lock = CONSOLE.lock();
    let console = &mut *lock;

    const CTRL_P: u8 = b'P' - b'@';
    const CTRL_U: u8 = b'U' - b'@';
    const CTRL_H: u8 = b'H' - b'@';
    const CTRL_D: u8 = b'D' - b'@';

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
                console.buffer[console.edit & INPUT_BUFFER] = char;
                console.edit += 1;

                if char == b'\n' || char == CTRL_D || console.edit == console.read + INPUT_BUFFER {
                    console.write = console.edit;
                    PROCESS_MANAGER.wakeup(&console.read as *const _ as usize)
                }
            }
        }
    }
}