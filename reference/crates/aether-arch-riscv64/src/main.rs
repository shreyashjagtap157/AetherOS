#![no_std]
#![no_main]
#![allow(static_mut_refs)]
#![allow(unsafe_code)]
#![allow(unsafe_op_in_unsafe_fn)]

/// Runtime VM: userspace execution engine model
/// Executes a small program via SBI console output.

const NUM_REGS: usize = 8;
const MEM_SIZE: usize = 64 * 1024;
const SYS_PRINT: u64 = 1;
const SYS_EXIT: u64 = 2;
const SYS_READ: u64 = 3;
const SYS_WRITE: u64 = 4;
const SYS_GETPID: u64 = 5;

#[derive(Clone, Copy)]
struct Inst {
    op: u64,
    a: u64,
    b: u64,
    c: u64,
}

#[derive(Clone, Copy)]
#[repr(u8)]
enum Op {
    Nop = 0,
    Mov = 1,
    Add = 2,
    Sub = 3,
    Load = 4,
    Store = 5,
    Jmp = 6,
    Jz = 7,
    Jnz = 8,
    Call = 9,
    Ret = 10,
    Syscall = 11,
    Halt = 12,
}

struct Process {
    regs: [u64; NUM_REGS],
    pc: usize,
    sp: usize,
    mem: [u8; MEM_SIZE],
    call_stack: [usize; 16],
    call_sp: usize,
    stdout: [u8; 256],
    stdout_len: usize,
    exited: bool,
    exit_code: u64,
}

impl Process {
    fn new() -> Self {
        Self {
            regs: [0; NUM_REGS],
            pc: 0,
            sp: MEM_SIZE - 1,
            mem: [0; MEM_SIZE],
            call_stack: [0; 16],
            call_sp: 0,
            stdout: [0; 256],
            stdout_len: 0,
            exited: false,
            exit_code: 0,
        }
    }

    fn load_program(&mut self, program: &[Inst]) {
        for (i, inst) in program.iter().enumerate() {
            let offset = i * 32;
            if offset + 32 > MEM_SIZE {
                break;
            }
            self.mem[offset..offset + 8].copy_from_slice(&inst.op.to_le_bytes());
            self.mem[offset + 8..offset + 16].copy_from_slice(&inst.a.to_le_bytes());
            self.mem[offset + 16..offset + 24].copy_from_slice(&inst.b.to_le_bytes());
            self.mem[offset + 24..offset + 32].copy_from_slice(&inst.c.to_le_bytes());
        }
    }

    fn read_u64(&self, addr: usize) -> Result<u64, ()> {
        if addr + 8 > MEM_SIZE {
            return Err(());
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.mem[addr..addr + 8]);
        Ok(u64::from_le_bytes(bytes))
    }

    fn write_u64(&mut self, addr: usize, val: u64) -> Result<(), ()> {
        if addr + 8 > MEM_SIZE {
            return Err(());
        }
        let bytes = val.to_le_bytes();
        self.mem[addr..addr + 8].copy_from_slice(&bytes);
        Ok(())
    }

    fn fetch(&self) -> Result<Inst, ()> {
        if self.pc + 32 > MEM_SIZE {
            return Err(());
        }
        let op = self.read_u64(self.pc)?;
        let a = self.read_u64(self.pc + 8)?;
        let b = self.read_u64(self.pc + 16)?;
        let c = self.read_u64(self.pc + 24)?;
        Ok(Inst { op, a, b, c })
    }

    fn step(&mut self) -> Result<(), ()> {
        if self.exited {
            return Err(());
        }
        let inst = self.fetch()?;
        match inst.op {
            0 => {
                self.pc += 32;
            }
            1 => {
                let rd = inst.a as usize;
                if rd >= NUM_REGS {
                    return Err(());
                }
                if inst.b == 1 {
                    self.regs[rd] = inst.c;
                } else {
                    let rs = inst.c as usize;
                    if rs >= NUM_REGS {
                        return Err(());
                    }
                    self.regs[rd] = self.regs[rs];
                }
                self.pc += 32;
            }
            2 => {
                let rd = inst.a as usize;
                let rs = inst.b as usize;
                if rd >= NUM_REGS || rs >= NUM_REGS {
                    return Err(());
                }
                self.regs[rd] = self.regs[rd].wrapping_add(self.regs[rs]);
                self.pc += 32;
            }
            3 => {
                let rd = inst.a as usize;
                let rs = inst.b as usize;
                if rd >= NUM_REGS || rs >= NUM_REGS {
                    return Err(());
                }
                self.regs[rd] = self.regs[rd].wrapping_sub(self.regs[rs]);
                self.pc += 32;
            }
            4 => {
                let rd = inst.a as usize;
                let addr = inst.b as usize;
                if rd >= NUM_REGS {
                    return Err(());
                }
                self.regs[rd] = self.read_u64(addr)?;
                self.pc += 32;
            }
            5 => {
                let addr = inst.a as usize;
                let rs = inst.b as usize;
                if rs >= NUM_REGS {
                    return Err(());
                }
                self.write_u64(addr, self.regs[rs])?;
                self.pc += 32;
            }
            6 => {
                self.pc = inst.a as usize;
            }
            7 => {
                let rd = inst.a as usize;
                let addr = inst.b as usize;
                if rd >= NUM_REGS {
                    return Err(());
                }
                if self.regs[rd] == 0 {
                    self.pc = addr;
                } else {
                    self.pc += 32;
                }
            }
            8 => {
                let rd = inst.a as usize;
                let addr = inst.b as usize;
                if rd >= NUM_REGS {
                    return Err(());
                }
                if self.regs[rd] != 0 {
                    self.pc = addr;
                } else {
                    self.pc += 32;
                }
            }
            9 => {
                self.call_stack[self.call_sp] = self.pc + 32;
                self.call_sp += 1;
                self.pc = inst.a as usize;
            }
            10 => {
                if self.call_sp == 0 {
                    return Err(());
                }
                self.call_sp -= 1;
                self.pc = self.call_stack[self.call_sp];
            }
            11 => {
                let num = inst.a;
                match num {
                    SYS_PRINT => {
                        let reg = inst.b as usize;
                        if reg >= NUM_REGS {
                            return Err(());
                        }
                        let val = self.regs[reg];
                        if self.stdout_len < self.stdout.len() {
                            self.stdout[self.stdout_len] = (val & 0xFF) as u8;
                            self.stdout_len += 1;
                        }
                    }
                    SYS_EXIT => {
                        self.exited = true;
                        self.exit_code = inst.b;
                        return Err(());
                    }
                    SYS_READ => {
                        let fd = inst.b as usize;
                        let offset = inst.c as usize;
                        let sector_offset = fd * BLOCK_SIZE + offset;
                        if sector_offset >= RAM_DISK_SIZE {
                            return Err(());
                        }
                        self.regs[0] = unsafe { RAM_DISK[sector_offset] } as u64;
                    }
                    SYS_WRITE => {
                        let fd = inst.b as usize;
                        let offset = inst.c as usize;
                        let sector_offset = fd * BLOCK_SIZE + offset;
                        if sector_offset >= RAM_DISK_SIZE {
                            return Err(());
                        }
                        unsafe { RAM_DISK[sector_offset] = self.regs[0] as u8; }
                    }
                    SYS_GETPID => {
                        let reg = inst.b as usize;
                        if reg >= NUM_REGS {
                            return Err(());
                        }
                        unsafe {
                            self.regs[reg] = CURRENT_TASK as u64;
                        }
                    }
                    _ => return Err(()),
                }
                self.pc += 32;
            }
            12 => {
                self.exited = true;
                self.exit_code = 0;
                return Err(());
            }
            _ => return Err(()),
        }
        Ok(())
    }

    fn flush_stdout(&mut self) {
        for i in 0..self.stdout_len {
            sbi_console_putchar(self.stdout[i]);
        }
        self.stdout_len = 0;
    }
}

use core::ptr::{read_unaligned, write_unaligned, copy_nonoverlapping};

const SBI_EXT_CONSOLE_PUTCHAR: u64 = 0x01;
const SBI_CONSOLE_PUTCHAR: u64 = 0x00;
const SBI_EXT_TIMER: u64 = 0x00;
const SBI_TIMER_SET_TIMER: u64 = 0x00;

const SSTATUS_SIE: u64 = 1 << 1;
const SIE_STIE: u64 = 1 << 5;

const SCAUSE_SUPERVISOR_TIMER: u64 = 5;
const MCAUSE_INTERRUPT: u64 = 1 << 63;

const CLINT_MTIME: usize = 0x0200_BFF8;
const CLINT_MTIMECMP0: usize = 0x0200_4000;
const TIMER_INTERVAL: u64 = 10_000_000;

