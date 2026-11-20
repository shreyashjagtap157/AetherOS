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

// Stack for task 1 (4 KiB, grows down).
static mut TASK1_STACK: [u64; 128] = [0; 128];

// ---------------------------------------------------------------------------
// Minimum executor: task context switching
// ---------------------------------------------------------------------------

/// Saved general-purpose register state for one task.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct TaskContext {
    /// Program counter (mepc CSR value).
    pc: u64,
    ra: u64,
    gp: u64,
    tp: u64,
    t0: u64,
    t1: u64,
    t2: u64,
    s0: u64,
    s1: u64,
    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
    s2: u64,
    s3: u64,
    s4: u64,
    s5: u64,
    s6: u64,
    s7: u64,
    s8: u64,
    s9: u64,
    s10: u64,
    s11: u64,
    t3: u64,
    t4: u64,
    t5: u64,
    t6: u64,
    /// Stack pointer (restored after mret).
    sp: u64,
}

/// Number of concurrently scheduled tasks.
const MAX_TASKS: usize = 2;

/// Per-task saved register state.
static mut TASKS: [TaskContext; MAX_TASKS] = [TaskContext {
    pc: 0,
    ra: 0,
    gp: 0,
    tp: 0,
    t0: 0,
    t1: 0,
    t2: 0,
    s0: 0,
    s1: 0,
    a0: 0,
    a1: 0,
    a2: 0,
    a3: 0,
    a4: 0,
    a5: 0,
    a6: 0,
    a7: 0,
    s2: 0,
    s3: 0,
    s4: 0,
    s5: 0,
    s6: 0,
    s7: 0,
    s8: 0,
    s9: 0,
    s10: 0,
    s11: 0,
    t3: 0,
    t4: 0,
    t5: 0,
    t6: 0,
    sp: 0,
}; MAX_TASKS];

/// Index of the currently running task.
static mut CURRENT_TASK: usize = 0;

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

fn read_mepc() -> u64 {
    let val: u64;
    unsafe {
        core::arch::asm!("csrr {}, mepc", out(reg) val, options(nomem));
    }
    val
}

fn write_mepc(val: u64) {
    unsafe {
        core::arch::asm!("csrw mepc, {}", in(reg) val, options(nomem));
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

/// Task 1 entry point: prints a marker and loops (waits for next timer tick).
#[unsafe(no_mangle)]
pub extern "C" fn task1_entry() -> ! {
    sbi_console_write(b"task1 running\n");
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
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

/// Save current task context from the trap frame, switch to the next task,
/// and prepare the stack so trap_entry's register restore loads the next
/// task's registers. Called from trap_handler on every timer tick.
///
/// Stack frame layout on entry (sp points to frame start):
///   offset 0: ra, 8: gp, 16: tp, ..., 232: t6  (256 bytes total)
fn save_and_switch() {
        unsafe {
    // Current stack pointer (points to saved trap frame).
    let sp: u64;
    core::arch::asm!("mv {}, sp", out(reg) sp, options(nomem));

    let current = CURRENT_TASK;
    let task = &mut TASKS[current];

    // Save program counter from mepc CSR.
    task.pc = read_mepc();

    // Save all 31 GPRs from the trap frame (sp + offset, 8 bytes each).
    let frame = sp as *mut u64;
    task.ra = *frame.add(0);
    task.gp = *frame.add(1);
    task.tp = *frame.add(2);
    task.t0 = *frame.add(3);
    task.t1 = *frame.add(4);
    task.t2 = *frame.add(5);
    task.s0 = *frame.add(6);
    task.s1 = *frame.add(7);
    task.a0 = *frame.add(8);
    task.a1 = *frame.add(9);
    task.a2 = *frame.add(10);
    task.a3 = *frame.add(11);
    task.a4 = *frame.add(12);
    task.a5 = *frame.add(13);
    task.a6 = *frame.add(14);
    task.a7 = *frame.add(15);
    task.s2 = *frame.add(16);
    task.s3 = *frame.add(17);
    task.s4 = *frame.add(18);
    task.s5 = *frame.add(19);
    task.s6 = *frame.add(20);
    task.s7 = *frame.add(21);
    task.s8 = *frame.add(22);
    task.s9 = *frame.add(23);
    task.s10 = *frame.add(24);
    task.s11 = *frame.add(25);
    task.t3 = *frame.add(26);
    task.t4 = *frame.add(27);
    task.t5 = *frame.add(28);
    task.t6 = *frame.add(29);

    // Original sp (before trap_entry's `addi sp, sp, -256`).
    task.sp = sp.wrapping_add(256);

    // Switch to next task (round-robin).
    let next = (current + 1) % MAX_TASKS;
    CURRENT_TASK = next;

    let next_task = &TASKS[next];

    // Set mepc so mret returns to the next task's pc.
    write_mepc(next_task.pc);

    // Where trap_entry will restore from: it does ld reg, offset(sp) then
    // addi sp, sp, 256. We want final sp = next_task.sp, so set sp now to
    // next_task.sp - 256 and write next task's regs at that frame.
    let next_sp = next_task.sp.wrapping_sub(256);
    let next_frame = next_sp as *mut u64;
    *next_frame.add(0) = next_task.ra;
    *next_frame.add(1) = next_task.gp;
    *next_frame.add(2) = next_task.tp;
    *next_frame.add(3) = next_task.t0;
    *next_frame.add(4) = next_task.t1;
    *next_frame.add(5) = next_task.t2;
    *next_frame.add(6) = next_task.s0;
    *next_frame.add(7) = next_task.s1;
    *next_frame.add(8) = next_task.a0;
    *next_frame.add(9) = next_task.a1;
    *next_frame.add(10) = next_task.a2;
    *next_frame.add(11) = next_task.a3;
    *next_frame.add(12) = next_task.a4;
    *next_frame.add(13) = next_task.a5;
    *next_frame.add(14) = next_task.a6;
    *next_frame.add(15) = next_task.a7;
    *next_frame.add(16) = next_task.s2;
    *next_frame.add(17) = next_task.s3;
    *next_frame.add(18) = next_task.s4;
    *next_frame.add(19) = next_task.s5;
    *next_frame.add(20) = next_task.s6;
    *next_frame.add(21) = next_task.s7;
    *next_frame.add(22) = next_task.s8;
    *next_frame.add(23) = next_task.s9;
    *next_frame.add(24) = next_task.s10;
    *next_frame.add(25) = next_task.s11;
    *next_frame.add(26) = next_task.t3;
    *next_frame.add(27) = next_task.t4;
    *next_frame.add(28) = next_task.t5;
    *next_frame.add(29) = next_task.t6;

    // Set sp so trap_entry's loads read next task's regs.
    core::arch::asm!("mv sp, {}", in(reg) next_sp, options(nomem));
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
pub extern "C" fn trap_handler(_mcause: u64, mepc: u64, _mtval: u64) -> u64 {
    let mcause = _mcause;

    if mcause == MCAUSE_MACHINE_TIMER {
        unsafe {
            TICKS = TICKS.wrapping_add(1);
        }
        sbi_console_write(b"tick ");
        sbi_console_write_u64(unsafe { TICKS });

        // Context switch on timer tick: save current task, restore next.
        save_and_switch();

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

    // Initialise task 1 with a valid entry point and stack.
    unsafe {
        TASKS[1].pc = task1_entry as *const () as usize as u64;
        TASKS[1].sp = (&raw const TASK1_STACK as *const [u64; 128] as usize + 1024) as u64;
    }
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
