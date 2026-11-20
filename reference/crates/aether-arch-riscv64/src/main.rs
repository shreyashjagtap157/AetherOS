#![no_std]
#![no_main]

use core::panic::PanicInfo;

// SSTATUS bits
const SSTATUS_SIE: u64 = 1 << 1;

// SIE bits
const SIE_STIE: u64 = 1 << 5;

// Supervisor interrupt causes
const SCAUSE_SUPERVISOR_TIMER: u64 = 5;

const CLINT_BASE: usize = 0x0200_0000;
const CLINT_MTIME: usize = CLINT_BASE + 0xBFF8;
const CLINT_MTIMECMP0: usize = CLINT_BASE + 0x4000;
const TIMER_INTERVAL: u64 = 10_000_000;

const SBI_EXT_CONSOLE_PUTCHAR: u64 = 0x01;
const SBI_CONSOLE_PUTCHAR: u64 = 0x00;

static mut TICKS: u64 = 0;

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

core::arch::global_asm!(
    ".section .text",
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
    "  csrr a0, scause",
    "  csrr a1, sepc",
    "  csrr a2, stval",
    "",
    "  call trap_handler",
    "",
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
    "  sret",
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

fn write_stvec(val: u64) {
    unsafe {
        core::arch::asm!("csrw stvec, {}", in(reg) val, options(nomem));
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

fn clint_read_mtime() -> u64 {
    unsafe { (CLINT_MTIME as *const u64).read_volatile() }
}

fn clint_write_mtimecmp0(val: u64) {
    unsafe {
        (CLINT_MTIMECMP0 as *mut u64).write_volatile(val);
    }
}

unsafe extern "C" {
    fn trap_entry();
}

#[unsafe(no_mangle)]
pub extern "C" fn trap_handler(mcause: u64, mepc: u64, _mtval: u64) -> u64 {
    if mcause == SCAUSE_SUPERVISOR_TIMER {
        unsafe {
            TICKS = TICKS.wrapping_add(1);
        }
        sbi_console_write(b"tick\n");

        let now = clint_read_mtime();
        clint_write_mtimecmp0(now.wrapping_add(TIMER_INTERVAL));
    }
    mepc
}

#[unsafe(no_mangle)]
pub extern "C" fn start() -> ! {
    write_stvec(trap_entry as *const () as usize as u64);

    set_sie(SIE_STIE);
    set_sstatus(SSTATUS_SIE);

    let now = clint_read_mtime();
    clint_write_mtimecmp0(now.wrapping_add(TIMER_INTERVAL));

    sbi_console_write(b"hello from kernel\n");
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}