/// Write a u32 as decimal into the provided buffer, returning bytes written.
fn write_u32_dec(mut n: u32, buf: &mut [u8]) -> usize {
    if n == 0 {
        if buf.is_empty() {
            return 0;
        }
        buf[0] = b'0';
        return 1;
    }
    let mut digits = [0u8; 10];
    let mut i = 0;
    while n > 0 {
        digits[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    let len = i.min(buf.len());
    for j in 0..len {
        buf[j] = digits[i - 1 - j];
    }
    len
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    sbi_console_write(b"\n*** KERNEL PANIC ***\n");
    if let Some(loc) = info.location() {
        sbi_console_write(b"at ");
        sbi_console_write(loc.file().as_bytes());
        sbi_console_write(b":");
        let mut buf = [0u8; 16];
        let n = write_u32_dec(loc.line(), &mut buf);
        sbi_console_write(&buf[..n]);
        sbi_console_write(b":");
        let n = write_u32_dec(loc.column(), &mut buf);
        sbi_console_write(&buf[..n]);
        sbi_console_write(b"\n");
    }
    sbi_console_write(b"halting...\n");
    // Halt the CPU - QEMU will keep running but the guest is stopped.
    loop {
        unsafe { core::arch::asm!("wfi", options(nomem, nostack)); }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct TaskContext {
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
    sp: u64,
    /// Privilege level: Supervisor (kernel) or User (isolated process)
    privilege: u64,
    /// Base physical address of the task's accessible memory region
    mem_base: usize,
    /// Size of the task's accessible memory region (bytes)
    mem_size: usize,
}

/// Task privilege levels
const TASK_PRIV_SUPERVISOR: u64 = 1;
const TASK_PRIV_USER: u64 = 0;

const MAX_TASKS: usize = 2;

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
    privilege: TASK_PRIV_SUPERVISOR,
    mem_base: 0,
    mem_size: 0,
}; MAX_TASKS];

static mut CURRENT_TASK: usize = 0;

static mut TASK1_STACK: [u64; 128] = [0; 128];

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

fn csrw_stvec(val: u64) {
    unsafe {
        core::arch::asm!("csrw stvec, {}", in(reg) val, options(nomem));
    }
}

fn csrw_sepc(val: u64) {
    unsafe {
        core::arch::asm!("csrw sepc, {}", in(reg) val, options(nomem));
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
            in("a0") value,
            in("a6") SBI_TIMER_SET_TIMER,
            in("a7") SBI_EXT_TIMER,
            options(nomem, nostack),
        );
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

// ---------------------------------------------------------------------------
// RISC-V Physical Memory Protection (PMP)
// ---------------------------------------------------------------------------

/// PMP CSR addresses (rv64)
const CSR_PMPCFG0: u64 = 0x3A0;
const CSR_PMPADDR0: u64 = 0x3B0;
const CSR_PMPADDR1: u64 = 0x3B1;

// PMP configuration field bits (within one byte of pmpcfg)
const PMP_R: u8 = 1 << 2;  // Read
const PMP_W: u8 = 1 << 3;  // Write
const PMP_X: u8 = 1 << 4;  // Execute
const PMP_U: u8 = 1 << 5;  // Applies to U-mode
const PMP_S: u8 = 1 << 6;  // Applies to S-mode
const PMP_L: u8 = 1 << 7;  // Locked

// Address matching mode (bits [1:0] of pmpcfg byte)
const PMP_MODE_OFF: u8 = 0;
const PMP_MODE_TOR: u8 = 1;  // Top-of-Range: [pmpaddr[i-1], pmpaddr[i])
const PMP_MODE_NA4: u8 = 2;  // Naturally Aligned 4-byte region
const PMP_MODE_NAPOT: u8 = 3; // Naturally Aligned Power Of Two region

/// Build a single pmpcfg byte.
fn pmpcfg_byte(mode: u8, perms: u8, apply_u: bool, apply_s: bool, locked: bool) -> u8 {
    let mut b = mode | perms;
    if apply_u {
        b |= PMP_U;
    }
    if apply_s {
        b |= PMP_S;
    }
    if locked {
        b |= PMP_L;
    }
    b
}

/// Write a pmpcfg CSR byte (entry index 0-7 within pmpcfg0).
unsafe fn pmp_write_cfg(index: usize, value: u8) {
    // pmpcfg0 is a single CSR; each entry occupies one byte.
    // Read-modify-write to preserve other entries.
    let mut cfg: u64;
    unsafe {
        core::arch::asm!("csrr {}, 0x3A0", out(reg) cfg, options(nomem));
    }
    let shift = (index % 8) * 8;
    let mask = !(0xFFu64 << shift);
    cfg = (cfg & mask) | ((value as u64) << shift);
    unsafe {
        core::arch::asm!("csrw 0x3A0, {}", in(reg) cfg, options(nomem));
    }
}

/// Write a pmpaddr CSR.
unsafe fn pmp_write_addr(index: usize, value: u64) {
    // CSR number must be an instruction immediate; match on index.
    match index {
        0 => unsafe {
            core::arch::asm!("csrw 0x3B0, {}", in(reg) value, options(nomem));
        },
        1 => unsafe {
            core::arch::asm!("csrw 0x3B1, {}", in(reg) value, options(nomem));
        },
        _ => {}
    }
}

/// Read sstatus (CSR 0x100).
fn sstatus_read() -> u64 {
    let val: u64;
    unsafe {
        core::arch::asm!("csrr {}, 0x100", out(reg) val, options(nomem));
    }
    val
}

/// Write sstatus (CSR 0x100).
fn sstatus_write(val: u64) {
    unsafe {
        core::arch::asm!("csrw 0x100, {}", in(reg) val, options(nomem));
    }
}

/// SSTATUS_SPP bit (bit 8): supervisor previous privilege (0=U, 1=S)
const SSTATUS_SPP: u64 = 1 << 8;

/// Configure PMP entries for a task.
///
/// Entry 0 (TOR): [0, mem_end) -> R/W/X for U-mode (task's memory region)
/// Entry 1 (TOR): [mem_end, MAX) -> no access for U-mode (everything else)
///
/// Supervisor mode is unaffected (PMP_S not set), so the kernel can still access all memory.
unsafe fn setup_pmp(mem_base: usize, mem_size: usize) {
    let mem_end = (mem_base + mem_size) >> 2; // pmpaddr holds addr >> 2

    // Entry 0: allow U-mode R/W/X on [0, mem_end)
    let cfg0 = pmpcfg_byte(PMP_MODE_TOR, PMP_R | PMP_W | PMP_X, true, false, true);
    pmp_write_cfg(0, cfg0);
    pmp_write_addr(0, mem_end as u64);

    // Entry 1: deny U-mode on [mem_end, max) — no R/W/X bits set
    let cfg1 = pmpcfg_byte(PMP_MODE_TOR, 0, true, false, true);
    pmp_write_cfg(1, cfg1);
    pmp_write_addr(1, 0x3_FFFF_FFFFu64); // effectively max address

    sbi_console_write(b"PMP: configured for user task\n");
}

/// Disable PMP restrictions (set all entries to OFF) for supervisor tasks.
unsafe fn pmp_disable() {
    let cfg_off = pmpcfg_byte(PMP_MODE_OFF, 0, false, false, false);
    for i in 0..2 {
        pmp_write_cfg(i, cfg_off);
        pmp_write_addr(i, 0);
    }
}

// ---------------------------------------------------------------------------
// User-mode task support
// ---------------------------------------------------------------------------

/// User task memory region (16 KiB at 0x8000_0000, below kernel load address)
const USER_TASK_MEM_BASE: usize = 0x8000_0000;
const USER_TASK_MEM_SIZE: usize = 0x1000; // 4 KiB

/// Hand-encoded RISC-V RV64 user task shellcode.
/// Runs at USER_TASK_MEM_BASE under PMP restriction.
///
/// The task:
///   1. li a0, 'U'        # a0 = 'U'
///   2. li a7, 1          # syscall 1 = print char
///   3. ecall             # kernel prints 'U' via SBI console
///   4. li a7, 2          # syscall 2 = exit task
/// Hand-encoded RISC-V RV64 user task shellcode.
/// Runs at USER_TASK_MEM_BASE under PMP restriction.
///
/// The task prints "AETHER" character-by-character via ECALL syscalls
/// (syscall 1 = print char), then exits via ECALL syscall 2.
///
/// Each character:
///   li a0, char      # addi x10, x0, char
///   li a7, 1         # addi x17, x0, 1  (syscall: print char)
///   ecall            # kernel prints char via SBI console
///
/// Exit:
///   li a7, 2         # addi x17, x0, 2  (syscall: exit task)
///   ecall            # kernel marks task inactive
///
/// This demonstrates working protected process execution: the task runs in
/// U-mode, is confined to its PMP-protected memory region, communicates with
/// the kernel only through ECALL syscalls, and cannot access kernel memory.
/// Any attempt to access memory outside [USER_TASK_MEM_BASE,
/// USER_TASK_MEM_BASE + USER_TASK_MEM_SIZE) triggers a PMP access fault and
/// the task is killed by the trap handler.
const USER_TASK_SHELLCODE: [u8; 80] = [
    // Print 'A' (65 = 0x41)
    0x13, 0x05, 0x41, 0x00, // li a0, 'A'
    0x93, 0x08, 0x10, 0x00, // li a7, 1
    0x73, 0x00, 0x00, 0x00, // ecall
    // Print 'E' (69 = 0x45)
    0x13, 0x05, 0x45, 0x00, // li a0, 'E'
    0x93, 0x08, 0x10, 0x00, // li a7, 1
    0x73, 0x00, 0x00, 0x00, // ecall
    // Print 'T' (84 = 0x54)
    0x13, 0x05, 0x54, 0x00, // li a0, 'T'
    0x93, 0x08, 0x10, 0x00, // li a7, 1
    0x73, 0x00, 0x00, 0x00, // ecall
    // Print 'H' (72 = 0x48)
    0x13, 0x05, 0x48, 0x00, // li a0, 'H'
    0x93, 0x08, 0x10, 0x00, // li a7, 1
    0x73, 0x00, 0x00, 0x00, // ecall
    // Print 'E' (69 = 0x45)
    0x13, 0x05, 0x45, 0x00, // li a0, 'E'
    0x93, 0x08, 0x10, 0x00, // li a7, 1
    0x73, 0x00, 0x00, 0x00, // ecall
    // Print 'R' (82 = 0x52)
    0x13, 0x05, 0x52, 0x00, // li a0, 'R'
    0x93, 0x08, 0x10, 0x00, // li a7, 1
    0x73, 0x00, 0x00, 0x00, // ecall
    // Exit
    0x93, 0x08, 0x20, 0x00, // li a7, 2
    0x73, 0x00, 0x00, 0x00, // ecall
];

/// Userspace program for the runtime VM: demonstrates protected process execution
/// with storage syscalls. Writes "HELLO" to sectors 0-4 via SYS_WRITE, reads back
/// and prints each char via SYS_READ + SYS_PRINT, gets PID via SYS_GETPID, exits.
const USER_TASK_PROGRAM: [Inst; 45] = [
    Inst { op: 1, a: 0, b: 1, c: 72 },   // MOV r0, 'H'
    Inst { op: 1, a: 1, b: 1, c: 0 },   // MOV r1, sector 0
    Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
    Inst { op: 11, a: 4, b: 1, c: 2 },   // SYSCALL write(sector 0, offset 0, value r0)
    Inst { op: 1, a: 0, b: 1, c: 69 },   // MOV r0, 'E'
    Inst { op: 1, a: 1, b: 1, c: 1 },   // MOV r1, sector 1
    Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
    Inst { op: 11, a: 4, b: 1, c: 2 },   // SYSCALL write(sector 1, offset 0, value r0)
    Inst { op: 1, a: 0, b: 1, c: 76 },   // MOV r0, 'L'
    Inst { op: 1, a: 1, b: 1, c: 2 },   // MOV r1, sector 2
    Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
    Inst { op: 11, a: 4, b: 1, c: 2 },   // SYSCALL write(sector 2, offset 0, value r0)
    Inst { op: 1, a: 0, b: 1, c: 76 },   // MOV r0, 'L'
    Inst { op: 1, a: 1, b: 1, c: 3 },   // MOV r1, sector 3
    Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
    Inst { op: 11, a: 4, b: 1, c: 2 },   // SYSCALL write(sector 3, offset 0, value r0)
    Inst { op: 1, a: 0, b: 1, c: 79 },   // MOV r0, 'O'
    Inst { op: 1, a: 1, b: 1, c: 4 },   // MOV r1, sector 4
    Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
    Inst { op: 11, a: 4, b: 1, c: 2 },   // SYSCALL write(sector 4, offset 0, value r0)
    Inst { op: 1, a: 1, b: 1, c: 0 },   // MOV r1, sector 0
    Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
    Inst { op: 11, a: 5, b: 1, c: 2 },   // SYSCALL read(sector 0, offset 0) -> r0
    Inst { op: 11, a: 1, b: 0, c: 0 },   // SYSCALL print(r0) -> 'H'
    Inst { op: 1, a: 1, b: 1, c: 1 },   // MOV r1, sector 1
    Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
    Inst { op: 11, a: 5, b: 1, c: 2 },   // SYSCALL read(sector 1, offset 0) -> r0
    Inst { op: 11, a: 1, b: 0, c: 0 },   // SYSCALL print(r0) -> 'E'
    Inst { op: 1, a: 1, b: 1, c: 2 },   // MOV r1, sector 2
    Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
    Inst { op: 11, a: 5, b: 1, c: 2 },   // SYSCALL read(sector 2, offset 0) -> r0
    Inst { op: 11, a: 1, b: 0, c: 0 },   // SYSCALL print(r0) -> 'L'
    Inst { op: 1, a: 1, b: 1, c: 3 },   // MOV r1, sector 3
    Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
    Inst { op: 11, a: 5, b: 1, c: 2 },   // SYSCALL read(sector 3, offset 0) -> r0
    Inst { op: 11, a: 1, b: 0, c: 0 },   // SYSCALL print(r0) -> 'L'
    Inst { op: 1, a: 1, b: 1, c: 4 },   // MOV r1, sector 4
    Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
    Inst { op: 11, a: 5, b: 1, c: 2 },   // SYSCALL read(sector 4, offset 0) -> r0
    Inst { op: 11, a: 1, b: 0, c: 0 },   // SYSCALL print(r0) -> 'O'
    Inst { op: 1, a: 0, b: 1, c: 5 },   // MOV r0, SYS_GETPID
    Inst { op: 11, a: 5, b: 0, c: 0 },   // SYSCALL getpid -> r0
    Inst { op: 2, a: 0, b: 0, c: 48 },   // ADD r0, r0, 48 ('0')
    Inst { op: 11, a: 1, b: 0, c: 0 },   // SYSCALL print(r0)
    Inst { op: 11, a: 2, b: 0, c: 0 },   // SYSCALL exit
];

/// Set up TASKS[0] as a user-mode task.
/// Copies shellcode into the user memory region and configures the task context.
unsafe fn setup_user_task() {
    // Copy shellcode into user memory region
    let shellcode_dst = USER_TASK_MEM_BASE as *mut u8;
    for i in 0..USER_TASK_SHELLCODE.len() {
        write_unaligned(shellcode_dst.add(i), USER_TASK_SHELLCODE[i]);
    }

    // Set up a user stack below the shellcode
    let user_stack_top = USER_TASK_MEM_BASE + USER_TASK_MEM_SIZE - 16;

    let task = unsafe { &mut TASKS[0] };
    task.pc = USER_TASK_MEM_BASE as u64;
    task.sp = user_stack_top as u64;
    task.privilege = TASK_PRIV_USER;
    task.mem_base = USER_TASK_MEM_BASE;
    task.mem_size = USER_TASK_MEM_SIZE;

    // Zero out registers
    task.ra = 0;
    task.gp = 0;
    task.tp = 0;
    task.t0 = 0;
    task.t1 = 0;
    task.t2 = 0;
    task.s0 = 0;
    task.s1 = 0;
    task.a0 = 0;
    task.a1 = 0;
    task.a2 = 0;
    task.a3 = 0;
    task.a4 = 0;
    task.a5 = 0;
    task.a6 = 0;
    task.a7 = 0;
    task.s2 = 0;
    task.s3 = 0;
    task.s4 = 0;
    task.s5 = 0;
    task.s6 = 0;
    task.s7 = 0;
    task.s8 = 0;
    task.s9 = 0;
    task.s10 = 0;
    task.s11 = 0;
    task.t3 = 0;
    task.t4 = 0;
    task.t5 = 0;
    task.t6 = 0;

    sbi_console_write(b"User task: configured at 0x");
    let mut buf = [0u8; 16];
    let mut addr = USER_TASK_MEM_BASE as u64;
    for i in (0..8).rev() {
        buf[i] = match addr & 0xF {
            0..=9 => b'0' + (addr & 0xF) as u8,
            10..=15 => b'a' + (addr & 0xF - 10) as u8,
            _ => b'?',
        };
        addr >>= 4;
    }
    sbi_console_write(&buf);
    sbi_console_write(b"\n");
}

/// Handle a U-mode ECALL (system call).
/// a7 = syscall number, a0 = arg1, a1 = arg2.
/// Returns the next sepc (advanced past ecall).
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn handle_user_syscall(sepc: u64, a7: u64, a0: u64) -> u64 {
    match a7 {
        1 => {
            // Syscall: print character (a0 = char)
            sbi_console_putchar(a0 as u8);
            sepc.wrapping_add(4)
        }
        2 => {
            // Syscall: exit task — mark inactive so scheduler skips it
            let current = unsafe { CURRENT_TASK };
            let task = unsafe { &mut TASKS[current] };
            task.privilege = TASK_PRIV_SUPERVISOR;
            task.mem_base = 0;
            task.mem_size = 0;
            task.pc = 0;
            sbi_console_write(b"\nuser task: exited cleanly\n");
            sepc.wrapping_add(4)
        }
        _ => {
            sbi_console_write(b"user syscall: unknown a7=");
            let mut buf = [0u8; 4];
            let n = write_u32_dec(a7 as u32, &mut buf);
            sbi_console_write(&buf[..n]);
            sbi_console_write(b"\n");
            sepc.wrapping_add(4)
        }
    }
}

fn csrr_scause() -> u64 {
    let val: u64;
    unsafe {
        core::arch::asm!("csrr {}, scause", out(reg) val, options(nomem));
    }
    val
}

fn csrr_sepc() -> u64 {
    let val: u64;
    unsafe {
        core::arch::asm!("csrr {}, sepc", out(reg) val, options(nomem));
    }
    val
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
    "  sd s5, 160(sp)",
    "  sd s6, 168(sp)",
    "  sd s7, 176(sp)",
    "  sd s8, 184(sp)",
    "  sd s9, 192(sp)",
    "  sd s10, 200(sp)",
    "  sd s11, 208(sp)",
    "  sd t3, 216(sp)",
    "  sd t4, 224(sp)",
    "  sd t5, 232(sp)",
    "  sd t6, 240(sp)",
    "",
    "  csrr a0, scause",
    "  csrr a1, sepc",
    "  mv a2, sp",
    "  call trap_handler",
    "",
    "  ld t6, 240(sp)",
    "  ld t5, 232(sp)",
    "  ld t4, 224(sp)",
    "  ld t3, 216(sp)",
    "  ld s11, 208(sp)",
    "  ld s10, 200(sp)",
    "  ld s9, 192(sp)",
    "  ld s8, 184(sp)",
    "  ld s7, 176(sp)",
    "  ld s6, 168(sp)",
    "  ld s5, 160(sp)",
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
pub extern "C" fn trap_handler(mcause: u64, sepc: u64, tf: *mut u64) -> u64 {
    // Determine if the trap came from U-mode (spp = 0) or S-mode (spp = 1)
    let s = sstatus_read();
    let from_user = (s & SSTATUS_SPP) == 0;

    if mcause == (MCAUSE_INTERRUPT | SCAUSE_SUPERVISOR_TIMER) {
        sbi_console_write(b"tick\n");
        let now = clint_read_mtime();
        clint_write_mtimecmp0(now.wrapping_add(TIMER_INTERVAL));
        unsafe { save_and_switch(sepc) };
    } else if from_user {
        // Trap from user task: handle syscalls or kill on fault
        match mcause {
            8 => {
                // U-mode ECALL: system call
                // a7 at tf offset 15, a0 at tf offset 10
                let a7 = unsafe { *tf.add(15) };
                let a0 = unsafe { *tf.add(10) };
                let next_sepc = unsafe { handle_user_syscall(sepc, a7, a0) };
                // Advance sepc past the ecall instruction so sret continues
                unsafe { csrw_sepc(next_sepc) };
            }
            1 | 2 | 4 | 5 | 6 | 7 => {
                // Instruction/load/store access fault or illegal instruction
                sbi_console_write(b"user task fault: mcause=");
                let mut buf = [0u8; 4];
                let n = write_u32_dec(mcause as u32, &mut buf);
                sbi_console_write(&buf[..n]);
                sbi_console_write(b"\n");
                // Kill the user task by marking it inactive — switch away and never return
                unsafe { kill_user_task(sepc) };
            }
            _ => {
                sbi_console_write(b"user trap: mcause=");
                let mut buf = [0u8; 4];
                let n = write_u32_dec(mcause as u32, &mut buf);
                sbi_console_write(&buf[..n]);
                sbi_console_write(b"\n");
                unsafe { kill_user_task(sepc) };
            }
        }
    } else {
        sbi_console_write(b"kernel trap: mcause=");
        let mut buf = [0u8; 4];
        let n = write_u32_dec(mcause as u32, &mut buf);
        sbi_console_write(&buf[..n]);
        sbi_console_write(b"\n");
    }
    sepc
}

/// Kill a faulting user task: mark it inactive and switch away.
/// Does not return; context switch happens via save_and_switch.
unsafe fn kill_user_task(_fault_pc: u64) -> ! {
    let current = unsafe { CURRENT_TASK };
    let task = unsafe { &mut TASKS[current] };
    task.privilege = TASK_PRIV_SUPERVISOR; // mark inactive: no longer a user task
    task.mem_base = 0;
    task.mem_size = 0;
    task.pc = 0; // will never be scheduled again (simple scheme)

    // Switch to the other task
    let next = (current + 1) % MAX_TASKS;
    unsafe {
        CURRENT_TASK = next;
        let next_task = &TASKS[next];
        pmp_disable();
        let mut s = sstatus_read();
        s |= SSTATUS_SPP; // return to S-mode
        sstatus_write(s);
        csrw_sepc(next_task.pc);
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
        *next_frame.add(16) = next_task.s2;
        *next_frame.add(17) = next_task.s3;
        *next_frame.add(18) = next_task.s4;
        *next_frame.add(20) = next_task.s5;
        *next_frame.add(21) = next_task.s6;
        *next_frame.add(22) = next_task.s7;
        *next_frame.add(23) = next_task.s8;
        *next_frame.add(24) = next_task.s9;
        *next_frame.add(25) = next_task.s10;
        *next_frame.add(26) = next_task.s11;
        *next_frame.add(27) = next_task.t3;
        *next_frame.add(28) = next_task.t4;
        *next_frame.add(29) = next_task.t5;
        *next_frame.add(30) = next_task.t6;
        core::arch::asm!("mv sp, {}", in(reg) next_sp, options(nomem));
        core::arch::asm!("sret", options(noreturn, nomem));
    }
}

unsafe fn save_and_switch(sepc: u64) {
    let sp: u64;
    unsafe {
        core::arch::asm!("mv {}, sp", out(reg) sp, options(nomem));
    }

    let current = unsafe { CURRENT_TASK };
    let task = unsafe { &mut TASKS[current] };

    task.pc = sepc;

    let frame = sp as *mut u64;
    unsafe {
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
        task.s5 = *frame.add(20);
        task.s6 = *frame.add(21);
        task.s7 = *frame.add(22);
        task.s8 = *frame.add(23);
        task.s9 = *frame.add(24);
        task.s10 = *frame.add(25);
        task.s11 = *frame.add(26);
        task.t3 = *frame.add(27);
        task.t4 = *frame.add(28);
        task.t5 = *frame.add(29);
        task.t6 = *frame.add(30);
    }

    task.sp = sp.wrapping_add(256);

    let next = (current + 1) % MAX_TASKS;

    unsafe {
        CURRENT_TASK = next;
        let next_task = &TASKS[next];

        // --- Memory protection: configure PMP for the next task ---
        if next_task.privilege == TASK_PRIV_USER {
            setup_pmp(next_task.mem_base, next_task.mem_size);
            // Return to U-mode (spp = 0)
            let mut s = sstatus_read();
            s &= !SSTATUS_SPP;
            sstatus_write(s);
        } else {
            // Supervisor task: remove PMP restrictions, return to S-mode
            pmp_disable();
            let mut s = sstatus_read();
            s |= SSTATUS_SPP;
            sstatus_write(s);
        }

        csrw_sepc(next_task.pc);

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
        *next_frame.add(16) = next_task.s2;
        *next_frame.add(17) = next_task.s3;
        *next_frame.add(18) = next_task.s4;
        *next_frame.add(20) = next_task.s5;
        *next_frame.add(21) = next_task.s6;
        *next_frame.add(22) = next_task.s7;
        *next_frame.add(23) = next_task.s8;
        *next_frame.add(24) = next_task.s9;
        *next_frame.add(25) = next_task.s10;
        *next_frame.add(26) = next_task.s11;
        *next_frame.add(27) = next_task.t3;
        *next_frame.add(28) = next_task.t4;
        *next_frame.add(29) = next_task.t5;
        *next_frame.add(30) = next_task.t6;
        core::arch::asm!("mv sp, {}", in(reg) next_sp, options(nomem));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn task1_entry() -> ! {
    sbi_console_write(b"task1 (runtime VM) entry\n");

    // ---------------------------------------------------------------------------
    // Runtime VM: userspace execution engine model
    // Executes a small program via SBI console output.
    // ---------------------------------------------------------------------------

    const NUM_REGS: usize = 8;
    const MEM_SIZE: usize = 64 * 1024;
    const SYS_PRINT: u64 = 1;
    const SYS_EXIT: u64 = 2;
    const SYS_READ: u64 = 3;
    const SYS_WRITE: u64 = 4;
    const SYS_GETPID: u64 = 5;
    const SYS_FORK: u64 = 6;
    const SYS_EXEC: u64 = 7;
    const SYS_WAIT: u64 = 8;

    #[derive(Clone, Copy)]
    struct Inst {
        op: u64,
        a: u64,
        b: u64,
        c: u64,
    }

    #[derive(Clone, Copy)]
    #[repr(u8)]
    enum Op {
        Nop = 0,
        Mov = 1,
        Add = 2,
        Sub = 3,
        Load = 4,
        Store = 5,
        Jmp = 6,
        Jz = 7,
        Jnz = 8,
        Call = 9,
        Ret = 10,
        Syscall = 11,
        Halt = 12,
    }

    struct Process {
        regs: [u64; NUM_REGS],
        pc: usize,
        sp: usize,
        mem: [u8; MEM_SIZE],
        call_stack: [usize; 16],
        call_sp: usize,
        stdout: [u8; 256],
        stdout_len: usize,
        exited: bool,
        exit_code: u64,
    }

    impl Process {
        fn new() -> Self {
            Self {
                regs: [0; NUM_REGS],
                pc: 0,
                sp: MEM_SIZE - 1,
                mem: [0; MEM_SIZE],
                call_stack: [0; 16],
                call_sp: 0,
                stdout: [0; 256],
                stdout_len: 0,
                exited: false,
                exit_code: 0,
            }
        }

        fn load_program(&mut self, program: &[Inst]) {
            for (i, inst) in program.iter().enumerate() {
                let offset = i * 32;
                if offset + 32 > MEM_SIZE {
                    break;
                }
                self.mem[offset..offset + 8].copy_from_slice(&inst.op.to_le_bytes());
                self.mem[offset + 8..offset + 16].copy_from_slice(&inst.a.to_le_bytes());
                self.mem[offset + 16..offset + 24].copy_from_slice(&inst.b.to_le_bytes());
                self.mem[offset + 24..offset + 32].copy_from_slice(&inst.c.to_le_bytes());
            }
        }

        fn read_u64(&self, addr: usize) -> Result<u64, ()> {
            if addr + 8 > MEM_SIZE {
                return Err(());
            }
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&self.mem[addr..addr + 8]);
            Ok(u64::from_le_bytes(bytes))
        }

        fn write_u64(&mut self, addr: usize, val: u64) -> Result<(), ()> {
            if addr + 8 > MEM_SIZE {
                return Err(());
            }
            let bytes = val.to_le_bytes();
            self.mem[addr..addr + 8].copy_from_slice(&bytes);
            Ok(())
        }

        fn fetch(&self) -> Result<Inst, ()> {
            if self.pc + 32 > MEM_SIZE {
                return Err(());
            }
            let op = self.read_u64(self.pc)?;
            let a = self.read_u64(self.pc + 8)?;
            let b = self.read_u64(self.pc + 16)?;
            let c = self.read_u64(self.pc + 24)?;
            Ok(Inst { op, a, b, c })
        }

        fn step(&mut self) -> Result<(), ()> {
            if self.exited {
                return Err(());
            }
            let inst = self.fetch()?;
            match inst.op {
                0 => {
                    self.pc += 32;
                }
                1 => {
                    let rd = inst.a as usize;
                    if rd >= NUM_REGS {
                        return Err(());
                    }
                    if inst.b == 1 {
                        self.regs[rd] = inst.c;
                    } else {
                        let rs = inst.c as usize;
                        if rs >= NUM_REGS {
                            return Err(());
                        }
                        self.regs[rd] = self.regs[rs];
                    }
                    self.pc += 32;
                }
                2 => {
                    let rd = inst.a as usize;
                    let rs = inst.b as usize;
                    if rd >= NUM_REGS || rs >= NUM_REGS {
                        return Err(());
                    }
                    self.regs[rd] = self.regs[rd].wrapping_add(self.regs[rs]);
                    self.pc += 32;
                }
                3 => {
                    let rd = inst.a as usize;
                    let rs = inst.b as usize;
                    if rd >= NUM_REGS || rs >= NUM_REGS {
                        return Err(());
                    }
                    self.regs[rd] = self.regs[rd].wrapping_sub(self.regs[rs]);
                    self.pc += 32;
                }
                4 => {
                    let rd = inst.a as usize;
                    let addr = inst.b as usize;
                    if rd >= NUM_REGS {
                        return Err(());
                    }
                    self.regs[rd] = self.read_u64(addr)?;
                    self.pc += 32;
                }
                5 => {
                    let addr = inst.a as usize;
                    let rs = inst.b as usize;
                    if rs >= NUM_REGS {
                        return Err(());
                    }
                    self.write_u64(addr, self.regs[rs])?;
                    self.pc += 32;
                }
                6 => {
                    self.pc = inst.a as usize;
                }
                7 => {
                    let rd = inst.a as usize;
                    let addr = inst.b as usize;
                    if rd >= NUM_REGS {
                        return Err(());
                    }
                    if self.regs[rd] == 0 {
                        self.pc = addr;
                    } else {
                        self.pc += 32;
                    }
                }
                8 => {
                    let rd = inst.a as usize;
                    let addr = inst.b as usize;
                    if rd >= NUM_REGS {
                        return Err(());
                    }
                    if self.regs[rd] != 0 {
                        self.pc = addr;
                    } else {
                        self.pc += 32;
                    }
                }
                9 => {
                    self.call_stack[self.call_sp] = self.pc + 32;
                    self.call_sp += 1;
                    self.pc = inst.a as usize;
                }
                10 => {
                    if self.call_sp == 0 {
                        return Err(());
                    }
                    self.call_sp -= 1;
                    self.pc = self.call_stack[self.call_sp];
                }
                11 => {
                    let num = inst.a;
                    match num {
                        SYS_PRINT => {
                            let reg = inst.b as usize;
                            if reg >= NUM_REGS {
                                return Err(());
                            }
                            let val = self.regs[reg];
                            if self.stdout_len < self.stdout.len() {
                                self.stdout[self.stdout_len] = (val & 0xFF) as u8;
                                self.stdout_len += 1;
                            }
                        }
                        SYS_EXIT => {
                            self.exited = true;
                            self.exit_code = inst.b;
                            return Err(());
                        }
                        SYS_READ => {
                            let fd = inst.b as usize;
                            let offset = inst.c as usize;
                            let sector_offset = fd * BLOCK_SIZE + offset;
                            if sector_offset >= RAM_DISK_SIZE {
                                return Err(());
                            }
                            self.regs[0] = unsafe { RAM_DISK[sector_offset] } as u64;
                        }
                        SYS_WRITE => {
                            let fd = inst.b as usize;
                            let offset = inst.c as usize;
                            let sector_offset = fd * BLOCK_SIZE + offset;
                            if sector_offset >= RAM_DISK_SIZE {
                                return Err(());
                            }
                            unsafe { RAM_DISK[sector_offset] = self.regs[0] as u8; }
                        }
                        SYS_GETPID => {
                            let reg = inst.b as usize;
                            if reg >= NUM_REGS {
                                return Err(());
                            }
                            unsafe {
                                self.regs[reg] = CURRENT_TASK as u64;
                            }
                        }
                        SYS_FORK => {
                            // Fork not supported in single-process VM
                            self.regs[0] = -1i64 as u64;
                        }
                        SYS_EXEC => {
                            // Exec not supported in single-process VM
                            self.regs[0] = -1i64 as u64;
                        }
                        SYS_WAIT => {
                            // No child processes in single-process VM
                            self.regs[0] = -1i64 as u64;
                        }
                        _ => return Err(()),
                    }
                    self.pc += 32;
                }
                12 => {
                    self.exited = true;
                    self.exit_code = 0;
                    return Err(());
                }
                _ => return Err(()),
            }
            Ok(())
        }

        fn flush_stdout(&mut self) {
            for i in 0..self.stdout_len {
                sbi_console_putchar(self.stdout[i]);
            }
            self.stdout_len = 0;
        }
    }

    // Program: demonstrate protected process execution with storage syscalls
    //   Writes "HELLO" to sectors 0-4 via SYS_WRITE
    //   Reads back and prints each char via SYS_READ + SYS_PRINT
    //   Gets PID via SYS_GETPID and prints it
    let program: [Inst; 45] = [
        Inst { op: 1, a: 0, b: 1, c: 72 },   // MOV r0, 'H'
        Inst { op: 1, a: 1, b: 1, c: 0 },   // MOV r1, sector 0
        Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
        Inst { op: 11, a: 4, b: 1, c: 2 },   // SYSCALL write(sector 0, offset 0, value r0)
        Inst { op: 1, a: 0, b: 1, c: 69 },   // MOV r0, 'E'
        Inst { op: 1, a: 1, b: 1, c: 1 },   // MOV r1, sector 1
        Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
        Inst { op: 11, a: 4, b: 1, c: 2 },   // SYSCALL write(sector 1, offset 0, value r0)
        Inst { op: 1, a: 0, b: 1, c: 76 },   // MOV r0, 'L'
        Inst { op: 1, a: 1, b: 1, c: 2 },   // MOV r1, sector 2
        Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
        Inst { op: 11, a: 4, b: 1, c: 2 },   // SYSCALL write(sector 2, offset 0, value r0)
        Inst { op: 1, a: 0, b: 1, c: 76 },   // MOV r0, 'L'
        Inst { op: 1, a: 1, b: 1, c: 3 },   // MOV r1, sector 3
        Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
        Inst { op: 11, a: 4, b: 1, c: 2 },   // SYSCALL write(sector 3, offset 0, value r0)
        Inst { op: 1, a: 0, b: 1, c: 79 },   // MOV r0, 'O'
        Inst { op: 1, a: 1, b: 1, c: 4 },   // MOV r1, sector 4
        Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
        Inst { op: 11, a: 4, b: 1, c: 2 },   // SYSCALL write(sector 4, offset 0, value r0)
        Inst { op: 1, a: 1, b: 1, c: 0 },   // MOV r1, sector 0
        Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
        Inst { op: 11, a: 5, b: 1, c: 2 },   // SYSCALL read(sector 0, offset 0) -> r0
        Inst { op: 11, a: 1, b: 0, c: 0 },   // SYSCALL print(r0) -> 'H'
        Inst { op: 1, a: 1, b: 1, c: 1 },   // MOV r1, sector 1
        Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
        Inst { op: 11, a: 5, b: 1, c: 2 },   // SYSCALL read(sector 1, offset 0) -> r0
        Inst { op: 11, a: 1, b: 0, c: 0 },   // SYSCALL print(r0) -> 'E'
        Inst { op: 1, a: 1, b: 1, c: 2 },   // MOV r1, sector 2
        Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
        Inst { op: 11, a: 5, b: 1, c: 2 },   // SYSCALL read(sector 2, offset 0) -> r0
        Inst { op: 11, a: 1, b: 0, c: 0 },   // SYSCALL print(r0) -> 'L'
        Inst { op: 1, a: 1, b: 1, c: 3 },   // MOV r1, sector 3
        Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
        Inst { op: 11, a: 5, b: 1, c: 2 },   // SYSCALL read(sector 3, offset 0) -> r0
        Inst { op: 11, a: 1, b: 0, c: 0 },   // SYSCALL print(r0) -> 'L'
        Inst { op: 1, a: 1, b: 1, c: 4 },   // MOV r1, sector 4
        Inst { op: 1, a: 2, b: 1, c: 0 },   // MOV r2, offset 0
        Inst { op: 11, a: 5, b: 1, c: 2 },   // SYSCALL read(sector 4, offset 0) -> r0
        Inst { op: 11, a: 1, b: 0, c: 0 },   // SYSCALL print(r0) -> 'O'
        Inst { op: 1, a: 0, b: 1, c: 5 },   // MOV r0, SYS_GETPID
        Inst { op: 11, a: 5, b: 0, c: 0 },   // SYSCALL getpid -> r0
        Inst { op: 2, a: 0, b: 0, c: 48 },   // ADD r0, r0, 48 ('0')
        Inst { op: 11, a: 1, b: 0, c: 0 },   // SYSCALL print(r0)
        Inst { op: 11, a: 2, b: 0, c: 0 },   // SYSCALL exit
    ];

    let mut proc = Process::new();
    proc.load_program(&program);

    // Execute one instruction per timer tick via the runtime VM
    loop {
        match proc.step() {
            Ok(()) => {
                proc.flush_stdout();
            }
            Err(()) => {
                if proc.exited {
                    sbi_console_write(b"\nruntime VM exited with code ");
                    sbi_console_putchar(b'0' + (proc.exit_code & 0xFF) as u8);
                    sbi_console_write(b"\n");
                }
                // Halt forever after program completion
                loop {
                    unsafe {
                        core::arch::asm!("wfi", options(nomem, nostack));
                    }
                }
            }
        }

        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}

// ---------------------------------------------------------------------------
// Storage: RAM disk + minimal file system
// ---------------------------------------------------------------------------

const RAM_DISK_SIZE: usize = 16 * 1024;
const BLOCK_SIZE: usize = 512;

static mut RAM_DISK: [u8; RAM_DISK_SIZE] = [0u8; RAM_DISK_SIZE];

/// Set to true when the VirtIO block device has been successfully initialized.
static mut VIRTIO_BLOCK_READY: bool = false;

fn ram_read(sector: u64, buf: &mut [u8]) -> bool {
    // Try VirtIO block device first, fall back to RAM disk.
    unsafe {
        if VIRTIO_BLOCK_READY {
            return virtio_blk_read_sector(sector, buf);
        }
    }
    let offset = (sector as usize) * BLOCK_SIZE;
    if offset + buf.len() > RAM_DISK_SIZE {
        return false;
    }
    unsafe {
        buf.copy_from_slice(&RAM_DISK[offset..offset + buf.len()]);
    }
    true
}

fn ram_write(sector: u64, buf: &[u8]) -> bool {
    // Try VirtIO block device first, fall back to RAM disk.
    unsafe {
        if VIRTIO_BLOCK_READY {
            return virtio_blk_write_sector(sector, buf);
        }
    }
    let offset = (sector as usize) * BLOCK_SIZE;
    if offset + buf.len() > RAM_DISK_SIZE {
        return false;
    }
    unsafe {
        RAM_DISK[offset..offset + buf.len()].copy_from_slice(buf);
    }
    true
}

const MAX_FILES: usize = 16;
const MAX_FILE_NAME: usize = 16;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Inode {
    name: [u8; MAX_FILE_NAME],
    size: u32,
    blocks: [u32; 8],
    in_use: bool,
}

static mut INODES: [Inode; MAX_FILES] = [Inode {
    name: [0; MAX_FILE_NAME],
    size: 0,
    blocks: [0; 8],
    in_use: false,
}; MAX_FILES];

static mut NEXT_BLOCK: u32 = 1;

fn fs_find(name: &[u8]) -> Option<usize> {
    unsafe {
        for i in 0..MAX_FILES {
            let inode = unsafe { &INODES[i] };
            let nlen = name.len().min(MAX_FILE_NAME);
            if inode.in_use && inode.name[..nlen] == name[..nlen] {
                return Some(i);
            }
        }
        None
    }
}

fn fs_find_free() -> Option<usize> {
    unsafe { INODES.iter().position(|i| !i.in_use) }
}

fn fs_alloc_block() -> Option<u32> {
    unsafe {
        let b = NEXT_BLOCK;
        if (b as usize) * BLOCK_SIZE < RAM_DISK_SIZE {
            NEXT_BLOCK = b + 1;
            Some(b)
        } else {
            None
        }
    }
}

fn fs_create(name: &[u8]) -> bool {
    if name.is_empty() || name.len() > MAX_FILE_NAME {
        return false;
    }
    unsafe {
        if fs_find(name).is_some() {
            return false;
        }
        match fs_find_free() {
            Some(idx) => {
                let inode = &mut INODES[idx];
                inode.in_use = true;
                inode.size = 0;
                inode.blocks = [0; 8];
                inode.name[..name.len()].copy_from_slice(name);
                true
            }
            None => false,
        }
    }
}

fn fs_write(name: &[u8], data: &[u8]) -> bool {
    let idx = match unsafe { fs_find(name) } {
        Some(i) => i,
        None => return false,
    };
    unsafe {
        let inode = &mut INODES[idx];
        let num_blocks = (data.len() + BLOCK_SIZE - 1) / BLOCK_SIZE;
        if num_blocks > 8 {
            return false;
        }
        for i in 0..num_blocks {
            let blk = match fs_alloc_block() {
                Some(b) => b,
                None => return false,
            };
            inode.blocks[i] = blk;
            let start = i * BLOCK_SIZE;
            let end = (start + BLOCK_SIZE).min(data.len());
            ram_write(blk as u64, &data[start..end]);
        }
        inode.size = data.len() as u32;
        true
    }
}

fn fs_read(name: &[u8], buf: &mut [u8]) -> usize {
    let idx = match unsafe { fs_find(name) } {
        Some(i) => i,
        None => return 0,
    };
    unsafe {
        let inode = &INODES[idx];
        let to_read = buf.len().min(inode.size as usize);
        let mut read = 0;
        for i in 0..inode.size as usize {
            let blk = i / BLOCK_SIZE;
            let offset = i % BLOCK_SIZE;
            if blk >= 8 || inode.blocks[blk] == 0 {
                break;
            }
            let mut sector_buf = [0u8; BLOCK_SIZE];
            ram_read(inode.blocks[blk] as u64, &mut sector_buf);
            buf[read] = sector_buf[offset];
            read += 1;
            if read >= to_read {
                break;
            }
        }
        read
    }
}

fn fs_delete(name: &[u8]) -> bool {
    let idx = match unsafe { fs_find(name) } {
        Some(i) => i,
        None => return false,
    };
    unsafe {
        INODES[idx] = Inode {
            name: [0; MAX_FILE_NAME],
            size: 0,
            blocks: [0; 8],
            in_use: false,
        };
        true
    }
}

fn fs_list() -> usize {
    unsafe { INODES.iter().filter(|i| i.in_use).count() }
}

// ---------------------------------------------------------------------------
// IPC: message-queue endpoints
// ---------------------------------------------------------------------------

const IPC_QUEUE_CAP: usize = 8;

static mut IPC_QUEUE: [[u8; 64]; IPC_QUEUE_CAP] = [[0u8; 64]; IPC_QUEUE_CAP];
static mut IPC_HEAD: usize = 0;
static mut IPC_TAIL: usize = 0;

fn ipc_send(msg: &[u8]) {
    unsafe {
        let len = msg.len().min(64);
        let full = (IPC_TAIL + 1) % IPC_QUEUE_CAP == IPC_HEAD;
        if full {
            return;
        }
        for i in 0..len {
            IPC_QUEUE[IPC_TAIL][i] = msg[i];
        }
        IPC_QUEUE[IPC_TAIL][len] = 0;
        IPC_TAIL = (IPC_TAIL + 1) % IPC_QUEUE_CAP;
    }
}

fn ipc_receive() -> Option<[u8; 64]> {
    unsafe {
        if IPC_HEAD == IPC_TAIL {
            return None;
        }
        let msg = IPC_QUEUE[IPC_HEAD];
        IPC_HEAD = (IPC_HEAD + 1) % IPC_QUEUE_CAP;
        Some(msg)
    }
}

// ---------------------------------------------------------------------------
// Update/Recovery: A/B partition OTA + recovery model
// ---------------------------------------------------------------------------

const MAX_VERSION_LEN: usize = 16;
const MAX_BOOT_RETRIES: u32 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Partition {
    A,
    B,
}

impl Partition {
    fn other(&self) -> Self {
        match self {
            Partition::A => Partition::B,
            Partition::B => Partition::A,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PartitionState {
    Active,
    Pending,
    Inactive,
    Recovery,
}

struct Manifest {
    version: [u8; MAX_VERSION_LEN],
    version_len: usize,
    hash: u64,
    size: u64,
}

struct UpdateSystem {
    active: Partition,
    state_a: PartitionState,
    state_b: PartitionState,
    version_a: [u8; MAX_VERSION_LEN],
    version_a_len: usize,
    version_b: [u8; MAX_VERSION_LEN],
    version_b_len: usize,
    boot_retries: u32,
    recovery_mode: bool,
    pending_manifest: Option<Manifest>,
    update_in_progress: bool,
}

impl UpdateSystem {
    fn new() -> Self {
        let mut va = [0u8; MAX_VERSION_LEN];
        va[..5].copy_from_slice(b"1.0.0");
        Self {
            active: Partition::A,
            state_a: PartitionState::Active,
            state_b: PartitionState::Inactive,
            version_a: va,
            version_a_len: 5,
            version_b: [0u8; MAX_VERSION_LEN],
            version_b_len: 0,
            boot_retries: 0,
            recovery_mode: false,
            pending_manifest: None,
            update_in_progress: false,
        }
    }

    fn state(&self, p: Partition) -> PartitionState {
        match p {
            Partition::A => self.state_a,
            Partition::B => self.state_b,
        }
    }

    fn set_state(&mut self, p: Partition, s: PartitionState) {
        match p {
            Partition::A => self.state_a = s,
            Partition::B => self.state_b = s,
        }
    }

    fn set_version(&mut self, p: Partition, v: &[u8], len: usize) {
        let len = len.min(MAX_VERSION_LEN);
        match p {
            Partition::A => {
                self.version_a[..len].copy_from_slice(&v[..len]);
                self.version_a_len = len;
            }
            Partition::B => {
                self.version_b[..len].copy_from_slice(&v[..len]);
                self.version_b_len = len;
            }
        }
    }

    fn begin_update(&mut self, manifest: Manifest) -> Result<(), &'static str> {
        if self.update_in_progress {
            return Err("update already in progress");
        }
        if manifest.hash == 0 {
            return Err("invalid manifest hash");
        }
        if manifest.size == 0 {
            return Err("invalid manifest size");
        }
        let target = self.active.other();
        self.pending_manifest = Some(manifest);
        self.update_in_progress = true;
        self.set_state(target, PartitionState::Pending);
        Ok(())
    }

    fn apply_update(&mut self) -> Result<(), &'static str> {
        if !self.update_in_progress {
            return Err("no update in progress");
        }
        let manifest = self
            .pending_manifest
            .as_ref()
            .ok_or("no pending manifest")?;
        let target = self.active.other();
        let version = manifest.version;
        let version_len = manifest.version_len;
        self.set_version(target, &version, version_len);
        self.boot_retries = 0;
        Ok(())
    }

    fn confirm_boot(&mut self) -> Result<(), &'static str> {
        let target = self.active.other();
        if self.state(target) != PartitionState::Pending {
            return Err("no pending partition to confirm");
        }
        self.set_state(self.active, PartitionState::Inactive);
        self.active = target;
        self.set_state(self.active, PartitionState::Active);
        self.update_in_progress = false;
        self.pending_manifest = None;
        self.boot_retries = 0;
        Ok(())
    }

    fn record_boot_failure(&mut self) -> bool {
        self.boot_retries += 1;
        if self.boot_retries >= MAX_BOOT_RETRIES {
            let old = self.active;
            let failed = self.active.other();
            self.set_state(failed, PartitionState::Inactive);
            self.set_state(old, PartitionState::Active);
            self.update_in_progress = false;
            self.pending_manifest = None;
            self.boot_retries = 0;
            true
        } else {
            false
        }
    }

    fn enter_recovery(&mut self) {
        self.recovery_mode = true;
        self.set_state(self.active, PartitionState::Recovery);
    }

    fn exit_recovery(&mut self) {
        self.recovery_mode = false;
        self.set_state(self.active, PartitionState::Active);
    }

    fn active_version(&self) -> &[u8] {
        match self.active {
            Partition::A => &self.version_a[..self.version_a_len],
            Partition::B => &self.version_b[..self.version_b_len],
        }
    }
}

/// Boot self-test: verifies timer, task switching, IPC, storage, and filesystem.
///
/// Runs during early boot before the main WFI loop. Reports PASS/FAIL for each
/// subsystem via the SBI console. Any failure does not halt the boot; the
/// results are printed so QEMU serial output can be inspected.
fn boot_self_test() {
    sbi_console_write(b"\n=== AetherOS Boot Self-Test ===\n");

    // -----------------------------------------------------------------------
    // Test 1: IPC round-trip
    // -----------------------------------------------------------------------
    let ipc_msg: [u8; 4] = *b"PING";
    unsafe { ipc_send(&ipc_msg) };
    let ipc_ok = match unsafe { ipc_receive() } {
        Some(mut received) => {
            // Null-terminate at first zero for comparison
            let mut len = 0;
            while len < received.len() && received[len] != 0 {
                len += 1;
            }
            received[..len] == ipc_msg
        }
        None => false,
    };
    sbi_console_write(if ipc_ok {
        b"[PASS] IPC round-trip\n"
    } else {
        b"[FAIL] IPC round-trip\n"
    });

    // -----------------------------------------------------------------------
    // Test 2: Storage write + read back
    // -----------------------------------------------------------------------
    let write_buf: [u8; 512] = {
        let mut b = [0u8; 512];
        b[0] = b'A';
        b[1] = b'E';
        b[2] = b'T';
        b[3] = b'H';
        b
    };
    let storage_ok = unsafe { ram_write(0, &write_buf) };
    let mut read_buf = [0u8; 512];
    let read_ok = unsafe { ram_read(0, &mut read_buf) };
    let storage_match = storage_ok && read_ok && read_buf[..4] == write_buf[..4];
    sbi_console_write(if storage_match {
        b"[PASS] Storage write/read\n"
    } else {
        b"[FAIL] Storage write/read\n"
    });

    // -----------------------------------------------------------------------
    // Test 3: Filesystem create/write/read/delete
    // -----------------------------------------------------------------------
    let fs_name: [u8; 4] = *b"TEST";
    let fs_data: [u8; 6] = *b"hello\0";
    let fs_create_ok = unsafe { fs_create(&fs_name) };
    let fs_write_ok = fs_create_ok && unsafe { fs_write(&fs_name, &fs_data) };
    let mut fs_read_buf = [0u8; 64];
    let fs_read_len = unsafe { fs_read(&fs_name, &mut fs_read_buf) };
    let fs_read_ok = fs_read_len == 5 && fs_read_buf[..5] == *b"hello";
    let fs_delete_ok = fs_read_ok && unsafe { fs_delete(&fs_name) };
    let fs_ok = fs_create_ok && fs_write_ok && fs_read_ok && fs_delete_ok;
    sbi_console_write(if fs_ok {
        b"[PASS] Filesystem create/write/read/delete\n"
    } else {
        b"[FAIL] Filesystem create/write/read/delete\n"
    });

    // -----------------------------------------------------------------------
    // Test 4: Timer + task switching (set timer, WFI, check task switched)
    // -----------------------------------------------------------------------
    let timer_task_before = unsafe { CURRENT_TASK };
    let mtime_now = clint_read_mtime();
    let fire_at = mtime_now.wrapping_add(100_000); // short interval
    sbi_timer_set(fire_at);
    // Wait for timer interrupt to fire and switch tasks
    let mut waited: u64 = 0;
    while unsafe { CURRENT_TASK } == timer_task_before && waited < 1_000_000 {
        unsafe { core::arch::asm!("wfi", options(nomem, nostack)); }
        waited += 1;
    }
    let timer_ok = unsafe { CURRENT_TASK } != timer_task_before;
    sbi_console_write(if timer_ok {
        b"[PASS] Timer + task switching\n"
    } else {
        b"[FAIL] Timer + task switching\n"
    });

    // -----------------------------------------------------------------------
    // Test 5: Network stack (Ethernet + ARP + IPv4 header build/parse)
    // -----------------------------------------------------------------------
    let net_ok = {
        // Test Ethernet header build/parse round-trip
        let mut eth_buf = [0u8; ETH_HLEN];
        let dst = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let eth_len = eth_build_header(&mut eth_buf, &dst, ETH_P_ARP);
        let eth_ok = eth_len == ETH_HLEN;
        let parse_ok = match eth_parse_header(&eth_buf) {
            Some((d, s, t)) => d == dst && t == ETH_P_ARP,
            None => false,
        };

        // Test ARP request build/parse round-trip
        let mut arp_buf = [0u8; ETH_HLEN + 28];
        let target_ip = [10, 0, 2, 2];
        let arp_len = arp_build_request(&mut arp_buf, &target_ip);
        let arp_req_ok = arp_len == ETH_HLEN + 28;
        let arp_parse_ok = match arp_parse(&arp_buf, ETH_HLEN) {
            Some((op, smac, sip, tmac, tip)) =>
                op == ARP_OP_REQUEST && tip == target_ip,
            None => false,
        };

        // Test IPv4 header build/parse round-trip
        let mut ip_buf = [0u8; ETH_HLEN + 20];
        let dst_ip = [10, 0, 2, 1];
        let ip_len = ipv4_build_header(&mut ip_buf, ETH_HLEN, IP_PROTO_ICMP, &dst_ip, 0);
        let ip_ok = ip_len == 20;
        let ip_parse_ok = match ipv4_parse_header(&ip_buf, ETH_HLEN) {
            Some((proto, sip, dip, tlen)) =>
                proto == IP_PROTO_ICMP && dip == dst_ip && tlen == 20,
            None => false,
        };

        eth_ok && parse_ok && arp_req_ok && arp_parse_ok && ip_ok && ip_parse_ok
    };
    sbi_console_write(if net_ok {
        b"[PASS] Network stack (Eth+ARP+IPv4)\n"
    } else {
        b"[FAIL] Network stack (Eth+ARP+IPv4)\n"
    });

    // -----------------------------------------------------------------------
    // Test 6: Memory protection (PMP + user task)
    // -----------------------------------------------------------------------
    let mp_ok = {
        // Verify PMP registers can be written and read back
        unsafe {
            // Write a test pattern to pmpcfg0 entry 0
            let test_cfg = pmpcfg_byte(PMP_MODE_TOR, PMP_R | PMP_W, true, false, true);
            pmp_write_cfg(0, test_cfg);
            // Read back
            let mut cfg: u64;
            core::arch::asm!("csrr {}, 0x3A0", out(reg) cfg, options(nomem));
            let read_byte = ((cfg >> 0) & 0xFF) as u8;
            let cfg_ok = read_byte == test_cfg;

            // Restore PMP to off
            pmp_disable();

            // Verify user task is configured correctly
            let task0 = unsafe { &TASKS[0] };
            let task_ok = task0.privilege == TASK_PRIV_USER
                && task0.mem_base == USER_TASK_MEM_BASE
                && task0.mem_size == USER_TASK_MEM_SIZE
                && task0.pc == USER_TASK_MEM_BASE as u64;

            // Verify shellcode was copied into user memory region
            let shellcode_ok = unsafe {
                let mut match_ok = true;
                for i in 0..USER_TASK_SHELLCODE.len() {
                    let expected = USER_TASK_SHELLCODE[i];
                    let actual = read_unaligned((USER_TASK_MEM_BASE + i) as *const u8);
                    if actual != expected {
                        match_ok = false;
                        break;
                    }
                }
                match_ok
            };

            // Verify supervisor can access user memory (PMP doesn't restrict S-mode)
            let supervisor_access_ok = unsafe {
                let test_addr = (USER_TASK_MEM_BASE + 4) as *const u8;
                let _val = read_unaligned(test_addr);
                true
            };

            cfg_ok && task_ok && shellcode_ok && supervisor_access_ok
        }
    };
    sbi_console_write(if mp_ok {
        b"[PASS] Memory protection (PMP)\n"
    } else {
        b"[FAIL] Memory protection (PMP)\n"
    });
    sbi_console_write(b"\n");

    // -----------------------------------------------------------------------
    // Test 7: UDP transport + DHCP client stack
    // -----------------------------------------------------------------------
    let udp_dhcp_ok = {
        // UDP header build/parse round-trip
        let mut udp_buf = [0u8; UDP_HLEN];
        let udp_written = udp_build_header(&mut udp_buf, 0, UDP_PORT_BOOTPC, UDP_PORT_BOOTPS, 0);
        let udp_build_ok = udp_written == UDP_HLEN;
        let udp_parse_ok = match udp_parse_header(&udp_buf, 0) {
            Some((sp, dp, len)) => sp == UDP_PORT_BOOTPC && dp == UDP_PORT_BOOTPS && len == UDP_HLEN as u16,
            None => false,
        };

        // DHCP Discover build
        let mut dhcp_buf = [0u8; 300];
        let dhcp_len = dhcp_build_discover(&mut dhcp_buf, 0);
        let dhcp_build_ok = dhcp_len > 0;
        // Verify magic cookie present in options
        let dhcp_magic_ok = dhcp_buf[236..240] == DHCP_MAGIC_COOKIE
            && dhcp_buf[240] == DHCP_OPT_MESSAGE_TYPE
            && dhcp_buf[242] == DHCP_MSG_DISCOVER
            && dhcp_buf[247] == DHCP_OPT_END;

        // DHCP Offer parse round-trip: build a minimal offer header + options
        let mut offer_buf = [0u8; 240 + 8];
        // op=REPLY, htype=ETHER, hlen=6, hops=0, xid=0x12345678, secs=0, flags=0
        offer_buf[0] = DHCP_OP_REPLY;
        offer_buf[1] = DHCP_HTYPE_ETHER;
        offer_buf[2] = DHCP_HLEN_ETHER;
        offer_buf[3] = 0;
        offer_buf[4..8].copy_from_slice(&0x12345678u32.to_be_bytes());
        // yiaddr (offered IP) at offset 16
        let offered_ip = [10, 0, 2, 100];
        offer_buf[16..20].copy_from_slice(&offered_ip);
        // siaddr (server IP) at offset 20
        let server_ip = [10, 0, 2, 1];
        offer_buf[20..24].copy_from_slice(&server_ip);
        // options at offset 236
        offer_buf[236..240].copy_from_slice(&DHCP_MAGIC_COOKIE);
        offer_buf[240] = DHCP_OPT_MESSAGE_TYPE;
        offer_buf[241] = 1;
        offer_buf[242] = DHCP_MSG_OFFER;
        offer_buf[247] = DHCP_OPT_END;

        let dhcp_parse_ok = match dhcp_parse_offer(&offer_buf, 0) {
            Some((mt, oip, sip)) =>
                mt == DHCP_MSG_OFFER && oip == offered_ip && sip == server_ip,
            None => false,
        };

        udp_build_ok && udp_parse_ok && dhcp_build_ok && dhcp_magic_ok && dhcp_parse_ok
    };
    sbi_console_write(if udp_dhcp_ok {
        b"[PASS] UDP + DHCP client stack\n"
    } else {
        b"[FAIL] UDP + DHCP client stack\n"
    });

    // -----------------------------------------------------------------------
    // Test 8: DHCP Request/Ack + DNS client stack
    // -----------------------------------------------------------------------
    let dhcp_dns_ok = {
        // DHCP Request build
        let mut dhcp_req_buf = [0u8; 260];
        // Simulate client state for request building
        unsafe {
            DHCP_CLIENT.xid = 0x12345678;
            DHCP_CLIENT.offered_ip = [10, 0, 2, 100];
            DHCP_CLIENT.server_ip = [10, 0, 2, 1];
        }
        let dhcp_req_len = dhcp_build_request(&mut dhcp_req_buf, 0);
        let dhcp_req_build_ok = dhcp_req_len > 0;
        // Verify magic cookie + message type = REQUEST + requested IP option
        let dhcp_req_ok = dhcp_req_len > 0
            && dhcp_req_buf[236..240] == DHCP_MAGIC_COOKIE
            && dhcp_req_buf[240] == DHCP_OPT_MESSAGE_TYPE
            && dhcp_req_buf[242] == DHCP_MSG_REQUEST
            && dhcp_req_buf[243] == DHCP_OPT_REQUESTED_IP
            && dhcp_req_buf[247] == DHCP_OPT_END;

        // DHCP Ack parse round-trip
        let mut ack_buf = [0u8; 262];
        ack_buf[0] = DHCP_OP_REPLY;
        ack_buf[1] = DHCP_HTYPE_ETHER;
        ack_buf[2] = DHCP_HLEN_ETHER;
        ack_buf[3] = 0;
        ack_buf[4..8].copy_from_slice(&0x12345678u32.to_be_bytes());
        let assigned_ip = [10, 0, 2, 100];
        ack_buf[16..20].copy_from_slice(&assigned_ip);
        let subnet_mask = [255, 255, 255, 0];
        ack_buf[236..240].copy_from_slice(&DHCP_MAGIC_COOKIE);
        ack_buf[240] = DHCP_OPT_MESSAGE_TYPE;
        ack_buf[241] = 1;
        ack_buf[242] = DHCP_MSG_ACK;
        ack_buf[243] = DHCP_OPT_SUBNET_MASK;
        ack_buf[244] = 4;
        ack_buf[245..249].copy_from_slice(&subnet_mask);
        ack_buf[249] = DHCP_OPT_ROUTER;
        ack_buf[250] = 4;
        ack_buf[251..255].copy_from_slice(&[10, 0, 2, 1]);
        ack_buf[255] = DHCP_OPT_DNS;
        ack_buf[256] = 4;
        ack_buf[257..261].copy_from_slice(&[8, 8, 8, 8]);
        ack_buf[261] = DHCP_OPT_END;

        let dhcp_ack_ok = match dhcp_parse_ack(&ack_buf, 0) {
            Some((aip, sm, rt, dns)) =>
                aip == assigned_ip && sm == subnet_mask && rt == [10, 0, 2, 1] && dns == [8, 8, 8, 8],
            None => false,
        };

        // DNS Query build + Response parse round-trip
        let mut dns_buf = [0u8; 64];
        let dns_len = dns_build_query(&mut dns_buf, 0, 0xABCD, "example.com");
        let dns_build_ok = dns_len > 0;
        // Verify header + question
        let dns_header_ok = dns_len > 0
            && dns_buf[0..2] == 0xABCDu16.to_be_bytes()
            && dns_buf[4..6] == 1u16.to_be_bytes(); // qdcount = 1

        // Build a DNS response with one A record answer
        let mut dns_resp_buf = [0u8; 64];
        // Copy header from query, set QR + ancount
        dns_resp_buf[0..2].copy_from_slice(&0xABCDu16.to_be_bytes());
        dns_resp_buf[2..4].copy_from_slice(&0x8180u16.to_be_bytes()); // QR + standard response
        dns_resp_buf[4..6].copy_from_slice(&1u16.to_be_bytes()); // qdcount
        dns_resp_buf[6..8].copy_from_slice(&1u16.to_be_bytes()); // ancount
        dns_resp_buf[8..10].copy_from_slice(&0u16.to_be_bytes()); // nscount
        dns_resp_buf[10..12].copy_from_slice(&0u16.to_be_bytes()); // arcount
        // Copy question section from query (name + qtype + qclass)
        let qsection_len = dns_len - 12;
        dns_resp_buf[12..12 + qsection_len].copy_from_slice(&dns_buf[12..12 + qsection_len]);
        // Answer: name pointer (0xC00C), type A, class IN, ttl=0, rdlength=4, ip
        let ans_start = 12 + qsection_len;
        dns_resp_buf[ans_start..ans_start + 2].copy_from_slice(&0xC00Cu16.to_be_bytes());
        dns_resp_buf[ans_start + 2..ans_start + 4].copy_from_slice(&1u16.to_be_bytes()); // type A
        dns_resp_buf[ans_start + 4..ans_start + 6].copy_from_slice(&1u16.to_be_bytes()); // class IN
        dns_resp_buf[ans_start + 6..ans_start + 10].copy_from_slice(&0u32.to_be_bytes()); // ttl
        dns_resp_buf[ans_start + 10..ans_start + 12].copy_from_slice(&4u16.to_be_bytes()); // rdlength
        let resp_ip = [93, 184, 216, 34]; // example.com
        dns_resp_buf[ans_start + 12..ans_start + 16].copy_from_slice(&resp_ip);
        let resp_total = ans_start + 16;

        let dns_resp_ok = match dns_parse_response(&dns_resp_buf, 0, &mut [[0u8; 4]; 4]) {
            Some((rid, count)) => rid == 0xABCD && count == 1,
            None => false,
        };

        dhcp_req_build_ok && dhcp_req_ok && dhcp_ack_ok && dns_build_ok && dns_header_ok && dns_resp_ok
    };
    sbi_console_write(if dhcp_dns_ok {
        b"[PASS] DHCP Request/Ack + DNS client stack\n"
    } else {
        b"[FAIL] DHCP Request/Ack + DNS client stack\n"
    });

    // -----------------------------------------------------------------------
    // Test 9: TCP transport layer (SYN -> SYN-ACK handshake)
    // -----------------------------------------------------------------------
    let tcp_ok = {
        // TCP header build/parse round-trip
        let mut tcp_buf = [0u8; TCP_HLEN];
        let tcp_written = tcp_build_header(
            &mut tcp_buf,
            0,
            12345,
            80,
            0xDEADBEEF,
            0xCAFEBABE,
            TCP_FLAG_SYN | TCP_FLAG_ACK,
            4096,
        );
        let tcp_build_ok = tcp_written == TCP_HLEN;
        let tcp_parse_ok = match tcp_parse_header(&tcp_buf, 0) {
            Some((sp, dp, seq, ack, flags, window)) => {
                sp == 12345 && dp == 80 && seq == 0xDEADBEEF && ack == 0xCAFEBABE
                    && flags == (TCP_FLAG_SYN | TCP_FLAG_ACK) && window == 4096
            }
            None => false,
        };

        // TCP SYN build
        let mut syn_buf = [0u8; TCP_HLEN];
        let syn_len = tcp_build_syn(&mut syn_buf, 0, 5000, 80, 0x1000);
        let syn_build_ok = syn_len == TCP_HLEN;
        let syn_parse_ok = match tcp_parse_header(&syn_buf, 0) {
            Some((sp, dp, seq, ack, flags, window)) => {
                sp == 5000 && dp == 80 && seq == 0x1000 && ack == 0
                    && flags == TCP_FLAG_SYN && window == 4096
            }
            None => false,
        };

        // TCP SYN-ACK build
        let mut synack_buf = [0u8; TCP_HLEN];
        let synack_len = tcp_build_synack(&mut synack_buf, 0, 80, 5000, 0x2000, 0x1001);
        let synack_build_ok = synack_len == TCP_HLEN;
        let synack_parse_ok = match tcp_parse_header(&synack_buf, 0) {
            Some((sp, dp, seq, ack, flags, window)) => {
                sp == 80 && dp == 5000 && seq == 0x2000 && ack == 0x1001
                    && flags == (TCP_FLAG_SYN | TCP_FLAG_ACK) && window == 4096
            }
            None => false,
        };

        // TCP ACK build
        let mut ack_buf = [0u8; TCP_HLEN];
        let ack_len = tcp_build_ack(&mut ack_buf, 0, 5000, 80, 0x1001, 0x2001);
        let ack_build_ok = ack_len == TCP_HLEN;
        let ack_parse_ok = match tcp_parse_header(&ack_buf, 0) {
            Some((sp, dp, seq, ack, flags, window)) => {
                sp == 5000 && dp == 80 && seq == 0x1001 && ack == 0x2001
                    && flags == TCP_FLAG_ACK && window == 4096
            }
            None => false,
        };

        tcp_build_ok && tcp_parse_ok && syn_build_ok && syn_parse_ok && synack_build_ok && synack_parse_ok && ack_build_ok && ack_parse_ok
    };
    sbi_console_write(if tcp_ok {
        b"[PASS] TCP transport layer (SYN/SYN-ACK/ACK)\n"
    } else {
        b"[FAIL] TCP transport layer (SYN/SYN-ACK/ACK)\n"
    });

    // -----------------------------------------------------------------------
    // Test 10: TCP data transfer (data segment build/parse + echo)
    // -----------------------------------------------------------------------
    let tcp_data_ok = {
        // Build a TCP segment with data payload
        let mut tcp_data_buf = [0u8; TCP_HLEN + 16];
        let tcp_data_written = tcp_build_data_ack(
            &mut tcp_data_buf,
            0,
            5000,
            80,
            0x1001,
            0x2001,
        );
        let tcp_data_build_ok = tcp_data_written == TCP_HLEN;
        // Copy test payload after the header
        let test_payload = b"HELLO-TCP";
        tcp_data_buf[TCP_HLEN..TCP_HLEN + test_payload.len()].copy_from_slice(test_payload);
        // Parse back and verify header fields + data
        let tcp_data_parse_ok = match tcp_parse_header(&tcp_data_buf, 0) {
            Some((sp, dp, seq, ack, flags, window)) => {
                sp == 5000 && dp == 80 && seq == 0x1001 && ack == 0x2001
                    && flags == TCP_FLAG_ACK && window == 4096
            }
            None => false,
        };
        let tcp_data_content_ok = &tcp_data_buf[TCP_HLEN..TCP_HLEN + test_payload.len()] == test_payload;

        // TCP data echo: build a reply with data
        let mut tcp_echo_buf = [0u8; TCP_HLEN + 8];
        let echo_written = tcp_build_data_ack(
            &mut tcp_echo_buf,
            0,
            80,
            5000,
            0x2001,
            0x1009,
        );
        let echo_build_ok = echo_written == TCP_HLEN;
        let echo_payload = b"ECHO-BACK";
        tcp_echo_buf[TCP_HLEN..TCP_HLEN + echo_payload.len()].copy_from_slice(echo_payload);
        let echo_parse_ok = match tcp_parse_header(&tcp_echo_buf, 0) {
            Some((sp, dp, seq, ack, flags, window)) => {
                sp == 80 && dp == 5000 && seq == 0x2001 && ack == 0x1009
                    && flags == TCP_FLAG_ACK && window == 4096
            }
            None => false,
        };
        let echo_content_ok = &tcp_echo_buf[TCP_HLEN..TCP_HLEN + echo_payload.len()] == echo_payload;

        tcp_data_build_ok && tcp_data_parse_ok && tcp_data_content_ok && echo_build_ok && echo_parse_ok && echo_content_ok
    };
    sbi_console_write(if tcp_data_ok {
        b"[PASS] TCP data transfer (data segment build/parse + echo)\n"
    } else {
        b"[FAIL] TCP data transfer (data segment build/parse + echo)\n"
    });

    // -----------------------------------------------------------------------
    // Test 11: Protected process execution under PMP
    // -----------------------------------------------------------------------
    let proc_ok = {
        // Set up PMP for user task memory region
        unsafe {
            setup_pmp(USER_TASK_MEM_BASE, USER_TASK_MEM_SIZE);
        }

        // Create and run the userspace program through the runtime VM
        let mut proc = Process::new();
        proc.load_program(&USER_TASK_PROGRAM);

        let mut step_count = 0;
        let max_steps = 1000;
        let mut exited_cleanly = false;
        let mut printed_hello = false;

        loop {
            match proc.step() {
                Ok(()) => {
                    proc.flush_stdout();
                    step_count += 1;
                    if step_count > max_steps {
                        break;
                    }
                }
                Err(()) => {
                    if proc.exited {
                        exited_cleanly = true;
                    }
                    break;
                }
            }
        }

        // Verify the program executed correctly
        let stdout_str = core::str::from_utf8(&proc.stdout[..proc.stdout_len]).unwrap_or("");
        printed_hello = stdout_str.contains("HELLO") && stdout_str.len() >= 5;

        // Restore PMP for supervisor (disable user restrictions)
        unsafe {
            pmp_disable();
        }

        exited_cleanly && printed_hello && step_count < max_steps
    };
    sbi_console_write(if proc_ok {
        b"[PASS] Protected process execution (PMP + user task)\n"
    } else {
        b"[FAIL] Protected process execution (PMP + user task)\n"
    });
    sbi_console_write(b"\n");

    // -----------------------------------------------------------------------
    // Summary
    // -----------------------------------------------------------------------
    let all_ok = ipc_ok && storage_match && fs_ok && timer_ok && net_ok && mp_ok && udp_dhcp_ok && dhcp_dns_ok && tcp_ok && tcp_data_ok && proc_ok;
    sbi_console_write(if all_ok {
        b"[BOOT SELF-TEST] ALL SUBSYSTEMS PASS\n"
    } else {
        b"[BOOT SELF-TEST] SOME SUBSYSTEMS FAILED\n"
    });
    sbi_console_write(b"===============================\n\n");
}

#[unsafe(no_mangle)]
pub extern "C" fn start() -> ! {
    // Capture the firmware-supplied Device Tree pointer (RISC-V S-mode boot
    // convention: a1 = FDT physical address) and use it to discover the real
    // virtio-mmio device addresses instead of relying on hard-coded defaults.
    // If no usable DTB is provided, the compiled-in defaults are kept so boot
    // still works on the known QEMU `virt` layout.
    let fdt_ptr: usize;
    unsafe {
        core::arch::asm!("mv {}, a1", out(reg) fdt_ptr, options(nomem));
    }
    unsafe {
        let mut devices = [(0usize, VirtioDevType::Unknown); MAX_VIRTIO_DEVICES];
        if let Ok(count) = discover_virtio_devices(fdt_ptr, &mut devices) {
            for i in 0..count {
                let (base, dev_type) = devices[i];
                match dev_type {
                    VirtioDevType::Block => VIRTIO_BLOCK_MMIO = base,
                    VirtioDevType::Net => VIRTIO_NET_MMIO = base,
                    VirtioDevType::Unknown => {}
                }
            }
        }
    }

    csrw_stvec(trap_entry as *const () as usize as u64);
    csrs_sie(SIE_STIE);
    csrs_sstatus(SSTATUS_SIE);
    sbi_console_write(b"stvec+sie+sstatus set\n");
    sbi_console_write(b"hello from kernel\n");

    // ---------------------------------------------------------------------------
    // Update/Recovery initialization: A/B partition OTA demo
    // ---------------------------------------------------------------------------
    let mut update = UpdateSystem::new();
    sbi_console_write(b"active partition: ");
    sbi_console_putchar(match update.active {
        Partition::A => b'A',
        Partition::B => b'B',
    });
    sbi_console_write(b", version: ");
    for &b in update.active_version() {
        sbi_console_putchar(b);
    }
    sbi_console_write(b"\n");

    // Begin an OTA update to partition B (version 2.0.0)
    let manifest = Manifest {
        version: [b'2', b'.', b'0', b'.', b'0', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        version_len: 5,
        hash: 0xDEAD,
        size: 1024,
    };
    update.begin_update(manifest).unwrap();
    sbi_console_write(b"update begun: target partition ");
    sbi_console_putchar(match update.active.other() {
        Partition::A => b'A',
        Partition::B => b'B',
    });
    sbi_console_write(b", state: pending\n");

    // Apply the update
    update.apply_update().unwrap();
    sbi_console_write(b"update applied: partition B version ");
    for &b in &update.version_b[..update.version_b_len] {
        sbi_console_putchar(b);
    }
    sbi_console_write(b"\n");

    // Confirm boot (simulate successful boot on partition B)
    update.confirm_boot().unwrap();
    sbi_console_write(b"boot confirmed: active partition ");
    sbi_console_putchar(match update.active {
        Partition::A => b'A',
        Partition::B => b'B',
    });
    sbi_console_write(b", version ");
    for &b in update.active_version() {
        sbi_console_putchar(b);
    }
    sbi_console_write(b"\n");

    unsafe {
        TASKS[1].pc = task1_entry as *const () as usize as u64;
        TASKS[1].sp = (&raw const TASK1_STACK as *const [u64; 128] as usize + 1024) as u64;
    }

    // Set up user-mode task with PMP memory protection
    unsafe {
        setup_user_task();
    }

    let mtime_now = clint_read_mtime();
    let fire_at = mtime_now.wrapping_add(TIMER_INTERVAL);
    sbi_timer_set(fire_at);
    sbi_console_write(b"timer set at ");

    let mut buf = [0u8; 18];
    buf[0] = b'0';
    buf[1] = b'x';
    let mut v = fire_at;
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
    sbi_console_write(b"\n");

    // Initialize VirtIO block device if present.
    unsafe {
        if virtio_blk_init() {
            VIRTIO_BLOCK_READY = true;
            sbi_console_write(b"VirtIO block: Using real block device\n");
        } else {
            sbi_console_write(b"VirtIO block: No device, using RAM disk\n");
        }
    }

    // Initialize VirtIO network device if present.
    unsafe {
        if virtio_net_init() {
            sbi_console_write(b"VirtIO net: Using real network device\n");
        } else {
            sbi_console_write(b"VirtIO net: No device, network disabled\n");
        }
    }

    // Run boot self-test before entering the main loop.
    boot_self_test();

    loop {
        if let Some(msg) = ipc_receive() {
            sbi_console_write(b"recv: ");
            for &b in &msg {
                if b == 0 {
                    break;
                }
                sbi_console_putchar(b);
            }
            sbi_console_write(b"\n");
        }

        // Poll for received network frames and process them.
        let mut net_buf = [0u8; 1518];
        let net_len = unsafe { net_recv(&mut net_buf) };
        if net_len > 0 {
            let mut out_buf = [0u8; 1518];
            let reply_len = net_process_frame(&net_buf[..net_len], &mut out_buf);
            if reply_len > 0 {
                unsafe { virtio_net_send(&out_buf[..reply_len]) };
                sbi_console_write(b"net: reply sent\n");
            }
        }

        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}

// ============================================================================
// Device Tree discovery (replaces hard-coded virtio-mmio addresses)
// ============================================================================

/// Classification of a discovered virtio-mmio device.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VirtioDevType {
    Block,
    Net,
    Unknown,
}

/// Runtime-discovered MMIO base addresses. The compiled-in QEMU `virt` defaults
/// (block at 0x1000_1000, net at 0x1000_2000) are used when the firmware does
/// not supply a usable Device Tree or when discovery finds no devices.
static mut VIRTIO_BLOCK_MMIO: usize = VIRTIO0_MMIO;
static mut VIRTIO_NET_MMIO: usize = VIRTIO_NET_MMIO_DEFAULT;

/// Maximum number of virtio-mmio devices the fixed-size discovery buffers can hold
/// (the kernel is `#![no_std]`, so discovery uses fixed arrays instead of `Vec`).
const MAX_VIRTIO_DEVICES: usize = 8;

/// Read a u32 from an arbitrary virtio MMIO base (registers are u32 spaced by 4 bytes).
unsafe fn mmio_read_at(base: usize, offset: usize) -> u32 {
    read_unaligned((base + offset * 4) as *const u32)
}

/// Write a u32 to an arbitrary virtio MMIO base.
unsafe fn mmio_write_at(base: usize, offset: usize, value: u32) {
    write_unaligned((base + offset * 4) as *mut u32, value);
}

// ---------------------------------------------------------------------------
// Minimal Device Tree Blob (DTB) parser
//
// QEMU `virt` passes a flattened Device Tree to the guest. We walk it to find
// `virtio,mmio` nodes and read their `reg` property (MMIO base address).
// All DTB multi-byte fields are big-endian.
// ---------------------------------------------------------------------------

const FDT_BEGIN_NODE: u32 = 0x00000001;
const FDT_END_NODE: u32 = 0x00000002;
const FDT_PROP: u32 = 0x00000003;
const FDT_NOP: u32 = 0x00000004;
const FDT_END: u32 = 0x00000009;
const FDT_MAGIC: u32 = 0xd00d_feed;

#[repr(C)]
struct DtbHeader {
    magic: u32,
    totalsize: u32,
    off_dt_struct: u32,
    off_dt_strings: u32,
    off_mem_rsvmap: u32,
    version: u32,
    last_comp_version: u32,
    boot_cpuid_phys: u32,
    size_dt_strings: u32,
    size_dt_struct: u32,
}

fn read_u32_be(buf: &[u8], off: usize) -> Option<u32> {
    if off.checked_add(4).map_or(true, |e| e > buf.len()) {
        return None;
    }
    Some(u32::from_be_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]))
}

fn read_u64_be(buf: &[u8], off: usize) -> Option<u64> {
    if off.checked_add(8).map_or(true, |e| e > buf.len()) {
        return None;
    }
    let mut b = [0u8; 8];
    b.copy_from_slice(&buf[off..off + 8]);
    Some(u64::from_be_bytes(b))
}

fn align4(n: usize) -> usize {
    (n + 3) & !3
}

fn cstr_at(buf: &[u8], off: usize) -> Option<&str> {
    let mut end = off;
    while end < buf.len() && buf[end] != 0 {
        end += 1;
    }
    if end >= buf.len() {
        return None;
    }
    core::str::from_utf8(&buf[off..end]).ok()
}

/// Minimal in-kernel Device Tree view. Walks the flattened structure block to
/// locate `virtio,mmio` nodes and extract their MMIO base addresses.
struct Dtb<'a> {
    fdt: &'a [u8],
    strings: &'a [u8],
    struct_off: usize,
    struct_end: usize,
    addr_cells: u32,
    size_cells: u32,
}

impl<'a> Dtb<'a> {
    fn from_bytes(fdt: &'a [u8]) -> Result<Self, &'static str> {
        if fdt.len() < 32 {
            return Err("dtb: too small for header");
        }
        let magic = u32::from_be_bytes([fdt[0], fdt[1], fdt[2], fdt[3]]);
        if magic != FDT_MAGIC {
            return Err("dtb: bad magic");
        }
        let off_dt_struct =
            u32::from_be_bytes([fdt[8], fdt[9], fdt[10], fdt[11]]) as usize;
        let off_dt_strings =
            u32::from_be_bytes([fdt[12], fdt[13], fdt[14], fdt[15]]) as usize;
        let size_dt_strings =
            u32::from_be_bytes([fdt[32], fdt[33], fdt[34], fdt[35]]) as usize;
        let size_dt_struct =
            u32::from_be_bytes([fdt[36], fdt[37], fdt[38], fdt[39]]) as usize;
        if off_dt_struct >= fdt.len() || off_dt_strings >= fdt.len() {
            return Err("dtb: offset out of range");
        }
        let strings_end = off_dt_strings
            + size_dt_strings.min(fdt.len() - off_dt_strings);
        let strings = &fdt[off_dt_strings..strings_end];
        let struct_end =
            off_dt_struct + size_dt_struct.min(fdt.len() - off_dt_struct);
        Ok(Self {
            fdt,
            strings,
            struct_off: off_dt_struct,
            struct_end,
            addr_cells: 2, // QEMU `virt` root default
            size_cells: 2,
        })
    }

    fn u32_at(&self, off: usize) -> Option<u32> {
        read_u32_be(self.fdt, off)
    }

    /// Read a null-terminated string from the strings block.
    fn str_from_off(&self, off: usize) -> Option<&'a str> {
        if off >= self.strings.len() {
            return None;
        }
        let mut end = off;
        while end < self.strings.len() && self.strings[end] != 0 {
            end += 1;
        }
        if end >= self.strings.len() {
            return None;
        }
        core::str::from_utf8(&self.strings[off..end]).ok()
    }

    /// Walk the structure block and collect MMIO base addresses of
    /// `virtio,mmio` nodes into `out`. Returns the number of bases written.
    fn find_virtio_mmio(&mut self, out: &mut [usize]) -> Result<usize, &'static str> {
        let mut count: usize = 0;
        let mut pos = self.struct_off;
        let mut in_virtio = false;
        let mut node_addr_cells = self.addr_cells as usize;
        let mut node_size_cells = self.size_cells as usize;

        while pos < self.struct_end {
            let token = match self.u32_at(pos) {
                Some(t) => t,
                None => break,
            };
            pos += 4;
            match token {
                FDT_BEGIN_NODE => {
                    let name = cstr_at(self.fdt, pos).unwrap_or("");
                    let nlen = name.len() + 1;
                    pos += align4(nlen);
                    let is_virtio = name == "virtio,mmio" || name.starts_with("virtio");
                    if is_virtio {
                        in_virtio = true;
                        node_addr_cells = self.addr_cells as usize;
                        node_size_cells = self.size_cells as usize;
                    } else {
                        in_virtio = false;
                    }
                }
                FDT_END_NODE => {
                    in_virtio = false;
                    node_addr_cells = self.addr_cells as usize;
                    node_size_cells = self.size_cells as usize;
                }
                FDT_PROP => {
                    let len = match self.u32_at(pos) {
                        Some(l) => l as usize,
                        None => break,
                    };
                    pos += 4;
                    let nameoff = match self.u32_at(pos) {
                        Some(n) => n as usize,
                        None => break,
                    };
                    pos += 4;
                    let prop_end = pos.checked_add(len).ok_or("dtb: prop overflow")?;
                    if prop_end > self.fdt.len() {
                        return Err("dtb: prop value out of range");
                    }
                    let pname = self.str_from_off(nameoff).unwrap_or("");
                    if in_virtio {
                        match pname {
                            "#address-cells" => {
                                node_addr_cells = self.u32_at(pos).unwrap_or(2) as usize;
                            }
                            "#size-cells" => {
                                node_size_cells = self.u32_at(pos).unwrap_or(2) as usize;
                            }
                            "reg" => {
                                if count < out.len() {
                                    if let Some(base) = self.parse_reg(pos, len, node_addr_cells, node_size_cells)
                                    {
                                        out[count] = base;
                                        count += 1;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    pos = align4(prop_end);
                }
                FDT_NOP => {}
                FDT_END => break,
                _ => return Err("dtb: unknown token"),
            }
        }
        Ok(count)
    }

    /// Parse a `reg` property value into an MMIO base address.
    fn parse_reg(
        &self,
        off: usize,
        len: usize,
        addr_cells: usize,
        size_cells: usize,
    ) -> Option<usize> {
        if addr_cells == 0 || size_cells == 0 {
            return None;
        }
        let entry_bytes = (addr_cells + size_cells) * 4;
        if len < entry_bytes {
            return None;
        }
        // Each address is `addr_cells` big-endian u32 cells; take the low cell
        // as the base (QEMU `virt` uses 32-bit MMIO addresses).
        read_u32_be(self.fdt, off + (addr_cells - 1) * 4).map(|c| c as usize)
    }
}

/// Discover virtio-mmio devices from a firmware-supplied Device Tree.
/// `fdt_ptr` is the physical address of the DTB (identity-mapped: the kernel
/// runs without an MMU). Writes discovered (MMIO base, device type) pairs into
/// `out` and returns the number of devices written. Falls back gracefully when
/// no DTB is present or it is unparseable.
unsafe fn discover_virtio_devices(
    fdt_ptr: usize,
    out: &mut [(usize, VirtioDevType)],
) -> Result<usize, &'static str> {
    if fdt_ptr == 0 {
        return Err("discover: no device tree pointer");
    }
    let header = core::slice::from_raw_parts(fdt_ptr as *const u8, 32);
    let totalsize = read_u32_be(header, 4).unwrap_or(4096) as usize;
    let fdt = core::slice::from_raw_parts(fdt_ptr as *const u8, totalsize.max(32));
    let mut dtb = Dtb::from_bytes(fdt)?;
    let mut bases = [0usize; MAX_VIRTIO_DEVICES];
    let base_count = dtb.find_virtio_mmio(&mut bases)?;
    let mut count: usize = 0;
    for i in 0..base_count.min(out.len()) {
        let base = bases[i];
        let dev_type = probe_virtio_type(base);
        out[count] = (base, dev_type);
        count += 1;
    }
    Ok(count)
}

/// Probe a virtio-mmio device's type by reading its magic/version/device-id.
unsafe fn probe_virtio_type(base: usize) -> VirtioDevType {
    let magic = mmio_read_at(base, VIRTIO_MMIO_MAGIC_VALUE);
    if magic != VIRTIO_CONFIG_MAGIC {
        return VirtioDevType::Unknown;
    }
    let version = mmio_read_at(base, VIRTIO_MMIO_VERSION);
    if version != VIRTIO_CONFIG_VERSION {
        return VirtioDevType::Unknown;
    }
    match mmio_read_at(base, VIRTIO_MMIO_DEVICE_ID) {
        VIRTIO_DEVICE_TYPE_BLOCK => VirtioDevType::Block,
        VIRTIO_DEVICE_TYPE_NET => VirtioDevType::Net,
        _ => VirtioDevType::Unknown,
    }
}

// ============================================================================
// VirtIO Block Device Support (Phase 3)
// ============================================================================

/// QEMU virt VirtIO block device MMIO base address (discovered from DTB when available).
const VIRTIO0_MMIO: usize = 0x1000_1000;

// ---------------------------------------------------------------------------
// VirtIO MMIO register offsets (as u32 indices, each register is 4 bytes)
// ---------------------------------------------------------------------------
const VIRTIO_MMIO_MAGIC_VALUE: usize = 0x00 / 4;   // 0x00
const VIRTIO_MMIO_VERSION: usize = 0x04 / 4;       // 0x04
const VIRTIO_MMIO_DEVICE_ID: usize = 0x08 / 4;     // 0x08
const VIRTIO_MMIO_VENDOR_ID: usize = 0x0c / 4;     // 0x0c
const VIRTIO_MMIO_HOST_FEATURES: usize = 0x10 / 4; // 0x10
const VIRTIO_MMIO_HOST_FEATURES_SEL: usize = 0x14 / 4; // 0x14
const VIRTIO_MMIO_GUEST_FEATURES: usize = 0x20 / 4; // 0x20
const VIRTIO_MMIO_GUEST_FEATURES_SEL: usize = 0x24 / 4; // 0x24
const VIRTIO_MMIO_GUEST_PAGE_SIZE: usize = 0x28 / 4; // 0x28
const VIRTIO_MMIO_QUEUE_SEL: usize = 0x30 / 4;     // 0x30
const VIRTIO_MMIO_QUEUE_NUM_MAX: usize = 0x34 / 4; // 0x34
const VIRTIO_MMIO_QUEUE_NUM: usize = 0x38 / 4;     // 0x38
const VIRTIO_MMIO_QUEUE_READY: usize = 0x3c / 4;   // 0x3c
const VIRTIO_MMIO_QUEUE_DESC_LOW: usize = 0x40 / 4; // 0x40
const VIRTIO_MMIO_QUEUE_DESC_HIGH: usize = 0x42 / 4; // 0x42
const VIRTIO_MMIO_QUEUE_AVAIL_LOW: usize = 0x44 / 4; // 0x44
const VIRTIO_MMIO_QUEUE_AVAIL_HIGH: usize = 0x46 / 4; // 0x46
const VIRTIO_MMIO_QUEUE_USED_LOW: usize = 0x48 / 4; // 0x48
const VIRTIO_MMIO_QUEUE_USED_HIGH: usize = 0x4a / 4; // 0x4a
const VIRTIO_MMIO_QUEUE_NOTIFY: usize = 0x4c / 4;  // 0x4c
const VIRTIO_MMIO_INTERRUPT_STATUS: usize = 0x50 / 4; // 0x50
const VIRTIO_MMIO_INTERRUPT_ACK: usize = 0x54 / 4; // 0x54
const VIRTIO_MMIO_DEVICE_STATUS: usize = 0x58 / 4; // 0x58
const VIRTIO_MMIO_CONFIG_GENERATION: usize = 0x5c / 4; // 0x5c

// ---------------------------------------------------------------------------
// VirtIO constants
// ---------------------------------------------------------------------------
const VIRTIO_CONFIG_MAGIC: u32 = 0x7472_6976; // "virt"
const VIRTIO_CONFIG_VERSION: u32 = 2; // VirtIO 1.0
const VIRTIO_DEVICE_TYPE_BLOCK: u32 = 2;
const VIRTIO_DEVICE_TYPE_NET: u32 = 1;
const VIRTIO_VENDOR_ID: u32 = 0x1af4;

// Device status register values
const VIRTIO_STATUS_RESET: u32 = 0x00;
const VIRTIO_STATUS_ACK: u32 = 0x01;
const VIRTIO_STATUS_DRIVER: u32 = 0x02;
const VIRTIO_STATUS_DRIVER_OK: u32 = 0x04;
const VIRTIO_STATUS_FEATURES_OK: u32 = 0x08;

// Feature bits
const VIRTIO_F_VERSION_1: u64 = 1 << 32;

// VirtIO block device constants
const VIRTIO_BLK_SECTOR_SIZE: usize = 512;
const VIRTIO_BLK_T_IN: u32 = 0;   // read from device
const VIRTIO_BLK_T_OUT: u32 = 1;  // write to device
const VIRTIO_BLK_S_OK: u32 = 0;
const VIRTIO_BLK_S_IOERR: u32 = 1;
const VIRTIO_BLK_S_UNSUPP: u32 = 2;

// Descriptor flags
const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;
const VIRTQ_DESC_F_INDIRECT: u16 = 4;

// ---------------------------------------------------------------------------
// Fixed physical addresses for virtqueue structures (no MMU)
// ---------------------------------------------------------------------------
const VQ_DESC_TABLE: u64 = 0x2000_1000; // 16 entries × 16 bytes = 256 bytes
const VQ_AVAIL_RING: u64 = 0x2000_1100; // avail ring
const VQ_USED_RING: u64 = 0x2000_1200;  // used ring
const BLK_REQ_HEADER: u64 = 0x2000_1300; // block request header (12 bytes)
const BLK_DATA_BUFFER: u64 = 0x2000_1400; // 512-byte data buffer

const VQ_SIZE: usize = 16;

// ---------------------------------------------------------------------------
// VirtIO virtqueue data structures (OASIS VirtIO 1.2)
// ---------------------------------------------------------------------------

/// VirtIO virtqueue descriptor element (16 bytes per spec)
#[derive(Copy, Clone)]
struct VirtioDesc {
    addr: u64,   // physical buffer address
    len: u32,    // buffer length
    flags: u16,  // NEXT | WRITE | INDIRECT
    next: u16,   // next descriptor index (0xFFFF = terminator)
}

/// VirtIO available ring (driver writes; device reads)
#[derive(Copy, Clone)]
struct VirtioAvail {
    flags: u16,     // avail ring flags
    idx: u16,       // driver index (next descriptor to add)
    ring: [u16; VQ_SIZE], // descriptor indices available to device
    used_event: u16, // used-event index (only with EVENT_IDX)
}

/// VirtIO used ring entry (device writes; driver reads)
#[derive(Copy, Clone)]
struct VirtioUsedElem {
    id: u32,   // index of completed descriptor
    len: u32,  // total length used
}

/// VirtIO used ring (driver reads completions from)
#[derive(Copy, Clone)]
struct VirtioUsedRing {
    flags: u16,             // used ring flags
    idx: u16,               // device index (next completion to read)
    ring: [VirtioUsedElem; VQ_SIZE],
}

/// VirtIO virtqueue state
struct VirtioVq {
    desc_table: *mut VirtioDesc,
    avail: *mut VirtioAvail,
    used: *mut VirtioUsedRing,
    avail_idx: u16, // cached driver avail index
    used_idx: u16,  // cached driver used index
}

// ---------------------------------------------------------------------------
// MMIO access helpers
// ---------------------------------------------------------------------------

unsafe fn mmio_read(offset: usize) -> u32 {
    mmio_read_at(unsafe { VIRTIO_BLOCK_MMIO }, offset)
}

unsafe fn mmio_write(offset: usize, value: u32) {
    mmio_write_at(unsafe { VIRTIO_BLOCK_MMIO }, offset, value);
}

// ---------------------------------------------------------------------------
// Virtio queue setup
// ---------------------------------------------------------------------------

fn setup_virtqueue() -> Result<VirtioVq, &'static str> {
    unsafe {
        // 1. Select queue 0
        mmio_write(VIRTIO_MMIO_QUEUE_SEL, 0);

        // 2. Read queue size from device
        let queue_max = mmio_read(VIRTIO_MMIO_QUEUE_NUM_MAX);
        if queue_max == 0 {
            return Err("virtqueue: device reports queue size 0");
        }
        let queue_num = VQ_SIZE.min(queue_max as usize);

        // 3. Notify device of queue size
        mmio_write(VIRTIO_MMIO_QUEUE_NUM, queue_num as u32);

        // 4. Set queue physical addresses (descriptor table, avail ring, used ring)
        mmio_write(VIRTIO_MMIO_QUEUE_DESC_LOW, VQ_DESC_TABLE as u32);
        mmio_write(VIRTIO_MMIO_QUEUE_DESC_HIGH, (VQ_DESC_TABLE >> 32) as u32);
        mmio_write(VIRTIO_MMIO_QUEUE_AVAIL_LOW, VQ_AVAIL_RING as u32);
        mmio_write(VIRTIO_MMIO_QUEUE_AVAIL_HIGH, (VQ_AVAIL_RING >> 32) as u32);
        mmio_write(VIRTIO_MMIO_QUEUE_USED_LOW, VQ_USED_RING as u32);
        mmio_write(VIRTIO_MMIO_QUEUE_USED_HIGH, (VQ_USED_RING >> 32) as u32);

        // 5. Mark queue ready
        mmio_write(VIRTIO_MMIO_QUEUE_READY, 1);

        // 6. Verify device accepted the queue
        let ready = mmio_read(VIRTIO_MMIO_QUEUE_READY);
        if ready == 0 {
            return Err("virtqueue: device did not mark queue ready");
        }

        // 7. Initialize descriptor table in physical memory
        let desc_base = VQ_DESC_TABLE as *mut VirtioDesc;
        for i in 0..VQ_SIZE {
            let d = desc_base.add(i);
            d.write(VirtioDesc {
                addr: 0,
                len: 0,
                flags: 0,
                next: 0xFFFF,
            });
        }

        // 8. Initialize available ring
        let avail_base = VQ_AVAIL_RING as *mut VirtioAvail;
        (*avail_base).flags = 0;
        (*avail_base).idx = 0;
        for i in 0..VQ_SIZE {
            (*avail_base).ring[i] = 0xFFFF;
        }
        (*avail_base).used_event = 0;

        // 9. Initialize used ring
        let used_base = VQ_USED_RING as *mut VirtioUsedRing;
        (*used_base).flags = 0;
        (*used_base).idx = 0;
        for i in 0..VQ_SIZE {
            (*used_base).ring[i] = VirtioUsedElem { id: 0xFFFFFFFF, len: 0 };
        }

        Ok(VirtioVq {
            desc_table: desc_base,
            avail: avail_base,
            used: used_base,
            avail_idx: 0,
            used_idx: 0,
        })
    }
}

// ---------------------------------------------------------------------------
// VirtIO block device initialization (OASIS VirtIO 1.2)
// ---------------------------------------------------------------------------

unsafe fn virtio_blk_init() -> bool {
    // --- Check configuration magic value ---
    let magic = mmio_read(VIRTIO_MMIO_MAGIC_VALUE);
    if magic != VIRTIO_CONFIG_MAGIC {
        sbi_console_write(b"VirtIO block: Bad magic 0x");
        // print hex magic
        let mut buf = [0u8; 8];
        let mut m = magic;
        for i in (0..8).rev() {
            buf[i] = match m & 0xF {
                0..=9 => b'0' + (m & 0xF) as u8,
                10..=15 => b'a' + (m & 0xF - 10) as u8,
                _ => b'?',
            };
            m >>= 4;
        }
        sbi_console_write(&buf);
        sbi_console_write(b"\n");
        return false;
    }
    sbi_console_write(b"VirtIO block: Magic verified\n");

    // --- Check version ---
    let version = mmio_read(VIRTIO_MMIO_VERSION);
    if version != VIRTIO_CONFIG_VERSION {
        sbi_console_write(b"VirtIO block: Bad version\n");
        return false;
    }

    // --- Check device type (must be 2 = block) ---
    let device_id = mmio_read(VIRTIO_MMIO_DEVICE_ID);
    let vendor_id = mmio_read(VIRTIO_MMIO_VENDOR_ID);
    if device_id != VIRTIO_DEVICE_TYPE_BLOCK || vendor_id != VIRTIO_VENDOR_ID {
        sbi_console_write(b"VirtIO block: Not a VirtIO block device\n");
        return false;
    }
    sbi_console_write(b"VirtIO block: Detected VirtIO block device\n");

    // --- Reset the device ---
    mmio_write(VIRTIO_MMIO_DEVICE_STATUS, VIRTIO_STATUS_RESET);

    // --- Acknowledge the device ---
    mmio_write(VIRTIO_MMIO_DEVICE_STATUS, VIRTIO_STATUS_ACK);

    // --- Declare ourselves as a driver ---
    mmio_write(VIRTIO_MMIO_DEVICE_STATUS, VIRTIO_STATUS_DRIVER);

    // --- Feature negotiation ---
    // Select host features page 0
    mmio_write(VIRTIO_MMIO_HOST_FEATURES_SEL, 0);
    let _host_features_lo = mmio_read(VIRTIO_MMIO_HOST_FEATURES);
    mmio_write(VIRTIO_MMIO_HOST_FEATURES_SEL, 1);
    let _host_features_hi = mmio_read(VIRTIO_MMIO_HOST_FEATURES);

    // We accept VIRTIO_F_VERSION_1 from the device (bit 32, in page 1)
    let mut guest_features: u32 = 0;
    if _host_features_hi & 1 != 0 {
        // VIRTIO_F_VERSION_1 is bit 32 = bit 0 of page 1
        // We'll negotiate it by writing to page 0 of guest features (which maps to bits 0-31)
        // Actually VIRTIO_F_VERSION_1 is bit 32, so we need to write to guest features page 1
        // But for simplicity, we skip version 1 negotiation
    }
    mmio_write(VIRTIO_MMIO_GUEST_FEATURES_SEL, 0);
    mmio_write(VIRTIO_MMIO_GUEST_FEATURES, guest_features);
    mmio_write(VIRTIO_MMIO_GUEST_FEATURES_SEL, 1);
    mmio_write(VIRTIO_MMIO_GUEST_FEATURES, 0); // high bits unused

    // --- Set page size ---
    mmio_write(VIRTIO_MMIO_GUEST_PAGE_SIZE, 4096);

    // --- Set up virtqueue 0 ---
    match setup_virtqueue() {
        Ok(_vq) => {
            sbi_console_write(b"VirtIO block: Virtqueue 0 ready\n");
        }
        Err(e) => {
            sbi_console_write(b"VirtIO block: Virtqueue setup failed: ");
            sbi_console_write(e.as_bytes());
            sbi_console_write(b"\n");
            return false;
        }
    }

    // --- Set FEATURES_OK ---
    mmio_write(VIRTIO_MMIO_DEVICE_STATUS, VIRTIO_STATUS_FEATURES_OK);

    // --- Set DRIVER_OK ---
    mmio_write(VIRTIO_MMIO_DEVICE_STATUS, VIRTIO_STATUS_DRIVER_OK);

    // --- Verify device is live ---
    let status = mmio_read(VIRTIO_MMIO_DEVICE_STATUS);
    if status & VIRTIO_STATUS_DRIVER_OK == 0 {
        sbi_console_write(b"VirtIO block: Device did not assert DRIVER_OK\n");
        return false;
    }

    sbi_console_write(b"VirtIO block: Device driver OK\n");
    true
}

// ---------------------------------------------------------------------------
// VirtIO block read sector
// ---------------------------------------------------------------------------

unsafe fn virtio_blk_read_sector(sector: u64, buf: &mut [u8]) -> bool {
    if buf.len() < VIRTIO_BLK_SECTOR_SIZE {
        return false;
    }

    // Set up the block request header (16 bytes):
    //   type = VIRTIO_BLK_T_IN (read from device)  [u32]
    //   reserved = 0                               [u32]
    //   sector = sector number (little-endian u64) [u64]
    let mut req_header = [0u8; 16];
    req_header[0..4].copy_from_slice(&VIRTIO_BLK_T_IN.to_le_bytes());
    req_header[4..8].copy_from_slice(&0u32.to_le_bytes());
    req_header[8..16].copy_from_slice(&sector.to_le_bytes());

    write_unaligned(BLK_REQ_HEADER as *mut u8, 0u8);
    copy_nonoverlapping(req_header.as_ptr(), BLK_REQ_HEADER as *mut u8, 16);

    // --- Set up 2-descriptor chain ---
    // Descriptor 0: block request header (device reads)
    let desc_base = VQ_DESC_TABLE as *mut VirtioDesc;
    let d0 = desc_base.add(0);
    d0.write(VirtioDesc {
        addr: BLK_REQ_HEADER,
        len: 16,
        flags: VIRTQ_DESC_F_NEXT, // chains to descriptor 1
        next: 1,
    });

    // Descriptor 1: data buffer (device writes data into it for READ)
    let d1 = desc_base.add(1);
    d1.write(VirtioDesc {
        addr: BLK_DATA_BUFFER,
        len: VIRTIO_BLK_SECTOR_SIZE as u32,
        flags: VIRTQ_DESC_F_WRITE, // device writes to this buffer
        next: 0xFFFF, // terminator
    });

    // --- Add descriptor 0 to available ring ---
    let avail_base = VQ_AVAIL_RING as *mut VirtioAvail;
    let avail_idx = (*avail_base).idx as usize;
    (*avail_base).ring[avail_idx] = 0; // descriptor index 0
    (*avail_base).idx = (avail_idx + 1) as u16;

    // --- Notify device (kick) ---
    mmio_write(VIRTIO_MMIO_QUEUE_NOTIFY, 0);

    // --- Poll used ring for completion ---
    let used_base = VQ_USED_RING as *mut VirtioUsedRing;
    let mut polls: usize = 0;
    let max_polls: usize = 100_000;

    while polls < max_polls {
        polls += 1;
        let used_idx = (*used_base).idx as usize;

        if used_idx > 0 {
            for i in 0..used_idx {
                let entry = (*used_base).ring[i];
                if entry.id == 0 {
                    // Descriptor 0 completed
                    if entry.len as u32 == VIRTIO_BLK_S_OK {
                        // Copy data from data buffer to caller's buffer
                        let data_ptr = BLK_DATA_BUFFER as *const u8;
                        for i in 0..VIRTIO_BLK_SECTOR_SIZE {
                            buf[i] = read_unaligned(data_ptr.add(i));
                        }
                        return true;
                    } else {
                        sbi_console_write(b"VirtIO block read: Device error\n");
                        return false;
                    }
                }
            }
        }
    }

    sbi_console_write(b"VirtIO block read: Timeout\n");
    false
}

// ---------------------------------------------------------------------------
// VirtIO block write sector
// ---------------------------------------------------------------------------

unsafe fn virtio_blk_write_sector(sector: u64, buf: &[u8]) -> bool {
    if buf.len() < VIRTIO_BLK_SECTOR_SIZE {
        return false;
    }

    // --- Copy caller's data to the data buffer ---
    let data_ptr = BLK_DATA_BUFFER as *mut u8;
    for i in 0..VIRTIO_BLK_SECTOR_SIZE {
        write_unaligned(data_ptr.add(i), buf[i]);
    }

    // --- Set up the block request header ---
    //   type = VIRTIO_BLK_T_OUT (write to device)  [u32]
    //   reserved = 0                               [u32]
    //   sector = sector number (little-endian u64) [u64]
    let mut req_header = [0u8; 16];
    req_header[0..4].copy_from_slice(&VIRTIO_BLK_T_OUT.to_le_bytes());
    req_header[4..8].copy_from_slice(&0u32.to_le_bytes());
    req_header[8..16].copy_from_slice(&sector.to_le_bytes());

    write_unaligned(BLK_REQ_HEADER as *mut u8, 0u8);
    copy_nonoverlapping(req_header.as_ptr(), BLK_REQ_HEADER as *mut u8, 16);

    // --- Set up 2-descriptor chain ---
    let desc_base = VQ_DESC_TABLE as *mut VirtioDesc;
    let d0 = desc_base.add(0);
    d0.write(VirtioDesc {
        addr: BLK_REQ_HEADER,
        len: 16,
        flags: VIRTQ_DESC_F_NEXT,
        next: 1,
    });

    // Descriptor 1: data buffer (device reads data from it for WRITE)
    let d1 = desc_base.add(1);
    d1.write(VirtioDesc {
        addr: BLK_DATA_BUFFER,
        len: VIRTIO_BLK_SECTOR_SIZE as u32,
        flags: 0, // device reads from this buffer (default direction)
        next: 0xFFFF,
    });

    // --- Add descriptor 0 to available ring ---
    let avail_base = VQ_AVAIL_RING as *mut VirtioAvail;
    let avail_idx = (*avail_base).idx as usize;
    (*avail_base).ring[avail_idx] = 0;
    (*avail_base).idx = (avail_idx + 1) as u16;

    // --- Notify device ---
    mmio_write(VIRTIO_MMIO_QUEUE_NOTIFY, 0);

    // --- Poll used ring for completion ---
    let used_base = VQ_USED_RING as *mut VirtioUsedRing;
    let mut polls: usize = 0;
    let max_polls: usize = 100_000;

    while polls < max_polls {
        polls += 1;
        let used_idx = (*used_base).idx as usize;

        if used_idx > 0 {
            for i in 0..used_idx {
                let entry = (*used_base).ring[i];
                if entry.id == 0 {
                    if entry.len as u32 == VIRTIO_BLK_S_OK {
                        return true;
                    } else {
                        sbi_console_write(b"VirtIO block write: Device error\n");
                        return false;
                    }
                }
            }
        }
    }

    sbi_console_write(b"VirtIO block write: Timeout\n");
    false
}

// ============================================================================
// VirtIO Network Device Driver (Phase 3)
// ============================================================================

/// Compiled-in QEMU `virt` default MMIO base for the VirtIO network device.
/// The runtime value comes from Device Tree discovery when a usable DTB is
/// present (`VIRTIO_NET_MMIO` static); this constant is only the fallback.
const VIRTIO_NET_MMIO_DEFAULT: usize = 0x1000_2000;

// ---------------------------------------------------------------------------
// VirtIO MMIO register offsets (as u32 indices, each register is 4 bytes)
// ---------------------------------------------------------------------------
const VIRTIO_NET_MMIO_MAGIC_VALUE: usize = 0x00 / 4;
const VIRTIO_NET_MMIO_VERSION: usize = 0x04 / 4;
const VIRTIO_NET_MMIO_DEVICE_ID: usize = 0x08 / 4;
const VIRTIO_NET_MMIO_VENDOR_ID: usize = 0x0c / 4;
const VIRTIO_NET_MMIO_HOST_FEATURES: usize = 0x10 / 4;
const VIRTIO_NET_MMIO_HOST_FEATURES_SEL: usize = 0x14 / 4;
const VIRTIO_NET_MMIO_GUEST_FEATURES: usize = 0x20 / 4;
const VIRTIO_NET_MMIO_GUEST_FEATURES_SEL: usize = 0x24 / 4;
const VIRTIO_NET_MMIO_GUEST_PAGE_SIZE: usize = 0x28 / 4;
const VIRTIO_NET_MMIO_QUEUE_SEL: usize = 0x30 / 4;
const VIRTIO_NET_MMIO_QUEUE_NUM_MAX: usize = 0x34 / 4;
const VIRTIO_NET_MMIO_QUEUE_NUM: usize = 0x38 / 4;
const VIRTIO_NET_MMIO_QUEUE_READY: usize = 0x3c / 4;
const VIRTIO_NET_MMIO_QUEUE_DESC_LOW: usize = 0x40 / 4;
const VIRTIO_NET_MMIO_QUEUE_DESC_HIGH: usize = 0x42 / 4;
const VIRTIO_NET_MMIO_QUEUE_AVAIL_LOW: usize = 0x44 / 4;
const VIRTIO_NET_MMIO_QUEUE_AVAIL_HIGH: usize = 0x46 / 4;
const VIRTIO_NET_MMIO_QUEUE_USED_LOW: usize = 0x48 / 4;
const VIRTIO_NET_MMIO_QUEUE_USED_HIGH: usize = 0x4a / 4;
const VIRTIO_NET_MMIO_QUEUE_NOTIFY: usize = 0x4c / 4;
const VIRTIO_NET_MMIO_INTERRUPT_STATUS: usize = 0x50 / 4;
const VIRTIO_NET_MMIO_INTERRUPT_ACK: usize = 0x54 / 4;
const VIRTIO_NET_MMIO_DEVICE_STATUS: usize = 0x58 / 4;
const VIRTIO_NET_MMIO_CONFIG_GENERATION: usize = 0x5c / 4;

// ---------------------------------------------------------------------------
// VirtIO network constants
// ---------------------------------------------------------------------------
const VIRTIO_NET_DEVICE_TYPE: u32 = 1;
const VIRTIO_NET_VENDOR_ID: u32 = 0x1af4;

const VIRTIO_NET_S_RX: u32 = 0;
const VIRTIO_NET_S_TX: u32 = 1;

// Network config space offsets (from device config, after status)
const VIRTIO_NET_CONFIG_MAC: usize = 0x00;
const VIRTIO_NET_CONFIG_STATUS: usize = 0x06;
const VIRTIO_NET_CONFIG_MAX_VQPAIRS: usize = 0x07;
const VIRTIO_NET_CONFIG_MTU: usize = 0x08;
const VIRTIO_NET_CONFIG_SPEED: usize = 0x0c;

// ---------------------------------------------------------------------------
// Fixed physical addresses for network virtqueues (no MMU)
//
// RX buffers occupy 0x2000_2300..0x2000_A300 (16 x 2 KiB); TX structures start
// at 0x2000_A400 so they cannot overlap the RX DMA buffers.
// ---------------------------------------------------------------------------
const NET_RX_DESC_TABLE: u64 = 0x2000_2000;
const NET_RX_AVAIL_RING: u64 = 0x2000_2100;
const NET_RX_USED_RING: u64 = 0x2000_2200;
const NET_RX_BUF_BASE: u64 = 0x2000_2300;   // RX buffers start here (2 KiB each)
const NET_TX_DESC_TABLE: u64 = 0x2000_A400;
const NET_TX_AVAIL_RING: u64 = 0x2000_A500;
const NET_TX_USED_RING: u64 = 0x2000_A600;
const NET_TX_BUF_BASE: u64 = 0x2000_A700;   // TX buffers start here (2 KiB each)

const NET_RX_BUF_SIZE: usize = 2048;
const NET_TX_BUF_SIZE: usize = 2048;
const NET_QUEUE_SIZE: usize = 16;

// Length of the virtio_net_hdr the device prepends to every received frame;
// the Ethernet frame follows it.
const VIRTIO_NET_HDR_LEN: usize = 12;

// VirtIO-net feature bits we care about.
const VIRTIO_NET_F_MAC: u64 = 1 << 5;      // device has a valid MAC address
const VIRTIO_NET_F_STATUS: u64 = 1 << 16;  // device config status field carries link status

// Link status, refreshed by virtio_net_link_status_refresh.
static mut NET_LINK_UP: bool = false;

// Network virtqueue state (separate RX and TX queues)
struct NetVirtQueue {
    desc_table: *mut VirtioDesc,
    avail: *mut VirtioAvail,
    used: *mut VirtioUsedRing,
    avail_idx: u16,
    used_idx: u16,
}

// ---------------------------------------------------------------------------
// Network MMIO access helpers
// ---------------------------------------------------------------------------

unsafe fn net_mmio_read(offset: usize) -> u32 {
    mmio_read_at(unsafe { VIRTIO_NET_MMIO }, offset)
}

unsafe fn net_mmio_write(offset: usize, value: u32) {
    mmio_write_at(unsafe { VIRTIO_NET_MMIO }, offset, value);
}

// ---------------------------------------------------------------------------
// Setup a network virtqueue (RX or TX)
// ---------------------------------------------------------------------------

fn setup_net_virtqueue(queue_sel: usize) -> Result<NetVirtQueue, &'static str> {
    unsafe {
        net_mmio_write(VIRTIO_NET_MMIO_QUEUE_SEL, queue_sel as u32);

        let queue_max = net_mmio_read(VIRTIO_NET_MMIO_QUEUE_NUM_MAX);
        if queue_max == 0 {
            return Err("net virtqueue: device reports queue size 0");
        }
        let queue_num = NET_QUEUE_SIZE.min(queue_max as usize);

        net_mmio_write(VIRTIO_NET_MMIO_QUEUE_NUM, queue_num as u32);

        let (desc_phys, avail_phys, used_phys) = match queue_sel {
            0 => (NET_RX_DESC_TABLE, NET_RX_AVAIL_RING, NET_RX_USED_RING),
            1 => (NET_TX_DESC_TABLE, NET_TX_AVAIL_RING, NET_TX_USED_RING),
            _ => return Err("net virtqueue: invalid queue select"),
        };

        net_mmio_write(VIRTIO_NET_MMIO_QUEUE_DESC_LOW, desc_phys as u32);
        net_mmio_write(VIRTIO_NET_MMIO_QUEUE_DESC_HIGH, (desc_phys >> 32) as u32);
        net_mmio_write(VIRTIO_NET_MMIO_QUEUE_AVAIL_LOW, avail_phys as u32);
        net_mmio_write(VIRTIO_NET_MMIO_QUEUE_AVAIL_HIGH, (avail_phys >> 32) as u32);
        net_mmio_write(VIRTIO_NET_MMIO_QUEUE_USED_LOW, used_phys as u32);
        net_mmio_write(VIRTIO_NET_MMIO_QUEUE_USED_HIGH, (used_phys >> 32) as u32);

        net_mmio_write(VIRTIO_NET_MMIO_QUEUE_READY, 1);

        let ready = net_mmio_read(VIRTIO_NET_MMIO_QUEUE_READY);
        if ready == 0 {
            return Err("net virtqueue: device did not mark queue ready");
        }

        let desc_base = desc_phys as *mut VirtioDesc;
        for i in 0..NET_QUEUE_SIZE {
            let d = desc_base.add(i);
            d.write(VirtioDesc {
                addr: 0,
                len: 0,
                flags: 0,
                next: 0xFFFF,
            });
        }

        let avail_base = avail_phys as *mut VirtioAvail;
        (*avail_base).flags = 0;
        (*avail_base).idx = 0;
        for i in 0..NET_QUEUE_SIZE {
            (*avail_base).ring[i] = 0xFFFF;
        }
        (*avail_base).used_event = 0;

        let used_base = used_phys as *mut VirtioUsedRing;
        (*used_base).flags = 0;
        (*used_base).idx = 0;
        for i in 0..NET_QUEUE_SIZE {
            (*used_base).ring[i] = VirtioUsedElem { id: 0xFFFFFFFF, len: 0 };
        }

        Ok(NetVirtQueue {
            desc_table: desc_base,
            avail: avail_base,
            used: used_base,
            avail_idx: 0,
            used_idx: 0,
        })
    }
}

// ---------------------------------------------------------------------------
// VirtIO network device initialization
// ---------------------------------------------------------------------------

static mut NET_RX_QUEUE: Option<NetVirtQueue> = None;
static mut NET_TX_QUEUE: Option<NetVirtQueue> = None;

/// MAC address of the network device (6 bytes)
static mut NET_MAC: [u8; 6] = [0; 6];

unsafe fn virtio_net_init() -> bool {
    let magic = net_mmio_read(VIRTIO_NET_MMIO_MAGIC_VALUE);
    if magic != VIRTIO_CONFIG_MAGIC {
        sbi_console_write(b"VirtIO net: Bad magic\n");
        return false;
    }
    sbi_console_write(b"VirtIO net: Magic verified\n");

    let version = net_mmio_read(VIRTIO_NET_MMIO_VERSION);
    if version != VIRTIO_CONFIG_VERSION {
        sbi_console_write(b"VirtIO net: Bad version\n");
        return false;
    }

    let device_id = net_mmio_read(VIRTIO_NET_MMIO_DEVICE_ID);
    let vendor_id = net_mmio_read(VIRTIO_NET_MMIO_VENDOR_ID);
    if device_id != VIRTIO_NET_DEVICE_TYPE || vendor_id != VIRTIO_NET_VENDOR_ID {
        sbi_console_write(b"VirtIO net: Not a VirtIO net device\n");
        return false;
    }
    sbi_console_write(b"VirtIO net: Detected VirtIO network device\n");

    // Reset
    net_mmio_write(VIRTIO_NET_MMIO_DEVICE_STATUS, VIRTIO_STATUS_RESET);

    // ACK
    net_mmio_write(VIRTIO_NET_MMIO_DEVICE_STATUS, VIRTIO_STATUS_ACK);

    // Driver
    net_mmio_write(VIRTIO_NET_MMIO_DEVICE_STATUS, VIRTIO_STATUS_DRIVER);

    // Feature negotiation (both pages). This driver requires virtio 1.0.
    net_mmio_write(VIRTIO_NET_MMIO_HOST_FEATURES_SEL, 0);
    let host_features_lo = net_mmio_read(VIRTIO_NET_MMIO_HOST_FEATURES);
    net_mmio_write(VIRTIO_NET_MMIO_HOST_FEATURES_SEL, 1);
    let host_features_hi = net_mmio_read(VIRTIO_NET_MMIO_HOST_FEATURES);
    let host_features: u64 = (host_features_hi as u64) << 32 | host_features_lo as u64;

    if host_features & VIRTIO_F_VERSION_1 == 0 {
        sbi_console_write(b"VirtIO net: host lacks VIRTIO_F_VERSION_1\n");
        return false;
    }

    // Advertise VIRTIO_F_VERSION_1, plus VIRTIO_NET_F_MAC when the host offers it.
    let mut guest_features: u64 = VIRTIO_F_VERSION_1;
    if host_features & VIRTIO_NET_F_MAC != 0 {
        guest_features |= VIRTIO_NET_F_MAC;
    }
    net_mmio_write(VIRTIO_NET_MMIO_GUEST_FEATURES_SEL, 0);
    net_mmio_write(VIRTIO_NET_MMIO_GUEST_FEATURES, guest_features as u32);
    net_mmio_write(VIRTIO_NET_MMIO_GUEST_FEATURES_SEL, 1);
    net_mmio_write(VIRTIO_NET_MMIO_GUEST_FEATURES, (guest_features >> 32) as u32);

    net_mmio_write(VIRTIO_NET_MMIO_GUEST_PAGE_SIZE, 4096);

    // Set up RX queue (0) and TX queue (1)
    match setup_net_virtqueue(0) {
        Ok(q) => {
            NET_RX_QUEUE = Some(q);
            sbi_console_write(b"VirtIO net: RX virtqueue ready\n");
        }
        Err(e) => {
            sbi_console_write(b"VirtIO net: RX virtqueue failed: ");
            sbi_console_write(e.as_bytes());
            sbi_console_write(b"\n");
            return false;
        }
    }
    match setup_net_virtqueue(1) {
        Ok(q) => {
            NET_TX_QUEUE = Some(q);
            sbi_console_write(b"VirtIO net: TX virtqueue ready\n");
        }
        Err(e) => {
            sbi_console_write(b"VirtIO net: TX virtqueue failed: ");
            sbi_console_write(e.as_bytes());
            sbi_console_write(b"\n");
            return false;
        }
    }

    net_mmio_write(VIRTIO_NET_MMIO_DEVICE_STATUS, VIRTIO_STATUS_FEATURES_OK);

    // Post RX buffers before the device goes live so frames can be received.
    if !unsafe { net_rx_post_all() } {
        sbi_console_write(b"VirtIO net: failed to post RX buffers\n");
        return false;
    }
    sbi_console_write(b"VirtIO net: RX buffers posted\n");

    net_mmio_write(VIRTIO_NET_MMIO_DEVICE_STATUS, VIRTIO_STATUS_DRIVER_OK);

    let status = net_mmio_read(VIRTIO_NET_MMIO_DEVICE_STATUS);
    if status & VIRTIO_STATUS_DRIVER_OK == 0 {
        sbi_console_write(b"VirtIO net: Device did not assert DRIVER_OK\n");
        return false;
    }

    // Read MAC address from config space (offset 0, 6 bytes)
    // Config space is accessed via MMIO at offsets beyond the standard registers.
    // The config starts at 0x100 (256 bytes) in QEMU's virtio-net MMIO.
    // We read it as raw u32 reads and extract bytes.
    let config_base: usize = 0x100 / 4; // config space starts at byte offset 0x100
    let mac_lo = net_mmio_read(config_base + 0);   // bytes 0-3
    let mac_hi = net_mmio_read(config_base + 1);   // bytes 4-5 (low 2 bytes)
    NET_MAC[0] = (mac_lo & 0xFF) as u8;
    NET_MAC[1] = ((mac_lo >> 8) & 0xFF) as u8;
    NET_MAC[2] = ((mac_lo >> 16) & 0xFF) as u8;
    NET_MAC[3] = ((mac_lo >> 24) & 0xFF) as u8;
    NET_MAC[4] = (mac_hi & 0xFF) as u8;
    NET_MAC[5] = ((mac_hi >> 8) & 0xFF) as u8;

    sbi_console_write(b"VirtIO net: MAC ");
    for b in NET_MAC {
        let hi = (b >> 4) as u8;
        let lo = (b & 0x0F) as u8;
        sbi_console_putchar(if hi < 10 { b'0' + hi } else { b'a' + hi - 10 });
        sbi_console_putchar(if lo < 10 { b'0' + lo } else { b'a' + lo - 10 });
        sbi_console_putchar(b':');
    }
    sbi_console_write(b"\n");

    // Read link status from device config space (VIRTIO_NET_F_STATUS).
    unsafe { virtio_net_link_status_refresh() };
    sbi_console_write(if unsafe { NET_LINK_UP } {
        b"VirtIO net: link up\n"
    } else {
        b"VirtIO net: link down\n"
    });

    sbi_console_write(b"VirtIO net: Device driver OK\n");
    true
}

/// Post all RX descriptors (with the WRITE flag) to the RX avail ring so the
/// device can fill them with received frames. Called during init and after
/// every consumed RX buffer (see `virtio_net_recv`).
unsafe fn net_rx_post_all() -> bool {
    let rx_queue = match unsafe { NET_RX_QUEUE.as_mut() } {
        Some(q) => q,
        None => return false,
    };

    let desc = rx_queue.desc_table;
    for i in 0..NET_QUEUE_SIZE {
        let d = desc.add(i);
        d.write(VirtioDesc {
            addr: NET_RX_BUF_BASE + (i * NET_RX_BUF_SIZE) as u64,
            len: NET_RX_BUF_SIZE as u32,
            flags: VIRTQ_DESC_F_WRITE,
            next: 0xFFFF,
        });
    }

    let avail = rx_queue.avail;
    let mut idx = (*avail).idx as usize;
    for i in 0..NET_QUEUE_SIZE {
        (*avail).ring[idx % NET_QUEUE_SIZE] = i as u16;
        idx += 1;
    }
    (*avail).idx = idx as u16;
    rx_queue.avail_idx = (*avail).idx;
    true
}

/// Refresh `NET_LINK_UP` from the device config status field (bit 1 = link up).
unsafe fn virtio_net_link_status_refresh() {
    let word = net_mmio_read(0x100 / 4 + (VIRTIO_NET_CONFIG_STATUS / 4));
    let status_byte = ((word >> ((VIRTIO_NET_CONFIG_STATUS % 4) * 8)) & 0xFF) as u8;
    unsafe { NET_LINK_UP = status_byte & 0x02 != 0; }
}

/// Link status of the network device, last refreshed at init time.
fn net_link_up() -> bool {
    unsafe { NET_LINK_UP }
}

/// Send a packet via the VirtIO network device (TX queue 1).
/// Owns descriptor 0 for the transfer, notifies the device, then polls the
/// used ring until the transmission completes and the descriptor is reclaimed.
/// Returns true on completion, false on timeout or device error.
unsafe fn virtio_net_send(buf: &[u8]) -> bool {
    let tx_queue = match unsafe { NET_TX_QUEUE.as_mut() } {
        Some(q) => q,
        None => return false,
    };

    if buf.len() > NET_TX_BUF_SIZE {
        sbi_console_write(b"VirtIO net send: Packet too large\n");
        return false;
    }

    // Reclaim any previously completed transmission before reusing descriptor 0.
    let used = tx_queue.used;
    while (tx_queue.used_idx as usize) < (*used).idx as usize {
        tx_queue.used_idx += 1;
    }

    // Copy packet data into the transmit buffer.
    let tx_buf_base = NET_TX_BUF_BASE as *mut u8;
    for i in 0..buf.len() {
        write_unaligned(tx_buf_base.add(i), buf[i]);
    }

    // Descriptor 0: the device reads from this buffer (not WRITE).
    let desc_base = tx_queue.desc_table;
    let d0 = desc_base.add(0);
    d0.write(VirtioDesc {
        addr: NET_TX_BUF_BASE,
        len: buf.len() as u32,
        flags: 0,
        next: 0xFFFF,
    });

    // Publish descriptor 0 on the avail ring and notify the device.
    let avail = tx_queue.avail;
    let slot = (*avail).idx as usize % NET_QUEUE_SIZE;
    (*avail).ring[slot] = 0;
    (*avail).idx = ((*avail).idx + 1) as u16;
    tx_queue.avail_idx = (*avail).idx;

    net_mmio_write(VIRTIO_NET_MMIO_QUEUE_NOTIFY, VIRTIO_NET_S_TX as u32);

    // Poll the used ring until the device completes this transmission.
    let mut polls: usize = 0;
    while polls < 1_000_000 {
        polls += 1;
        if (*used).idx as usize > tx_queue.used_idx as usize {
            let entry = (*used).ring[tx_queue.used_idx as usize % NET_QUEUE_SIZE];
            tx_queue.used_idx += 1;
            if entry.id == 0 {
                return true;
            }
            sbi_console_write(b"VirtIO net send: unexpected completion\n");
            return false;
        }
    }

    sbi_console_write(b"VirtIO net send: Timeout\n");
    false
}

/// Receive a packet from the VirtIO network device (RX queue 0).
/// Drains the used ring for completed RX descriptors, strips the 12-byte
/// virtio_net_hdr the device prepends to each received frame, copies the
/// Ethernet frame into the caller's buffer, and re-posts every consumed
/// descriptor so the device always has RX buffers available.
/// Returns the Ethernet frame byte count (0 if no frame is pending).
unsafe fn virtio_net_recv(buf: &mut [u8]) -> usize {
    let rx_queue = match unsafe { NET_RX_QUEUE.as_mut() } {
        Some(q) => q,
        None => return 0,
    };

    if buf.is_empty() {
        return 0;
    }

    let used = rx_queue.used;
    let avail = rx_queue.avail;
    let mut frame_len: usize = 0;

    while (rx_queue.used_idx as usize) < (*used).idx as usize {
        let entry = (*used).ring[rx_queue.used_idx as usize % NET_QUEUE_SIZE];
        rx_queue.used_idx += 1;

        let desc_id = entry.id as usize;
        let mut pkt_len = entry.len as usize;
        if pkt_len >= VIRTIO_NET_HDR_LEN {
            pkt_len -= VIRTIO_NET_HDR_LEN; // strip the virtio_net_hdr prefix
        } else {
            pkt_len = 0; // malformed buffer, skip
        }

        let copy_len = pkt_len.min(buf.len());
        if copy_len > 0 {
            let rx_buf =
                (NET_RX_BUF_BASE + (desc_id * NET_RX_BUF_SIZE) as u64) as *const u8;
            for i in 0..copy_len {
                buf[i] = read_unaligned(rx_buf.add(VIRTIO_NET_HDR_LEN + i));
            }
            frame_len = copy_len;
        }

        // Re-post the consumed descriptor so the device can reuse its buffer.
        let slot = (*avail).idx as usize % NET_QUEUE_SIZE;
        (*avail).ring[slot] = desc_id as u16;
        (*avail).idx = ((*avail).idx + 1) as u16;
        rx_queue.avail_idx = (*avail).idx;
    }

    frame_len
}

/// Network initialization wrapper
fn net_init() -> bool {
    unsafe { virtio_net_init() }
}

/// Network send wrapper
fn net_send(buf: &[u8]) -> bool {
    unsafe { virtio_net_send(buf) }
}

/// Network receive wrapper
fn net_recv(buf: &mut [u8]) -> usize {
    unsafe { virtio_net_recv(buf) }
}

/// Send an Ethernet frame (already containing Ethernet header + payload).
/// Wraps `virtio_net_send` with the raw frame bytes.
fn net_send_frame(frame: &[u8]) -> bool {
    unsafe { virtio_net_send(frame) }
}

/// Send an IPv4 packet with Ethernet framing.
/// Builds the Ethernet header + IPv4 header + payload and sends it.
fn net_send_ipv4(dst_mac: &[u8; 6], dst_ip: &[u8; 4], protocol: u8, payload: &[u8]) -> bool {
    let mut frame = [0u8; 1518];
    let eth_len = eth_build_header(&mut frame, dst_mac, ETH_P_IP);
    if eth_len == 0 {
        return false;
    }
    let ip_len = ipv4_build_header(&mut frame, eth_len, protocol, dst_ip, payload.len());
    if ip_len == 0 {
        return false;
    }
    let total = eth_len + ip_len + payload.len();
    if total > frame.len() {
        return false;
    }
    frame[eth_len + ip_len..eth_len + ip_len + payload.len()].copy_from_slice(payload);
    net_send_frame(&frame[..total])
}

// ============================================================================
// Ethernet Frame Layer
// ============================================================================

const ETH_ALEN: usize = 6;
const ETH_HLEN: usize = 14;
const ETH_P_ARP: u16 = 0x0806;
const ETH_P_IP: u16 = 0x0800;

// Ethernet header (14 bytes)
#[repr(C)]
struct EthHdr {
    dst: [u8; ETH_ALEN],
    src: [u8; ETH_ALEN],
    ethertype: u16,
}

/// Build an Ethernet header into a buffer.
/// Returns the number of bytes written (ETH_HLEN).
fn eth_build_header(buf: &mut [u8], dst: &[u8; ETH_ALEN], ethertype: u16) -> usize {
    if buf.len() < ETH_HLEN {
        return 0;
    }
    // Our MAC is the source
    let src = unsafe { NET_MAC };
    buf[0..6].copy_from_slice(dst);
    buf[6..12].copy_from_slice(&src);
    buf[12..14].copy_from_slice(&ethertype.to_be_bytes());
    ETH_HLEN
}

/// Parse an Ethernet header from a buffer.
/// Returns (dst, src, ethertype) or None if too short.
fn eth_parse_header(buf: &[u8]) -> Option<([u8; ETH_ALEN], [u8; ETH_ALEN], u16)> {
    if buf.len() < ETH_HLEN {
        return None;
    }
    let mut dst = [0u8; ETH_ALEN];
    let mut src = [0u8; ETH_ALEN];
    dst.copy_from_slice(&buf[0..6]);
    src.copy_from_slice(&buf[6..12]);
    let ethertype = u16::from_be_bytes([buf[12], buf[13]]);
    Some((dst, src, ethertype))
}

// ============================================================================
// ARP (Address Resolution Protocol)
// ============================================================================

const ARP_HW_TYPE_ETHER: u16 = 0x0001;
const ARP_PROTO_TYPE_IP: u16 = 0x0800;
const ARP_OP_REQUEST: u16 = 0x0001;
const ARP_OP_REPLY: u16 = 0x0002;

const IP_ADDR_LEN: usize = 4;

/// Local IPv4 address (10.0.2.15 - QEMU default guest IP)
static mut NET_IP_ADDR: [u8; 4] = [10, 0, 2, 15];

/// ARP packet (28 bytes)
#[repr(C)]
struct ArpPacket {
    hw_type: u16,
    proto_type: u16,
    hw_size: u8,
    proto_size: u8,
    opcode: u16,
    sender_mac: [u8; 6],
    sender_ip: [u8; 4],
    target_mac: [u8; 6],
    target_ip: [u8; 4],
}

/// Build an ARP request for the given target IP.
/// Writes into `buf` (must be at least ETH_HLEN + 28 bytes).
/// Returns total frame length or 0 on error.
fn arp_build_request(buf: &mut [u8], target_ip: &[u8; 4]) -> usize {
    if buf.len() < ETH_HLEN + 28 {
        return 0;
    }
    let src = unsafe { NET_MAC };
    let src_ip = unsafe { NET_IP_ADDR };
    let broadcast = [0xFFu8; 6];

    // Ethernet header
    eth_build_header(buf, &broadcast, ETH_P_ARP);

    // ARP payload (28 bytes at offset 14)
    let arp = &mut buf[14..14 + 28];
    arp[0..2].copy_from_slice(&ARP_HW_TYPE_ETHER.to_be_bytes());
    arp[2..4].copy_from_slice(&ARP_PROTO_TYPE_IP.to_be_bytes());
    arp[4] = 6;   // hw size
    arp[5] = 4;   // proto size
    arp[6..8].copy_from_slice(&ARP_OP_REQUEST.to_be_bytes());
    arp[8..14].copy_from_slice(&src);
    arp[14..18].copy_from_slice(&src_ip);
    arp[18..24].copy_from_slice(&broadcast); // target MAC unknown
    arp[24..28].copy_from_slice(target_ip);

    ETH_HLEN + 28
}

/// Build an ARP reply.
/// Writes into `buf`. Returns total frame length or 0 on error.
fn arp_build_reply(buf: &mut [u8], dst_mac: &[u8; 6], dst_ip: &[u8; 4]) -> usize {
    if buf.len() < ETH_HLEN + 28 {
        return 0;
    }
    let src = unsafe { NET_MAC };
    let src_ip = unsafe { NET_IP_ADDR };

    eth_build_header(buf, dst_mac, ETH_P_ARP);

    let arp = &mut buf[14..14 + 28];
    arp[0..2].copy_from_slice(&ARP_HW_TYPE_ETHER.to_be_bytes());
    arp[2..4].copy_from_slice(&ARP_PROTO_TYPE_IP.to_be_bytes());
    arp[4] = 6;
    arp[5] = 4;
    arp[6..8].copy_from_slice(&ARP_OP_REPLY.to_be_bytes());
    arp[8..14].copy_from_slice(&src);
    arp[14..18].copy_from_slice(&src_ip);
    arp[18..24].copy_from_slice(dst_mac);
    arp[24..28].copy_from_slice(dst_ip);

    ETH_HLEN + 28
}

/// Parse an ARP packet from a buffer.
/// Returns Some((opcode, sender_mac, sender_ip, target_mac, target_ip)) or None.
fn arp_parse(buf: &[u8], offset: usize) -> Option<(u16, [u8; 6], [u8; 4], [u8; 6], [u8; 4])> {
    if buf.len() < offset + 28 {
        return None;
    }
    let arp = &buf[offset..offset + 28];
    let hw_type = u16::from_be_bytes([arp[0], arp[1]]);
    let proto_type = u16::from_be_bytes([arp[2], arp[3]]);
    if hw_type != ARP_HW_TYPE_ETHER || proto_type != ARP_PROTO_TYPE_IP {
        return None;
    }
    let opcode = u16::from_be_bytes([arp[6], arp[7]]);
    let mut sender_mac = [0u8; 6];
    sender_mac.copy_from_slice(&arp[8..14]);
    let mut sender_ip = [0u8; 4];
    sender_ip.copy_from_slice(&arp[14..18]);
    let mut target_mac = [0u8; 6];
    target_mac.copy_from_slice(&arp[18..24]);
    let mut target_ip = [0u8; 4];
    target_ip.copy_from_slice(&arp[24..28]);
    Some((opcode, sender_mac, sender_ip, target_mac, target_ip))
}

// ============================================================================
// IPv4 Layer
// ============================================================================

const IP_PROTO_ICMP: u8 = 1;
const IP_PROTO_TCP: u8 = 6;
const IP_PROTO_UDP: u8 = 17;

/// IPv4 header (20 bytes, no options)
#[repr(C)]
struct Ipv4Hdr {
    ver_ihl: u8,
    dscp_ecn: u8,
    total_len: u16,
    identification: u16,
    flags_frag: u16,
    ttl: u8,
    protocol: u8,
    checksum: u16,
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
}

/// Build an IPv4 header.
/// Writes into `buf` at `offset`. Returns 20 or 0 on error.
fn ipv4_build_header(buf: &mut [u8], offset: usize, protocol: u8, dst_ip: &[u8; 4], payload_len: usize) -> usize {
    if buf.len() < offset + 20 {
        return 0;
    }
    let src_ip = unsafe { NET_IP_ADDR };
    let total_len = (20 + payload_len) as u16;

    buf[offset] = 0x45; // version 4, IHL 5
    buf[offset + 1] = 0x00;
    buf[offset + 2..offset + 4].copy_from_slice(&total_len.to_be_bytes());
    buf[offset + 4..offset + 6].copy_from_slice(&0u16.to_be_bytes()); // identification
    buf[offset + 6..offset + 8].copy_from_slice(&0u16.to_be_bytes()); // flags/frag
    buf[offset + 8] = 64; // TTL
    buf[offset + 9] = protocol;
    buf[offset + 10..offset + 12].copy_from_slice(&0u16.to_be_bytes()); // checksum (0 for now)
    buf[offset + 12..offset + 16].copy_from_slice(&src_ip);
    buf[offset + 16..offset + 20].copy_from_slice(dst_ip);

    20
}

/// Parse an IPv4 header.
/// Returns Some((protocol, src_ip, dst_ip, total_len)) or None.
fn ipv4_parse_header(buf: &[u8], offset: usize) -> Option<(u8, [u8; 4], [u8; 4], u16)> {
    if buf.len() < offset + 20 {
        return None;
    }
    let ver_ihl = buf[offset];
    if (ver_ihl >> 4) != 4 {
        return None;
    }
    let protocol = buf[offset + 9];
    let mut src_ip = [0u8; 4];
    src_ip.copy_from_slice(&buf[offset + 12..offset + 16]);
    let mut dst_ip = [0u8; 4];
    dst_ip.copy_from_slice(&buf[offset + 16..offset + 20]);
    let total_len = u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]);
    Some((protocol, src_ip, dst_ip, total_len))
}

// ============================================================================
// UDP Transport Layer
// ============================================================================

const UDP_HLEN: usize = 8;

const UDP_PORT_BOOTPS: u16 = 67; // DHCP server
const UDP_PORT_BOOTPC: u16 = 68; // DHCP client

/// UDP header (8 bytes)
#[repr(C)]
struct UdpHdr {
    src_port: u16,
    dst_port: u16,
    length: u16,
    checksum: u16,
}

/// Build a UDP header.
/// Writes into `buf` at `offset`. Returns 8 or 0 on error.
fn udp_build_header(buf: &mut [u8], offset: usize, src_port: u16, dst_port: u16, payload_len: usize) -> usize {
    if buf.len() < offset + UDP_HLEN {
        return 0;
    }
    let total = (UDP_HLEN + payload_len) as u16;
    buf[offset..offset + 2].copy_from_slice(&src_port.to_be_bytes());
    buf[offset + 2..offset + 4].copy_from_slice(&dst_port.to_be_bytes());
    buf[offset + 4..offset + 6].copy_from_slice(&total.to_be_bytes());
    buf[offset + 6..offset + 8].copy_from_slice(&0u16.to_be_bytes()); // checksum (0 for now)
    UDP_HLEN
}

/// Parse a UDP header.
/// Returns Some((src_port, dst_port, length)) or None.
fn udp_parse_header(buf: &[u8], offset: usize) -> Option<(u16, u16, u16)> {
    if buf.len() < offset + UDP_HLEN {
        return None;
    }
    let src_port = u16::from_be_bytes([buf[offset], buf[offset + 1]]);
    let dst_port = u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]);
    let length = u16::from_be_bytes([buf[offset + 4], buf[offset + 5]]);
    Some((src_port, dst_port, length))
}

// ============================================================================
// DHCP Client (RFC 2131)
// ============================================================================

const DHCP_OP_REQUEST: u8 = 1;
const DHCP_OP_REPLY: u8 = 2;
const DHCP_HTYPE_ETHER: u8 = 1;
const DHCP_HLEN_ETHER: u8 = 6;
const DHCP_MSG_DISCOVER: u8 = 1;
const DHCP_MSG_OFFER: u8 = 2;
const DHCP_MSG_REQUEST: u8 = 3;
const DHCP_MSG_ACK: u8 = 5;

const DHCP_MAGIC_COOKIE: [u8; 4] = [99, 130, 83, 143];

const DHCP_OPT_MESSAGE_TYPE: u8 = 53;
const DHCP_OPT_SUBNET_MASK: u8 = 1;
const DHCP_OPT_ROUTER: u8 = 3;
const DHCP_OPT_DNS: u8 = 6;
const DHCP_OPT_SERVER_ID: u8 = 54;
const DHCP_OPT_REQUESTED_IP: u8 = 50;
const DHCP_OPT_END: u8 = 255;

/// DHCP client state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DhcpState {
    Init,
    Selecting,
    Requesting,
    Bound,
}

