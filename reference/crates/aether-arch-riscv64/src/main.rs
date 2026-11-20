#![no_std]
#![no_main]

use core::panic::PanicInfo;

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

fn sbi_console_write_u64_hex(val: u64) {
    let mut buf = [0u8; 18];
    buf[0] = b'0';
    buf[1] = b'x';
    let mut v = val;
    for i in (2..18).rev() {
        buf[i] = match v % 16 {
            0..=9 => b'0' + (v % 16) as u8,
            10..=15 => b'a' + (v % 16 - 10) as u8,
            _ => b'?',
        };
        v /= 16;
    }
    for &b in &buf {
        sbi_console_putchar(b);
    }
}

fn csrr_mtvec() -> u64 {
    let val: u64;
    unsafe {
        core::arch::asm!("csrr {}, mtvec", out(reg) val, options(nomem));
    }
    val
}

#[unsafe(no_mangle)]
pub extern "C" fn start() -> ! {
    sbi_console_write(b"start\n");
    sbi_console_write(b"mtvec: ");
    let v = csrr_mtvec();
    sbi_console_write_u64_hex(v);
    sbi_console_write(b"\n");
    sbi_console_write(b"hello from kernel\n");
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}