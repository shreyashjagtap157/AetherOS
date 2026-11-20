#![no_std]
#![no_main]

use core::panic::PanicInfo;

const SBI_EXT_CONSOLE_PUTCHAR: u64 = 0x01;
const SBI_CONSOLE_PUTCHAR: u64 = 0x00;
const SBI_EXT_TIMER: u64 = 0x00;
const SBI_TIMER_SET_TIMER: u64 = 0x00;

// S-mode CSR bits
const SSTATUS_SIE: u64 = 1 << 1;
const SIE_STIE: u64 = 1 << 5;
const SCAUSE_SUPERVISOR_TIMER: u64 = 5;

const CLINT_MTIMECMP0: usize = 0x0200_4000;
const TIMER_INTERVAL: u64 = 10_000_000;

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

fn csrr_stvec() -> u64 {
    let val: u64;
    unsafe {
        core::arch::asm!("csrr {}, stvec", out(reg) val, options(nomem));
    }
    val
}

fn csrr_sstatus() -> u64 {
    let val: u64;
    unsafe {
        core::arch::asm!("csrr {}, sstatus", out(reg) val, options(nomem));
    }
    val
}

fn csrr_sie() -> u64 {
    let val: u64;
    unsafe {
        core::arch::asm!("csrr {}, sie", out(reg) val, options(nomem));
    }
    val
}

fn csrw_stvec(val: u64) {
    unsafe {
        core::arch::asm!("csrw stvec, {}", in(reg) val, options(nomem));
    }
}

fn csrs_sie(bits: u64) {
    unsafe {
        core::arch::asm!("csrs sie, {}", in(reg) bits, options(nomem));
    }
}

fn csrs_sstatus(bits: u64) {
    unsafe {
        core::arch::asm!("csrs sstatus, {}", in(reg) bits, options(nomem));
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

fn clint_write_mtimecmp0(val: u64) {
    unsafe {
        (CLINT_MTIMECMP0 as *mut u64).write_volatile(val);
    }
}

unsafe extern "C" {
    fn trap_entry();
}

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

#[unsafe(no_mangle)]
pub extern "C" fn trap_handler(mcause: u64, sepc: u64, stval: u64) -> u64 {
    if mcause == SCAUSE_SUPERVISOR_TIMER {
        unsafe {
            TICKS = TICKS.wrapping_add(1);
        }
        sbi_console_write(b"tick ");
        // Print tick count in hex
        let ticks = unsafe { TICKS };
        sbi_console_write_u64_hex(ticks);
        sbi_console_write(b"\n");

        // Re-arm timer
        let now = unsafe { (CLINT_MTIMECMP0 as *const u64).read_volatile() };
        unsafe {
            (CLINT_MTIMECMP0 as *mut u64).write_volatile(now.wrapping_add(TIMER_INTERVAL));
        }
    }
    sepc
}

#[unsafe(no_mangle)]
pub extern "C" fn start() -> ! {
    sbi_console_write(b"start\n");

    // Read current CSR values (diagnostic)
    let _stvec = csrr_stvec();
    let _sstatus = csrr_sstatus();
    let _sie = csrr_sie();

    // Install trap vector
    csrw_stvec(trap_entry as *const () as usize as u64);

    // Enable S-mode timer interrupt
    csrs_sie(SIE_STIE);
    csrs_sstatus(SSTATUS_SIE);

    // Set timer via SBI
    sbi_timer_set(10_000_000);

    sbi_console_write(b"hello from kernel\n");
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}