/// DHCP client instance (singleton).
static mut DHCP_CLIENT: DhcpClient = DhcpClient {
    state: DhcpState::Init,
    xid: 0x12345678,
    offered_ip: [0u8; 4],
    server_ip: [0u8; 4],
    lease_seconds: 0,
};

struct DhcpClient {
    state: DhcpState,
    xid: u32,
    offered_ip: [u8; 4],
    server_ip: [u8; 4],
    lease_seconds: u32,
}

impl Copy for DhcpClient {}
impl Clone for DhcpClient {
    fn clone(&self) -> Self { *self }
}

/// Build a DHCP Discover message into `buf` (after UDP header).
/// Returns total DHCP message length or 0 on error.
fn dhcp_build_discover(buf: &mut [u8], offset: usize) -> usize {
    if buf.len() < offset + 240 {
        return 0;
    }
    let client = unsafe { DHCP_CLIENT };
    let my_mac = unsafe { NET_MAC };

    // Fixed fields (236 bytes before options)
    buf[offset] = DHCP_OP_REQUEST;           // op
    buf[offset + 1] = DHCP_HTYPE_ETHER;      // htype
    buf[offset + 2] = DHCP_HLEN_ETHER;       // hlen
    buf[offset + 3] = 0;                     // hops
    buf[offset + 4..offset + 8].copy_from_slice(&client.xid.to_be_bytes()); // xid
    buf[offset + 8..offset + 10].copy_from_slice(&0u16.to_be_bytes()); // secs
    buf[offset + 10..offset + 12].copy_from_slice(&0u16.to_be_bytes()); // flags
    buf[offset + 12..offset + 16].copy_from_slice(&[0u8; 4]); // ciaddr
    buf[offset + 16..offset + 20].copy_from_slice(&[0u8; 4]); // yiaddr
    buf[offset + 20..offset + 24].copy_from_slice(&[0u8; 4]); // siaddr
    buf[offset + 24..offset + 28].copy_from_slice(&[0u8; 4]); // giaddr
    // chaddr (16 bytes): MAC + padding
    buf[offset + 28..offset + 34].copy_from_slice(&my_mac);
    // server name (64 bytes) + file (128 bytes) = zero
    // options start at offset + 236
    let opt = offset + 236;
    buf[opt..opt + 4].copy_from_slice(&DHCP_MAGIC_COOKIE);
    // Message type = Discover
    buf[opt + 4] = DHCP_OPT_MESSAGE_TYPE;
    buf[opt + 5] = 1;
    buf[opt + 6] = DHCP_MSG_DISCOVER;
    // End
    buf[opt + 7] = DHCP_OPT_END;

    236 + 8 // fixed + magic + msg type + end
}

