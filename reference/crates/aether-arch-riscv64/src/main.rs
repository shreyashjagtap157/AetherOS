#![no_std]
#![no_main]

use core::panic::PanicInfo;

const SBI_EXT_CONSOLE_PUTCHAR: u64 = 0x01;
const SBI_CONSOLE_PUTCHAR: u64 = 0x00;
const SBI_EXT_TIMER: u64 = 0x00;
const SBI_TIMER_SET_TIMER: u64 = 0x00;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

core::arch::global_asm!(
    ".section .text._start",
    ".globl _start",
    "_start:",
    "  la sp, _stack_top",
    "  call start",
    "  unimp",
);

fn sbi_console_putchar(byte: u8) {
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a0") u64::from(byte),
            in("a6") SBI_CONSOLE_PUTCHAR,
            in("a7") SBI_EXT_CONSOLE_PUTCHAR,
            options(nomem, nostack),
        );
    }
}

fn sbi_console_write(bytes: &[u8]) {
    for &byte in bytes {
        sbi_console_putchar(byte);
    }
}

fn sbi_timer_set(value: u64) {
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a0") SBI_TIMER_SET_TIMER,
            in("a1") value,
            in("a7") SBI_EXT_TIMER,
            options(nomem, nostack),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn start() -> ! {
    sbi_console_write(b"start\n");
    sbi_console_write(b"hello from kernel\n");
    // Set timer to fire after 1 second (10000000 time units at 10MHz)
    sbi_timer_set(10_000_000);
    sbi_console_write(b"timer set\n");
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}