use core::ptr;

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

macro_rules! ReadReg {
    ($reg: expr) => {
        unsafe { ptr::read_volatile((UART0 + $reg) as *const u8) }
    };
}

macro_rules! WriteReg {
    ($reg: expr, $value: expr) => {
        unsafe {
            ptr::write_volatile((UART0 + $reg) as *mut u8, $value);
        }
    };
}

pub fn uart_init() {
    // disable interrupts.
    WriteReg!(IER, 0x00);

    // special mode to set baud rate.
    WriteReg!(LCR, LCR_BAUD_LATCH);

    // LSB for baud rate of 38.4K.
    WriteReg!(0, 0x03);

    // MSB for baud rate of 38.4K.
    WriteReg!(1, 0x00);

    // leave set-baud mode,
    // and set word length to 8 bits, no parity.
    WriteReg!(LCR, LCR_EIGHT_BITS);

    // reset and enable FIFOs.
    WriteReg!(FCR, FCR_FIFO_ENABLE | FCR_FIFO_CLEAR);

    // enable transmit and receive interrupts.
    WriteReg!(IER, IER_TX_ENABLE | IER_RX_ENABLE);
}

pub fn uart_put_char_sync(c: u8) {
    while (ReadReg!(LSR) & LSR_TX_IDLE) == 0 {}
    WriteReg!(THR, c);
}