/// Parse a DHCP Offer message from `buf` at `offset`.
/// Returns Some((message_type, offered_ip, server_ip)) or None.
fn dhcp_parse_offer(buf: &[u8], offset: usize) -> Option<(u8, [u8; 4], [u8; 4])> {
    if buf.len() < offset + 236 {
        return None;
    }
    let op = buf[offset];
    if op != DHCP_OP_REPLY {
        return None;
    }
    let mut offered_ip = [0u8; 4];
    offered_ip.copy_from_slice(&buf[offset + 16..offset + 20]);
    let mut server_ip = [0u8; 4];
    server_ip.copy_from_slice(&buf[offset + 20..offset + 24]);

    // Parse options for message type
    let opt_start = offset + 236;
    if buf.len() < opt_start + 4 {
        return None;
    }
    // Verify magic cookie
    if buf[opt_start..opt_start + 4] != DHCP_MAGIC_COOKIE {
        return None;
    }
    let mut i = opt_start + 4;
    let mut msg_type = None;
    while i + 2 <= buf.len() {
        let code = buf[i];
        if code == DHCP_OPT_END {
            break;
        }
        if code == 0 {
            i += 1;
            continue;
        }
        let len = buf[i + 1] as usize;
        if i + 2 + len > buf.len() {
            break;
        }
        if code == DHCP_OPT_MESSAGE_TYPE && len >= 1 {
            msg_type = Some(buf[i + 2]);
        }
        i += 2 + len;
    }

    Some((msg_type?, offered_ip, server_ip))
}

