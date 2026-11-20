#![no_std]
#![no_main]

use core::panic::PanicInfo;

// Use S-mode CSRs (works whether kernel is in M or S mode)
const SSTATUS_SIE: u64 = 1 << 1;
const SIE_STIE: u64 = 1 << 5;

const SBI_EXT_CONSOLE_PUTCHAR: u64 = 0x01;
const SBI_CONSOLE_PUTCHAR: u64 = 0x00;

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

fn set_sie(bits: u64) {
    unsafe {
        core::arch::asm!("csrs sie, {}", in(reg) bits, options(nomem));
    }
}

fn set_sstatus(bits: u64) {
    unsafe {
        core::arch::asm!("csrs sstatus, {}", in(reg) bits, options(nomem));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn start() -> ! {
    set_sie(SIE_STIE);
    set_sstatus(SSTATUS_SIE);
    sbi_console_write(b"hello from kernel\n");
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}