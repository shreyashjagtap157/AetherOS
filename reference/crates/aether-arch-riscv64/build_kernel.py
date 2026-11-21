content = '''//! Minimal RISC-V64 kernel with trap handling, timer interrupts,
//! 2-task context switching, IPC message queue, and storage.
//!
//! Build:   cargo build --target riscv64gc-unknown-none-elf
//! Run:     qemu-system-riscv64 -machine virt -bios default \
//!            -kernel target/riscv64gc-unknown-none-elf/debug/aether-arch-riscv64 -nographic

#![no_std]
#![no_main]

use core::panic::PanicInfo;

const SBI_EXT_CONSOLE_PUTCHAR: u64 = 0x01;
const SBI_CONSOLE_PUTCHAR: u64 = 0x00;
const SBI_EXT_TIMER: u64 = 0x00;
const SBI_TIMER_SET_TIMER: u64 = 0x00;

const SSTATUS_SIE: u64 = 1 << 1;
const SIE_STIE: u64 = 1 << 5;

const SCAUSE_SUPERVISOR_TIMER: u64 = 5;

const CLINT_MTIME: usize = 0x0200_BFF8;
const CLINT_MTIMECMP0: usize = 0x0200_4000;
const TIMER_INTERVAL: u64 = 10_000_000;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
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
    "  sd s4, 152(sp)",
    "  sd s5, 160(sp)",
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
pub extern "C" fn trap_handler(mcause: u64, sepc: u64) -> u64 {
    if mcause == SCAUSE_SUPERVISOR_TIMER {
        sbi_console_write(b"tick\\n");
        let now = clint_read_mtime();
        clint_write_mtimecmp0(now.wrapping_add(TIMER_INTERVAL));
        unsafe { save_and_switch(sepc) };
    }
    sepc
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
    }

    task.sp = sp.wrapping_add(256);

    let next = (current + 1) % MAX_TASKS;

    unsafe {
        CURRENT_TASK = next;
        let next_task = &TASKS[next];

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
        core::arch::asm!("mv sp, {}", in(reg) next_sp, options(nomem));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn task1_entry() -> ! {
    sbi_console_write(b"task1 entry\\n");
    let mut counter: u64 = 0;
    loop {
        let bytes = counter.to_le_bytes();
        ipc_send(&bytes);
        counter = counter.wrapping_add(1);
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

fn ram_read(sector: u64, buf: &mut [u8]) -> bool {
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

static mut TASK1_STACK: [u64; 128] = [0; 128];

#[unsafe(no_mangle)]
pub extern "C" fn start() -> ! {
    csrw_stvec(trap_entry as *const () as usize as u64);
    csrs_sie(SIE_STIE);
    csrs_sstatus(SSTATUS_SIE);
    sbi_console_write(b"stvec+sie+sstatus set\\n");
    sbi_console_write(b"hello from kernel\\n");

    unsafe {
        TASKS[1].pc = task1_entry as *const () as usize as u64;
        TASKS[1].sp = (&raw const TASK1_STACK as *const [u64; 128] as usize + 1024) as u64;
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
    sbi_console_write(b"\\n");

    loop {
        if let Some(msg) = ipc_receive() {
            sbi_console_write(b"recv: ");
            for &b in &msg {
                if b == 0 {
                    break;
                }
                sbi_console_putchar(b);
            }
            sbi_console_write(b"\\n");
        }
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}
'''

open('src/main.rs', 'w').write(content)
print('Done')