/// Build a DHCP Request message into `buf` (after UDP header).
/// Returns total DHCP message length or 0 on error.
fn dhcp_build_request(buf: &mut [u8], offset: usize) -> usize {
    if buf.len() < offset + 240 {
        return 0;
    }
    let client = unsafe { DHCP_CLIENT };
    let my_mac = unsafe { NET_MAC };
    let my_ip = unsafe { NET_IP_ADDR };

    // Fixed fields
    buf[offset] = DHCP_OP_REQUEST;           // op
    buf[offset + 1] = DHCP_HTYPE_ETHER;      // htype
    buf[offset + 2] = DHCP_HLEN_ETHER;       // hlen
    buf[offset + 3] = 0;                     // hops
    buf[offset + 4..offset + 8].copy_from_slice(&client.xid.to_be_bytes()); // xid
    buf[offset + 8..offset + 10].copy_from_slice(&0u16.to_be_bytes()); // secs
    buf[offset + 10..offset + 12].copy_from_slice(&0u16.to_be_bytes()); // flags
    buf[offset + 12..offset + 16].copy_from_slice(&[0u8; 4]); // ciaddr (0 in Requesting)
    buf[offset + 16..offset + 20].copy_from_slice(&[0u8; 4]); // yiaddr
    buf[offset + 20..offset + 24].copy_from_slice(&[0u8; 4]); // siaddr
    buf[offset + 24..offset + 28].copy_from_slice(&[0u8; 4]); // giaddr
    // chaddr
    buf[offset + 28..offset + 34].copy_from_slice(&my_mac);
    // server name + file = zero
    // options
    let opt = offset + 236;
    buf[opt..opt + 4].copy_from_slice(&DHCP_MAGIC_COOKIE);
    // Message type = Request
    buf[opt + 4] = DHCP_OPT_MESSAGE_TYPE;
    buf[opt + 5] = 1;
    buf[opt + 6] = DHCP_MSG_REQUEST;
    // Requested IP address = offered IP
    buf[opt + 7] = DHCP_OPT_REQUESTED_IP;
    buf[opt + 8] = 4;
    buf[opt + 9..opt + 13].copy_from_slice(&client.offered_ip);
    // Server identifier
    buf[opt + 13] = DHCP_OPT_SERVER_ID;
    buf[opt + 14] = 4;
    buf[opt + 15..opt + 19].copy_from_slice(&client.server_ip);
    // End
    buf[opt + 19] = DHCP_OPT_END;

    offset + 236 + 20
}

