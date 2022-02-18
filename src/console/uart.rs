// Intel 8250 serial port (UART)
// https://wiki.osdev.org/UART

use core::fmt;
use console::{BACKSCHAR, BACKSPACE, console_interrupt};
use lapic::microdelay;
use x86::io::{inb, outb};

const COM1: u16 = 0x3f8;

static mut UART_PRESENT: bool = false;

pub struct UartWriter {
}

impl UartWriter {

    pub fn new() -> Self {
        uart_init();
        UartWriter {}
    }

    pub fn write_char(&self, ch: i32) {
        if ch == BACKSPACE {
            self.write_byte(BACKSCHAR);
            self.write_byte(b' ');
            self.write_byte(BACKSCHAR);
        } else {
            self.write_byte(ch as u8);
        }
    }

    fn write_byte(&self, byte: u8) {
        match byte {
            0x20..=0x7e | b'\n' => uart_put_char(byte as i32),
            _ => uart_put_char(0x3f as i32)
        }
    }

    pub fn write_string(&self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write_char(byte as i32),
                _ => self.write_byte(0xfe)
            }
        }
    }

    pub fn clear_screen(&self) {
        uart_put_char(27);
        uart_put_char('[' as i32);
        uart_put_char('2' as i32);
        uart_put_char('J' as i32);
        uart_put_char(27);
        uart_put_char('[' as i32);
        uart_put_char('H' as i32);
    }

}

impl fmt::Write for UartWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

fn uart_init() {
    unsafe {
        // Turn off the FIFO
        outb(COM1 + 2, 0);

        // 9600 baud, 8 data bits, 1 stop bit, parity off.
        outb(COM1 + 3, 0x80);    // Unlock divisor
        outb(COM1 + 0, 12);
        outb(COM1 + 1, 0);
        outb(COM1 + 3, 0x03);    // Lock divisor, 8 data bits.
        outb(COM1 + 4, 0);
        outb(COM1 + 1, 0x01);    // Enable receive interrupts.

        // If status is 0xFF, no serial port.
        if inb(COM1 + 5) == 0xFF {
            return;
        }

        UART_PRESENT = true;

        // Acknowledge pre-existing interrupt conditions;
        // enable interrupts.
        inb(COM1 + 2);
        inb(COM1 + 0);
        // TODO: Reimplement
        // ioapicenable(IRQ_COM1, 0);

        // Announce that we're here.
        for ch in b"xv6...\n" {
            uart_put_char(*ch as i32);
        }
    }
}

pub fn uart_put_char(c: i32) {
    unsafe {
        if !UART_PRESENT {
            return;
        }

        for _i in 0..128 {
            if (inb(COM1+5) & 0x20) != 0 {
                break;
            }

            microdelay(10);
        }

        outb(COM1+0, c as u8);
    }
}

pub fn uart_get_char() -> i32 {
    unsafe {
        if !UART_PRESENT {
            return -1;
        }
        if !inb(COM1 + 5) & 0x01 != 0 {
            return -1;
        }

        inb(COM1 + 0) as i32
    }
}

pub fn uart_interrupt() {
    console_interrupt(uart_get_char);
}