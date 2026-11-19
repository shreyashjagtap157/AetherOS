//! Minimal RISC-V64 bare-metal boot checkpoint for the AetherOS reference slice.
//!
//! This crate is deliberately tiny. It establishes the Phase B cross-toolchain
//! baseline by compiling to an ELF payload linked at `0x80200000` (the OpenSBI
//! load address used by QEMU `virt`). It is NOT evidence that AetherOS boots,
//! NOT a qualification artifact, and NOT hardware support.
//!
//! Output goes through the OpenSBI `console_putchar` ecall; the payload then
//! enters an idle loop. Boot correctness can ONLY be demonstrated by an actual
//! emulator run (`qemu-system-riscv64 -machine virt -bios default -kernel ...`)
//! recorded by the qualifying pipeline. This source compiles; that is all it
//! proves on its own.

#![no_std]
#![no_main]

core::arch::global_asm!(
    ".section .text._start",
    ".globl _start",
    "_start:",
    "la sp, _stack_top",
    "call rust_main",
    "unimp",
);

/// OpenSBI extension for writing one byte to the console.
const SBI_EXT_CONSOLE_PUTCHAR: u64 = 0x01;

/// `SBI_CONSOLE_PUTCHAR::SBI_CONSOLE_PUTCHAR` sub-extension.
const SBI_CONSOLE_PUTCHAR: u64 = 0x00;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

/// Writes one byte to the SBI console.
fn sbi_console_putchar(byte: u8) {
    // SAFETY: SBI ecall is the documented OpenSBI console ABI; the extension
    // ids are constants and registers are used per the SBI calling convention.
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

/// Writes a byte string to the SBI console.
fn sbi_console_write(bytes: &[u8]) {
    for &byte in bytes {
        sbi_console_putchar(byte);
    }
}

/// Payload entry point, reached from `_start`.
#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    sbi_console_write(b"AetherOS RV64 boot checkpoint\n");
    loop {}
}