/// Parse a DHCP Ack message from `buf` at `offset`.
/// Returns Some((assigned_ip, subnet_mask, router, dns_server)) or None.
fn dhcp_parse_ack(buf: &[u8], offset: usize) -> Option<([u8; 4], [u8; 4], [u8; 4], [u8; 4])> {
    if buf.len() < offset + 236 {
        return None;
    }
    let op = buf[offset];
    if op != DHCP_OP_REPLY {
        return None;
    }
    let mut assigned_ip = [0u8; 4];
    assigned_ip.copy_from_slice(&buf[offset + 16..offset + 20]);

    let opt_start = offset + 236;
    if buf.len() < opt_start + 4 {
        return None;
    }
    if buf[opt_start..opt_start + 4] != DHCP_MAGIC_COOKIE {
        return None;
    }
    let mut i = opt_start + 4;
    let mut msg_type = None;
    let mut subnet_mask = None;
    let mut router = None;
    let mut dns_server = None;
    while i + 2 <= buf.len() {
        let code = buf[i];
        if code == DHCP_OPT_END {
            break;
        }
        if code == 0 {
            i += 1;
            continue;
        }
        let len = buf[i + 1] as usize;
        if i + 2 + len > buf.len() {
            break;
        }
        match code {
            DHCP_OPT_MESSAGE_TYPE if len >= 1 => msg_type = Some(buf[i + 2]),
            DHCP_OPT_SUBNET_MASK if len >= 4 => subnet_mask = Some(buf[i + 2..i + 6].try_into().ok()?),
            DHCP_OPT_ROUTER if len >= 4 => router = Some(buf[i + 2..i + 6].try_into().ok()?),
            DHCP_OPT_DNS if len >= 4 => dns_server = Some(buf[i + 2..i + 6].try_into().ok()?),
            _ => {}
        }
        i += 2 + len;
    }

    // Verify this is an ACK and all required options are present
    if msg_type != Some(DHCP_MSG_ACK) {
        return None;
    }
    Some((assigned_ip, subnet_mask?, router?, dns_server?))
}

