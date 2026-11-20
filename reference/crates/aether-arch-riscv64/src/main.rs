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

fn csrw_satp(val: u64) {
    unsafe {
        core::arch::asm!("csrw satp, {}", in(reg) val, options(nomem));
    }
}

unsafe extern "C" {
    fn trap_entry();
}

core::arch::global_asm!(
    ".section .text",
    ".globl trap_entry",
    "trap_entry:",
    "  mret",
);

#[unsafe(no_mangle)]
pub extern "C" fn trap_handler(mcause: u64, sepc: u64) -> u64 {
    sepc
}

#[unsafe(no_mangle)]
pub extern "C" fn start() -> ! {
    csrw_stvec(trap_entry as *const () as usize as u64);
    csrs_sie(SIE_STIE);
    csrs_sstatus(SSTATUS_SIE);
    sbi_console_write(b"stvec+sie+sstatus set\n");
    sbi_console_write(b"hello from kernel\n");

    // Set up Sv39 address space: identity-map [0, 0x8240_0000).
    let root = alloc_frame();
    sv39_map(root, 0, 0, FRAME_ALLOC_BASE + FRAME_ALLOC_SIZE, PTE_R | PTE_W | PTE_X);
    let satp = (SATP_MODE_SV39 << 60) | ppn_of(root);
    unsafe {
        core::arch::asm!("csrw satp, {}", in(reg) satp, options(nomem));
        core::arch::asm!("sfence.vma", options(nomem));
    }
    sbi_console_write(b"satp enabled\n");

    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}

// Page frame allocator
const FRAME_ALLOC_BASE: usize = 0x8040_0000;
const FRAME_ALLOC_SIZE: usize = 32 * 1024 * 1024;
static mut FRAME_ALLOC_NEXT: usize = FRAME_ALLOC_BASE;

fn alloc_frame() -> usize {
    unsafe {
        let addr = FRAME_ALLOC_NEXT;
        FRAME_ALLOC_NEXT = FRAME_ALLOC_NEXT.checked_add(PAGE_SIZE)
            .expect("frame allocator exhausted");
        let mut p = addr as *mut u64;
        for _ in 0..PAGE_SIZE / 8 {
            p.write_volatile(0);
            p = p.add(1);
        }
        addr
    }
}

fn ppn_of(addr: usize) -> u64 {
    (addr >> 12) as u64
}

const PAGE_SIZE: usize = 4096;
const PTE_V: u64 = 1 << 0;
const PTE_R: u64 = 1 << 1;
const PTE_W: u64 = 1 << 2;
const PTE_X: u64 = 1 << 3;
const SATP_MODE_SV39: u64 = 8;

fn sv39_map(root_pa: usize, vaddr: usize, paddr: usize, size: usize, perm: u64) {
    assert_eq!(vaddr % PAGE_SIZE, 0);
    assert_eq!(paddr % PAGE_SIZE, 0);
    assert_eq!(size % PAGE_SIZE, 0);

    let mut table = root_pa as *mut u64;

    for offset in (0..size).step_by(PAGE_SIZE) {
        let va = vaddr + offset;
        let pa = paddr + offset;

        let l2 = ((va >> 30) & 0x1ff) as usize;
        let mut l2_entry = unsafe { table.add(l2).read_volatile() };
        if l2_entry & PTE_V == 0 {
            let child = alloc_frame();
            l2_entry = (ppn_of(child) << 10) | PTE_V;
            unsafe { table.add(l2).write_volatile(l2_entry) };
        }
        let l2_table = ((l2_entry >> 10) << 12) as *mut u64;

        let l1 = ((va >> 21) & 0x1ff) as usize;
        let mut l1_entry = unsafe { l2_table.add(l1).read_volatile() };
        if l1_entry & PTE_V == 0 {
            let child = alloc_frame();
            l1_entry = (ppn_of(child) << 10) | PTE_V;
            unsafe { l2_table.add(l1).write_volatile(l1_entry) };
        }
        let l1_table = ((l1_entry >> 10) << 12) as *mut u64;

        let l0 = ((va >> 12) & 0x1ff) as usize;
        let pte = (ppn_of(pa) << 10) | PTE_V | perm;
        unsafe { l1_table.add(l0).write_volatile(pte) };
    }
}