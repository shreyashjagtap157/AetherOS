//! Minimal RISC-V64 kernel for AetherOS Phase B.
//!
//! Extends the boot checkpoint with trap handling, machine timer
//! interrupts, and a tick-driven execution loop. This is the foundation
//! for the executor, Sv39 address spaces, and IPC path.
//!
//! Build:   cargo build --target riscv64gc-unknown-none-elf
//! Run:     qemu-system-riscv64 -machine virt -bios default \
//!            -kernel target/riscv64gc-unknown-none-elf/debug/aether-arch-riscv64 -nographic
//!
//! Timer interrupts are delivered by the QEMU virt CLINT (memory-mapped at
//! 0x02000000). Each tick prints a counter via the OpenSBI console ecall.

#![no_std]
#![no_main]

use core::panic::PanicInfo;

// MSTATUS bits
const MSTATUS_MIE: u64 = 1 << 3;

// MIE bits
const MIE_MTIE: u64 = 1 << 7;

// Machine interrupt causes
const MCAUSE_MACHINE_TIMER: u64 = 7;

// CLINT (QEMU virt machine, memory-mapped)
const CLINT_BASE: usize = 0x0200_0000;
const CLINT_MTIME: usize = CLINT_BASE + 0xBFF8;   // 64-bit read-only
const CLINT_MTIMECMP0: usize = CLINT_BASE + 0x4000; // 64-bit read-write, hart 0

// Timer interval in mtime ticks.
const TIMER_INTERVAL: u64 = 10_000_000;

// SBI
const SBI_EXT_CONSOLE_PUTCHAR: u64 = 0x01;
const SBI_CONSOLE_PUTCHAR: u64 = 0x00;

// Global tick counter, incremented by the timer interrupt handler.
static mut TICKS: u64 = 0;

// ---------------------------------------------------------------------------
// Trap entry (assembly): saves all registers, calls Rust handler,
// restores registers, mret.
// ---------------------------------------------------------------------------

core::arch::global_asm!(
    ".section .text.trap",
    ".globl trap_entry",
    "trap_entry:",
    "  addi sp, sp, -256",
    "  sd ra, 0(sp)",
    "  sd gp, 8(sp)",
    "  sd tp, 16(sp)",
    "  sd t0, 24(sp)",
    "  sd t1, 32(sp)",
    "  sd t2, 40(sp)",
    "  sd s0, 48(sp)",
    "  sd s1, 56(sp)",
    "  sd a0, 64(sp)",
    "  sd a1, 72(sp)",
    "  sd a2, 80(sp)",
    "  sd a3, 88(sp)",
    "  sd a4, 96(sp)",
    "  sd a5, 104(sp)",
    "  sd a6, 112(sp)",
    "  sd a7, 120(sp)",
    "  sd s2, 128(sp)",
    "  sd s3, 136(sp)",
    "  sd s4, 144(sp)",
    "  sd s5, 152(sp)",
    "  sd s6, 160(sp)",
    "  sd s7, 168(sp)",
    "  sd s8, 176(sp)",
    "  sd s9, 184(sp)",
    "  sd s10, 192(sp)",
    "  sd s11, 200(sp)",
    "  sd t3, 208(sp)",
    "  sd t4, 216(sp)",
    "  sd t5, 224(sp)",
    "  sd t6, 232(sp)",
    "",
    "  # Pass mcause, mepc, mtval in a0, a1, a2",
    "  csrr a0, mcause",
    "  csrr a1, mepc",
    "  csrr a2, mtval",
    "",
    "  call trap_handler",
    "",
    "  # Restore all registers",
    "  ld t6, 232(sp)",
    "  ld t5, 224(sp)",
    "  ld t4, 216(sp)",
    "  ld t3, 208(sp)",
    "  ld s11, 200(sp)",
    "  ld s10, 192(sp)",
    "  ld s9, 184(sp)",
    "  ld s8, 176(sp)",
    "  ld s7, 168(sp)",
    "  ld s6, 160(sp)",
    "  ld s5, 152(sp)",
    "  ld s4, 144(sp)",
    "  ld s3, 136(sp)",
    "  ld s2, 128(sp)",
    "  ld a7, 120(sp)",
    "  ld a6, 112(sp)",
    "  ld a5, 104(sp)",
    "  ld a4, 96(sp)",
    "  ld a3, 88(sp)",
    "  ld a2, 80(sp)",
    "  ld a1, 72(sp)",
    "  ld a0, 64(sp)",
    "  ld s1, 56(sp)",
    "  ld s0, 48(sp)",
    "  ld t2, 40(sp)",
    "  ld t1, 32(sp)",
    "  ld t0, 24(sp)",
    "  ld tp, 16(sp)",
    "  ld gp, 8(sp)",
    "  ld ra, 0(sp)",
    "  addi sp, sp, 256",
    "",
    "  mret",
);