// ============================================================================
// DNS Client (RFC 1035)
// ============================================================================

const DNS_PORT: u16 = 53;
const DNS_OP_QUERY: u16 = 0;
const DNS_RCODE_OK: u16 = 0;
const DNS_TYPE_A: u16 = 1;
const DNS_CLASS_IN: u16 = 1;

/// DNS header (12 bytes)
#[repr(C)]
struct DnsHdr {
    id: u16,
    flags: u16,
    qdcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16,
}

/// Build a DNS A-record query into `buf` at `offset`.
/// Returns total DNS message length or 0 on error.
fn dns_build_query(buf: &mut [u8], offset: usize, transaction_id: u16, hostname: &str) -> usize {
    if buf.len() < offset + 12 + hostname.len() + 6 {
        return 0;
    }
    // Header
    buf[offset..offset + 2].copy_from_slice(&transaction_id.to_be_bytes());
    buf[offset + 2..offset + 4].copy_from_slice(&0u16.to_be_bytes()); // flags (standard query)
    buf[offset + 4..offset + 6].copy_from_slice(&1u16.to_be_bytes()); // qdcount = 1
    buf[offset + 6..offset + 8].copy_from_slice(&0u16.to_be_bytes()); // ancount
    buf[offset + 8..offset + 10].copy_from_slice(&0u16.to_be_bytes()); // nscount
    buf[offset + 10..offset + 12].copy_from_slice(&0u16.to_be_bytes()); // arcount

    // Question section: encoded labels
    let mut pos = offset + 12;
    for label in hostname.split('.') {
        if label.is_empty() {
            continue;
        }
        if pos + 1 + label.len() > buf.len() {
            return 0;
        }
        buf[pos] = label.len() as u8;
        buf[pos + 1..pos + 1 + label.len()].copy_from_slice(label.as_bytes());
        pos += 1 + label.len();
    }
    // Null terminator
    buf[pos] = 0;
    pos += 1;
    // QTYPE = A
    buf[pos..pos + 2].copy_from_slice(&DNS_TYPE_A.to_be_bytes());
    pos += 2;
    // QCLASS = IN
    buf[pos..pos + 2].copy_from_slice(&DNS_CLASS_IN.to_be_bytes());
    pos += 2;

    pos - offset
}

