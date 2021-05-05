#![allow(dead_code)]

use core::ptr;

use spin::Mutex;

use crate::console::console_intr;

const UART0: usize = 0x10000000;

const RHR: usize = 0; /* receive holding register (for input bytes) */
const THR: usize = 0; /* transmit holding register (for output bytes) */
const IER: usize = 1; /* interrupt enable register */
const IER_TX_ENABLE: u8 = 1 << 0;
const IER_RX_ENABLE: u8 = 1 << 1;
const FCR: usize = 2;                 /* FIFO control register */
const FCR_FIFO_ENABLE: u8 = 1 << 0;
const FCR_FIFO_CLEAR: u8 = 3 << 1; /* clear the content of the two FIFOs */
const ISR: usize = 2;                 /* interrupt status register */
const LCR: usize = 3;                 /* line control register */
const LCR_EIGHT_BITS: u8 = 3 << 0;
const LCR_BAUD_LATCH: u8 = 1 << 7; /* special mode to set baud rate */
const LSR: usize = 5;                 /* line status register */
const LSR_RX_READY: u8 = 1 << 0;   /* input is waiting to be read from RHR */
const LSR_TX_IDLE: u8 = 1 << 5;    /* THR can accept another character to send */

static UART_LOCK: Mutex<()> = Mutex::new(());
const UART_TX_BUF_SIZE: usize = 32;
const UART_TX_BUF: [u8; UART_TX_BUF_SIZE] = [0; UART_TX_BUF_SIZE];
static mut UART_TX_W: usize = 0; /* write next to uart_tx_buf[uart_tx_w++] */
static mut UART_TX_R: usize = 0; /* read next from uart_tx_buf[uar_tx_r++] */

macro_rules! read_reg {
    ($reg: expr) => {
        unsafe { ptr::read_volatile((UART0 + $reg) as *const u8) }
    };
}

macro_rules! write_reg {
    ($reg: expr, $value: expr) => {
        unsafe {
            ptr::write_volatile((UART0 + $reg) as *mut u8, $value);
        }
    };
}

pub fn uart_init() {
    // disable interrupts.
    write_reg!(IER, 0x00);

    // special mode to set baud rate.
    write_reg!(LCR, LCR_BAUD_LATCH);

    // LSB for baud rate of 38.4K.
    write_reg!(0, 0x03);

    // MSB for baud rate of 38.4K.
    write_reg!(1, 0x00);

    // leave set-baud mode,
    // and set word length to 8 bits, no parity.
    write_reg!(LCR, LCR_EIGHT_BITS);

    // reset and enable FIFOs.
    write_reg!(FCR, FCR_FIFO_ENABLE | FCR_FIFO_CLEAR);

    // enable transmit and receive interrupts.
    write_reg!(IER, IER_TX_ENABLE | IER_RX_ENABLE);
}

// alternate version of uartputc() that doesn't
// use interrupts, for use by kernel printf() and
// to echo characters. it spins waiting for the uart's
// output register to be empty.
pub fn uart_put_char_sync(c: u8) {
    while (read_reg!(LSR) & LSR_TX_IDLE) == 0 {}
    write_reg!(THR, c);
}

// if the UART is idle, and a character is waiting
// in the transmit buffer, send it.
// caller must hold uart_tx_lock.
// called from both the top- and bottom-half.
pub fn uart_start() {
    loop {
        if unsafe { UART_TX_W == UART_TX_R } {
            // transmit buffer is empty.
            return;
        }

        if (read_reg!(LSR) & LSR_TX_IDLE) == 0 {
            // the UART transmit holding register is full,
            // so we cannot give it another byte.
            // it will interrupt when it's ready for a new byte.
            return;
        }

        let c = unsafe { UART_TX_BUF[UART_TX_R] };

        unsafe {
            UART_TX_R = (UART_TX_R + 1) % UART_TX_BUF_SIZE;
        }

        // maybe uartputc() is waiting for space in the buffer.
        // TODO wakeup(&uart_tx_r);

        write_reg!(THR, c);
    }
}

// read one input character from the UART.
// return -1 if none is waiting.
pub fn uart_get_char() -> Option<u8> {
    return if read_reg!(LSR) & 0x01 != 0 {
        // input data is ready.
        Some(read_reg!(RHR))
    } else {
        None
    };
}

// handle a uart interrupt, raised because input has
// arrived, or the uart is ready for more output, or
// both. called from trap.c.
pub fn uart_intr() {
    loop {
        let c = uart_get_char();
        if c.is_none() { break; }
        console_intr(c.unwrap());
    }

    UART_LOCK.lock();
    uart_start();
}