// ---------------------------------------------------------------------------
// _start: set sp, call rust_main (same as boot checkpoint)
// ---------------------------------------------------------------------------

core::arch::global_asm!(
    ".section .text._start",
    ".globl _start",
    "_start:",
    "  la sp, _stack_top",
    "  call start",
    "  unimp",
);

// ---------------------------------------------------------------------------
// CSR helpers (CSR address is an immediate operand, not a register)
// ---------------------------------------------------------------------------

fn write_mtvec(val: u64) {
    unsafe {
        core::arch::asm!("csrw mtvec, {}", in(reg) val, options(nomem));
    }
}

fn set_mie(bits: u64) {
    unsafe {
        core::arch::asm!("csrs mie, {}", in(reg) bits, options(nomem));
    }
}

fn set_mstatus(bits: u64) {
    unsafe {
        core::arch::asm!("csrs mstatus, {}", in(reg) bits, options(nomem));
    }
}

// ---------------------------------------------------------------------------
// SBI console
// ---------------------------------------------------------------------------

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

fn sbi_console_write_u64(value: u64) {
    let mut buf = [0u8; 20];
    let mut n = 0;
    let mut v = value;
    if v == 0 {
        buf[0] = b'0';
        n = 1;
    } else {
        while v > 0 {
            buf[n] = b'0' + (v % 10) as u8;
            v /= 10;
            n += 1;
        }
    }
    for i in (0..n).rev() {
        sbi_console_putchar(buf[i]);
    }
    sbi_console_putchar(b'\n');
}

// ---------------------------------------------------------------------------
// CLINT timer
// ---------------------------------------------------------------------------

fn clint_read_mtime() -> u64 {
    unsafe { (CLINT_MTIME as *const u64).read_volatile() }
}

fn clint_write_mtimecmp0(val: u64) {
    unsafe {
        (CLINT_MTIMECMP0 as *mut u64).write_volatile(val);
    }
}

// ---------------------------------------------------------------------------
// Trap handler (Rust)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn trap_entry();
}

/// Handles a machine trap. Called from the assembly trap_entry with
/// mcause in a0, mepc in a1, mtval in a2. Returns the next PC (mepc
/// is returned unchanged so mret re-executes the interrupted
/// instruction).
#[unsafe(no_mangle)]
pub extern "C" fn trap_handler(_mcause: u64, _mepc: u64, _mtval: u64) -> u64 {
    let mcause = _mcause;
    let mepc = _mepc;

    if mcause == MCAUSE_MACHINE_TIMER {
        unsafe {
            TICKS = TICKS.wrapping_add(1);
        }
        sbi_console_write(b"tick ");
        sbi_console_write_u64(unsafe { TICKS });

        // Arm the next timer interrupt.
        let now = clint_read_mtime();
        clint_write_mtimecmp0(now.wrapping_add(TIMER_INTERVAL));
    }

    mepc
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

/// Kernel initialisation: install trap vector, enable timer interrupts,
// arm the first timer, then hand off to rust_main.
#[unsafe(no_mangle)]
pub extern "C" fn start() -> ! {
    // Install trap vector.
    write_mtvec(trap_entry as *const () as usize as u64);

    // Enable machine timer interrupt.
    set_mie(MIE_MTIE);
    set_mstatus(MSTATUS_MIE);

    // Arm first timer interrupt.
    let now = clint_read_mtime();
    clint_write_mtimecmp0(now.wrapping_add(TIMER_INTERVAL));

    rust_main()
}

/// Kernel main: wait for interrupts; the timer handler fires on each tick.
#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    sbi_console_write(b"AetherOS RV64 kernel: trap + timer ready\n");
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}