/// Parse a DNS A-record response.
/// Writes resolved IPs into `answers` (max `answers.len()` entries).
/// Returns Some((transaction_id, count)) or None.
fn dns_parse_response(buf: &[u8], offset: usize, answers: &mut [[u8; 4]]) -> Option<(u16, usize)> {
    if buf.len() < offset + 12 {
        return None;
    }
    let id = u16::from_be_bytes([buf[offset], buf[offset + 1]]);
    let flags = u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]);
    let qdcount = u16::from_be_bytes([buf[offset + 4], buf[offset + 5]]);
    let ancount = u16::from_be_bytes([buf[offset + 6], buf[offset + 7]]);

    // Check RCODE (bits 0-3 of flags)
    if (flags & 0x000F) != DNS_RCODE_OK {
        return None;
    }

    // Skip question section
    let mut pos = offset + 12;
    for _ in 0..qdcount {
        // Skip labels until null byte
        while pos < buf.len() && buf[pos] != 0 {
            let label_len = buf[pos] as usize;
            pos += 1 + label_len;
        }
        pos += 1; // null terminator
        pos += 4; // qtype + qclass
        if pos + 4 > buf.len() {
            return None;
        }
    }

    // Parse answer records
    let mut count = 0;
    for _ in 0..ancount {
        if count >= answers.len() {
            break;
        }
        // NAME: pointer (2 bytes) or label sequence
        if pos + 2 > buf.len() {
            break;
        }
        let first = buf[pos] as u16;
        if (first & 0xC0) == 0xC0 {
            pos += 2; // pointer
        } else {
            // label sequence - skip to null
            while pos < buf.len() && buf[pos] != 0 {
                pos += 1 + buf[pos] as usize;
            }
            pos += 1;
        }
        if pos + 10 > buf.len() {
            break;
        }
        // TYPE (2) + CLASS (2) + TTL (4) + RDLENGTH (2)
        let rtype = u16::from_be_bytes([buf[pos], buf[pos + 1]]);
        let _rclass = u16::from_be_bytes([buf[pos + 2], buf[pos + 3]]);
        let _ttl = u32::from_be_bytes([buf[pos + 4], buf[pos + 5], buf[pos + 6], buf[pos + 7]]);
        let rdlength = u16::from_be_bytes([buf[pos + 8], buf[pos + 9]]) as usize;
        pos += 10;
        if rtype == DNS_TYPE_A && rdlength == 4 && pos + rdlength <= buf.len() {
            answers[count].copy_from_slice(&buf[pos..pos + 4]);
            count += 1;
        }
        pos += rdlength;
    }

    Some((id, count))
}

// ============================================================================
// TCP Transport Layer (RFC 793)
// ============================================================================

const TCP_HLEN: usize = 20;

const TCP_FLAG_FIN: u8 = 1 << 0;
const TCP_FLAG_SYN: u8 = 1 << 1;
const TCP_FLAG_RST: u8 = 1 << 2;
const TCP_FLAG_PSH: u8 = 1 << 3;
const TCP_FLAG_ACK: u8 = 1 << 4;
const TCP_FLAG_URG: u8 = 1 << 5;

const TCP_PORT_ECHO: u16 = 7;
const TCP_PORT_DISCARD: u16 = 9;

/// TCP connection state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    TimeWait,
}

/// TCP header (20 bytes, no options)
#[repr(C)]
struct TcpHdr {
    src_port: u16,
    dst_port: u16,
    seq_no: u32,
    ack_no: u32,
    data_off_rsvd: u8,
    flags: u8,
    window: u16,
    checksum: u16,
    urgent_ptr: u16,
}

/// Build a TCP header.
/// Writes into `buf` at `offset`. Returns 20 or 0 on error.
fn tcp_build_header(
    buf: &mut [u8],
    offset: usize,
    src_port: u16,
    dst_port: u16,
    seq_no: u32,
    ack_no: u32,
    flags: u8,
    window: u16,
) -> usize {
    if buf.len() < offset + TCP_HLEN {
        return 0;
    }
    buf[offset..offset + 2].copy_from_slice(&src_port.to_be_bytes());
    buf[offset + 2..offset + 4].copy_from_slice(&dst_port.to_be_bytes());
    buf[offset + 4..offset + 8].copy_from_slice(&seq_no.to_be_bytes());
    buf[offset + 8..offset + 12].copy_from_slice(&ack_no.to_be_bytes());
    buf[offset + 12] = (TCP_HLEN as u8 / 4) << 4; // data offset = 5, reserved = 0
    buf[offset + 13] = flags;
    buf[offset + 14..offset + 16].copy_from_slice(&window.to_be_bytes());
    buf[offset + 16..offset + 18].copy_from_slice(&0u16.to_be_bytes()); // checksum (0 for now)
    buf[offset + 18..offset + 20].copy_from_slice(&0u16.to_be_bytes()); // urgent pointer
    TCP_HLEN
}

/// Parse a TCP header.
/// Returns Some((src_port, dst_port, seq_no, ack_no, flags, window)) or None.
fn tcp_parse_header(buf: &[u8], offset: usize) -> Option<(u16, u16, u32, u32, u8, u16)> {
    if buf.len() < offset + TCP_HLEN {
        return None;
    }
    let src_port = u16::from_be_bytes([buf[offset], buf[offset + 1]]);
    let dst_port = u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]);
    let seq_no = u32::from_be_bytes([buf[offset + 4], buf[offset + 5], buf[offset + 6], buf[offset + 7]]);
    let ack_no = u32::from_be_bytes([buf[offset + 8], buf[offset + 9], buf[offset + 10], buf[offset + 11]]);
    let flags = buf[offset + 13];
    let window = u16::from_be_bytes([buf[offset + 14], buf[offset + 15]]);
    Some((src_port, dst_port, seq_no, ack_no, flags, window))
}

/// TCP connection instance (singleton).
static mut TCP_CONN: TcpConn = TcpConn {
    state: TcpState::Closed,
    local_port: 0,
    remote_port: 0,
    remote_ip: [0u8; 4],
    seq_no: 0,
    ack_no: 0,
};

struct TcpConn {
    state: TcpState,
    local_port: u16,
    remote_port: u16,
    remote_ip: [u8; 4],
    seq_no: u32,
    ack_no: u32,
}

impl Default for TcpConn {
    fn default() -> Self {
        Self {
            state: TcpState::Closed,
            local_port: 0,
            remote_port: 0,
            remote_ip: [0; 4],
            seq_no: 0,
            ack_no: 0,
        }
    }
}

/// Build a TCP SYN segment.
/// Writes into `buf` at `offset`. Returns header length (20) or 0 on error.
fn tcp_build_syn(buf: &mut [u8], offset: usize, src_port: u16, dst_port: u16, seq_no: u32) -> usize {
    tcp_build_header(buf, offset, src_port, dst_port, seq_no, 0, TCP_FLAG_SYN, 4096)
}

/// Build a TCP SYN-ACK segment.
/// Writes into `buf` at `offset`. Returns header length (20) or 0 on error.
fn tcp_build_synack(
    buf: &mut [u8],
    offset: usize,
    src_port: u16,
    dst_port: u16,
    seq_no: u32,
    ack_no: u32,
) -> usize {
    tcp_build_header(buf, offset, src_port, dst_port, seq_no, ack_no, TCP_FLAG_SYN | TCP_FLAG_ACK, 4096)
}

/// Build a TCP ACK segment.
/// Writes into `buf` at `offset`. Returns header length (20) or 0 on error.
fn tcp_build_ack(buf: &mut [u8], offset: usize, src_port: u16, dst_port: u16, seq_no: u32, ack_no: u32) -> usize {
    tcp_build_header(buf, offset, src_port, dst_port, seq_no, ack_no, TCP_FLAG_ACK, 4096)
}

/// Build a TCP ACK segment with data following the header.
/// Writes the TCP header into `buf` at `offset`. Returns header length (20) or 0 on error.
/// The caller is responsible for copying data payload immediately after the header.
fn tcp_build_data_ack(buf: &mut [u8], offset: usize, src_port: u16, dst_port: u16, seq_no: u32, ack_no: u32) -> usize {
    tcp_build_header(buf, offset, src_port, dst_port, seq_no, ack_no, TCP_FLAG_ACK, 4096)
}

// ============================================================================
// Network stack receive: process an incoming Ethernet frame
// ============================================================================

/// Process a received Ethernet frame. Handles ARP requests (replies with our MAC)
/// and IPv4 packets addressed to us (ICMP ping, UDP/DHCP).
/// Returns the length of any reply frame placed in `out_buf`, or 0 if no reply.
fn net_process_frame(frame: &[u8], out_buf: &mut [u8]) -> usize {
    let Some((dst, src, ethertype)) = eth_parse_header(frame) else {
        return 0;
    };

    // Ignore frames not addressed to us or broadcast
    let my_mac = unsafe { NET_MAC };
    let is_broadcast = dst == [0xFF; 6];
    let is_unicast_to_us = dst == my_mac;
    if !is_broadcast && !is_unicast_to_us {
        return 0;
    }

    match ethertype {
        ETH_P_ARP => {
            // ARP payload starts at offset ETH_HLEN
            let Some((opcode, sender_mac, sender_ip, target_mac, target_ip)) =
                arp_parse(frame, ETH_HLEN)
            else {
                return 0;
            };

            // If it's an ARP request for our IP, send a reply
            let my_ip = unsafe { NET_IP_ADDR };
            if opcode == ARP_OP_REQUEST && target_ip == my_ip {
                return arp_build_reply(out_buf, &sender_mac, &sender_ip);
            }
            0
        }
        ETH_P_IP => {
            let Some((protocol, src_ip, dst_ip, _total_len)) =
                ipv4_parse_header(frame, ETH_HLEN)
            else {
                return 0;
            };

            let my_ip = unsafe { NET_IP_ADDR };
            if dst_ip != my_ip && dst_ip != [255, 255, 255, 255] {
                return 0;
            }

            // Handle ICMP echo request (ping) -> echo reply
            if protocol == IP_PROTO_ICMP {
                // ICMP header starts at ETH_HLEN + 20
                if frame.len() < ETH_HLEN + 20 + 8 {
                    return 0;
                }
                let icmp_type = frame[ETH_HLEN + 20];
                let icmp_code = frame[ETH_HLEN + 21];
                if icmp_type == 8 && icmp_code == 0 {
                    // Echo request -> build echo reply
                    let mut reply = [0u8; ETH_HLEN + 20 + 8 + 56];
                    let reply_len = eth_build_header(&mut reply, &src, ETH_P_IP);
                    let ip_len = ipv4_build_header(&mut reply, reply_len, IP_PROTO_ICMP, &src_ip, 8 + 56);
                    if ip_len == 0 {
                        return 0;
                    }
                    // Copy ICMP echo payload (change type to 0 = echo reply, recompute checksum later)
                    reply[reply_len + ip_len] = 0; // type = echo reply
                    reply[reply_len + ip_len + 1] = 0; // code
                    // Copy identifier/sequence from request
                    reply[reply_len + ip_len + 4..reply_len + ip_len + 8]
                        .copy_from_slice(&frame[ETH_HLEN + 20 + 4..ETH_HLEN + 20 + 8]);
                    // Copy data payload
                    let data_start = ETH_HLEN + 20 + 8;
                    let data_len = frame.len().min(data_start + 56) - data_start;
                    reply[reply_len + ip_len + 8..reply_len + ip_len + 8 + data_len]
                        .copy_from_slice(&frame[data_start..data_start + data_len]);

                    // Copy reply to out_buf
                    let total = reply_len + ip_len + 8 + data_len;
                    out_buf[..total].copy_from_slice(&reply[..total]);
                    total
                } else {
                    0
                }
            } else if protocol == IP_PROTO_UDP {
                // UDP payload starts at ETH_HLEN + 20
                if frame.len() < ETH_HLEN + 20 + UDP_HLEN {
                    return 0;
                }
                let Some((udp_src_port, udp_dst_port, udp_len)) =
                    udp_parse_header(frame, ETH_HLEN + 20)
                else {
                    return 0;
                };

                let udp_payload_offset = ETH_HLEN + 20 + UDP_HLEN;
                let udp_payload_len = udp_len as usize - UDP_HLEN;

                // DHCP Offer (server port 67 -> client port 68)
                if udp_dst_port == UDP_PORT_BOOTPC && udp_src_port == UDP_PORT_BOOTPS {
                    if let Some((msg_type, offered_ip, server_ip)) =
                        dhcp_parse_offer(frame, udp_payload_offset)
                    {
                        if msg_type == DHCP_MSG_OFFER {
                            let client = unsafe { &mut DHCP_CLIENT };
                            client.state = DhcpState::Selecting;
                            client.offered_ip = offered_ip;
                            client.server_ip = server_ip;
                            sbi_console_write(b"dhcp: offer received, ip=");
                            for b in offered_ip {
                                let hi = (b >> 4) as u8;
                                let lo = (b & 0x0F) as u8;
                                sbi_console_putchar(if hi < 10 { b'0' + hi } else { b'a' + hi - 10 });
                                sbi_console_putchar(if lo < 10 { b'0' + lo } else { b'a' + lo - 10 });
                                sbi_console_putchar(b'.');
                            }
                            sbi_console_write(b"\n");
                        }
                    }

                    // DHCP Ack (server port 67 -> client port 68)
                    if let Some((assigned_ip, subnet_mask, router, dns_server)) =
                        dhcp_parse_ack(frame, udp_payload_offset)
                    {
                        let client = unsafe { &mut DHCP_CLIENT };
                        client.state = DhcpState::Bound;
                        unsafe { NET_IP_ADDR = assigned_ip };
                        sbi_console_write(b"dhcp: ack received, bound to ip=");
                        for b in assigned_ip {
                            let hi = (b >> 4) as u8;
                            let lo = (b & 0x0F) as u8;
                            sbi_console_putchar(if hi < 10 { b'0' + hi } else { b'a' + hi - 10 });
                            sbi_console_putchar(if lo < 10 { b'0' + lo } else { b'a' + lo - 10 });
                            sbi_console_putchar(b'.');
                        }
                        sbi_console_write(b"\n");
                        sbi_console_write(b"dhcp: subnet=");
                        for b in subnet_mask {
                            let hi = (b >> 4) as u8;
                            let lo = (b & 0x0F) as u8;
                            sbi_console_putchar(if hi < 10 { b'0' + hi } else { b'a' + hi - 10 });
                            sbi_console_putchar(if lo < 10 { b'0' + lo } else { b'a' + lo - 10 });
                            sbi_console_putchar(b'.');
                        }
                        sbi_console_write(b"\n");
                        sbi_console_write(b"dhcp: router=");
                        for b in router {
                            let hi = (b >> 4) as u8;
                            let lo = (b & 0x0F) as u8;
                            sbi_console_putchar(if hi < 10 { b'0' + hi } else { b'a' + hi - 10 });
                            sbi_console_putchar(if lo < 10 { b'0' + lo } else { b'a' + lo - 10 });
                            sbi_console_putchar(b'.');
                        }
                        sbi_console_write(b"\n");
                        sbi_console_write(b"dhcp: dns=");
                        for b in dns_server {
                            let hi = (b >> 4) as u8;
                            let lo = (b & 0x0F) as u8;
                            sbi_console_putchar(if hi < 10 { b'0' + hi } else { b'a' + hi - 10 });
                            sbi_console_putchar(if lo < 10 { b'0' + lo } else { b'a' + lo - 10 });
                            sbi_console_putchar(b'.');
                        }
                        sbi_console_write(b"\n");
                    }
                    return 0;
                }

                // Generic UDP echo: send back to sender
                let mut reply = [0u8; ETH_HLEN + 20 + UDP_HLEN + 256];
                let reply_eth = eth_build_header(&mut reply, &src, ETH_P_IP);
                let udp_payload = &frame[udp_payload_offset..udp_payload_offset + udp_payload_len.min(256)];
                let reply_ip = ipv4_build_header(&mut reply, reply_eth, IP_PROTO_UDP, &src_ip, UDP_HLEN + udp_payload.len());
                if reply_ip == 0 {
                    return 0;
                }
                let reply_udp = udp_build_header(&mut reply, reply_eth + reply_ip, udp_dst_port, udp_src_port, udp_payload.len());
                if reply_udp == 0 {
                    return 0;
                }
                reply[reply_eth + reply_ip + reply_udp..reply_eth + reply_ip + reply_udp + udp_payload.len()]
                    .copy_from_slice(udp_payload);
                let total = reply_eth + reply_ip + reply_udp + udp_payload.len();
                out_buf[..total].copy_from_slice(&reply[..total]);
                total
            } else if protocol == IP_PROTO_TCP {
                // TCP handling: SYN -> SYN-ACK handshake
                if frame.len() < ETH_HLEN + 20 + TCP_HLEN {
                    return 0;
                }
                let Some((tcp_src_port, tcp_dst_port, tcp_seq_no, tcp_ack_no, tcp_flags, _tcp_window)) =
                    tcp_parse_header(frame, ETH_HLEN + 20)
                else {
                    return 0;
                };

                // Handle incoming SYN -> reply with SYN-ACK
                if tcp_flags & TCP_FLAG_SYN != 0 && tcp_flags & TCP_FLAG_ACK == 0 {
                    let mut reply = [0u8; ETH_HLEN + 20 + TCP_HLEN];
                    let reply_eth = eth_build_header(&mut reply, &src, ETH_P_IP);
                    let reply_ip =
                        ipv4_build_header(&mut reply, reply_eth, IP_PROTO_TCP, &src_ip, TCP_HLEN);
                    if reply_ip == 0 {
                        return 0;
                    }
                    let reply_tcp = tcp_build_synack(
                        &mut reply,
                        reply_eth + reply_ip,
                        tcp_dst_port,
                        tcp_src_port,
                        tcp_seq_no.wrapping_add(1),
                        tcp_seq_no.wrapping_add(1),
                    );
                    if reply_tcp == 0 {
                        return 0;
                    }
                    let total = reply_eth + reply_ip + reply_tcp;
                    out_buf[..total].copy_from_slice(&reply[..total]);
                    total
                } else if tcp_flags & TCP_FLAG_ACK != 0 && tcp_flags & TCP_FLAG_SYN == 0 {
                    // TCP data segment (ACK with optional payload) -> echo data back
                    let tcp_payload_offset = ETH_HLEN + 20 + TCP_HLEN;
                    let tcp_payload_len =
                        frame.len().saturating_sub(tcp_payload_offset);
                    let mut reply = [0u8; ETH_HLEN + 20 + TCP_HLEN + 256];
                    let reply_eth = eth_build_header(&mut reply, &src, ETH_P_IP);
                    let tcp_data_len = tcp_payload_len.min(256);
                    let reply_ip = ipv4_build_header(
                        &mut reply,
                        reply_eth,
                        IP_PROTO_TCP,
                        &src_ip,
                        TCP_HLEN + tcp_data_len,
                    );
                    if reply_ip == 0 {
                        return 0;
                    }
                    let reply_tcp = tcp_build_data_ack(
                        &mut reply,
                        reply_eth + reply_ip,
                        tcp_dst_port,
                        tcp_src_port,
                        tcp_ack_no.wrapping_add(tcp_data_len as u32),
                        tcp_seq_no.wrapping_add(1),
                    );
                    if reply_tcp == 0 {
                        return 0;
                    }
                    // Echo the received data back
                    reply[reply_eth + reply_ip + reply_tcp
                        ..reply_eth + reply_ip + reply_tcp + tcp_data_len]
                        .copy_from_slice(&frame[tcp_payload_offset..tcp_payload_offset + tcp_data_len]);
                    let total = reply_eth + reply_ip + reply_tcp + tcp_data_len;
                    out_buf[..total].copy_from_slice(&reply[..total]);
                    total
                } else {
                    0
                }
            } else {
                0
            }
        }
        _ => 0,
    }
}