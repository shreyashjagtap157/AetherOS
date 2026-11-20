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

fn write_mtvec(val: u64) {
    unsafe {
        core::arch::asm!("csrw mtvec, {}", in(reg) val, options(nomem));
    }
}

// trap_entry as a Rust function (keeps it alive for linker)
#[unsafe(no_mangle)]
pub extern "C" fn trap_entry() {
    // No-op: just return
}

#[unsafe(no_mangle)]
pub extern "C" fn start() -> ! {
    write_mtvec(trap_entry as *const () as usize as u64);
    sbi_console_write(b"hello from kernel\n");
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}