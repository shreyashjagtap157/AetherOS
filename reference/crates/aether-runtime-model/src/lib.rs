//! Runtime model (host reference): minimal userspace execution engine.
//!
//! This is a bounded executable host model of the AetherOS runtime
//! subsystem. It provides a simple virtual machine that loads and
//! executes a minimal instruction set with basic syscalls (print, exit,
//! read, write). It is deliberately NOT a production runtime, JIT,
//! or sandbox.
//!
//! Scope:
//! - 8 general-purpose registers + pc + sp
//! - 16 instructions: NOP, MOV, ADD, SUB, LOAD, STORE, JMP, JZ, JNZ,
//!   CALL, RET, SYSCALL (print/exit/read/write), HALT
//! - Flat memory (64 KiB)
//! - Single process, no preemption, no protection
//!
//! Out of scope:
//! - ELF loader, dynamic linking, shared libraries.
//! - Memory protection, MMU, sandboxing.
//! - Multi-process, preemption, signals.
//! - File descriptors, pipes, sockets.

#![allow(missing_docs)]
#![allow(clippy::all)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use std::collections::VecDeque;

/// Number of general-purpose registers.
const NUM_REGS: usize = 8;

/// Memory size in bytes.
const MEM_SIZE: usize = 64 * 1024;

/// Syscall numbers.
const SYS_PRINT: u64 = 1;
const SYS_EXIT: u64 = 2;
const SYS_READ: u64 = 3;
const SYS_WRITE: u64 = 4;

/// Instruction opcodes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Op {
    Nop = 0,
    Mov = 1,      // MOV rd, rs (or MOV rd, imm)
    Add = 2,      // ADD rd, rs
    Sub = 3,      // SUB rd, rs
    Load = 4,     // LOAD rd, [addr]
    Store = 5,    // STORE [addr], rs
    Jmp = 6,      // JMP addr
    Jz = 7,       // JZ rd, addr
    Jnz = 8,      // JNZ rd, addr
    Call = 9,     // CALL addr
    Ret = 10,     // RET
    Syscall = 11, // SYSCALL num, arg
    Halt = 12,    // HALT
}

/// An instruction: opcode + up to 3 operands.
#[derive(Clone, Copy, Debug)]
pub struct Inst {
    pub op: Op,
    pub a: u64,
    pub b: u64,
    pub c: u64,
}

impl Inst {
    pub fn new(op: Op, a: u64, b: u64, c: u64) -> Self {
        Self { op, a, b, c }
    }
}

/// Runtime error types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    /// Program exited with code.
    Exit(u64),
    /// Memory access out of bounds.
    MemoryFault,
    /// Invalid instruction.
    InvalidInst,
    /// Stack overflow.
    StackOverflow,
    /// Stack underflow.
    StackUnderflow,
    /// Unhandled syscall.
    UnhandledSyscall,
}

/// Process state.
#[derive(Debug)]
pub struct Process {
    /// General-purpose registers.
    pub regs: [u64; NUM_REGS],
    /// Program counter.
    pub pc: usize,
    /// Stack pointer.
    pub sp: usize,
    /// Memory.
    pub mem: Vec<u8>,
    /// Call stack (return addresses).
    pub call_stack: Vec<usize>,
    /// Standard output capture.
    pub stdout: VecDeque<u8>,
    /// Standard input buffer.
    pub stdin: VecDeque<u8>,
    /// Whether the process has exited.
    pub exited: bool,
    /// Exit code.
    pub exit_code: u64,
}

impl Process {
    /// Creates a new process with the given program loaded at address 0.
    pub fn new(program: &[Inst]) -> Self {
        let mut mem = vec![0u8; MEM_SIZE];
        // Encode program into memory (each instruction = 4 u64 words).
        for (i, inst) in program.iter().enumerate() {
            let offset = i * 4 * 8;
            if offset + 32 > MEM_SIZE {
                break;
            }
            let words = [inst.op as u64, inst.a, inst.b, inst.c];
            for (j, w) in words.iter().enumerate() {
                let bytes = w.to_le_bytes();
                mem[offset + j * 8..offset + j * 8 + 8].copy_from_slice(&bytes);
            }
        }
        Self {
            regs: [0; NUM_REGS],
            pc: 0,
            sp: MEM_SIZE - 1,
            mem,
            call_stack: Vec::new(),
            stdout: VecDeque::new(),
            stdin: VecDeque::new(),
            exited: false,
            exit_code: 0,
        }
    }

    /// Reads a u64 from memory at the given address.
    fn read_u64(&self, addr: usize) -> Result<u64, RuntimeError> {
        if addr + 8 > MEM_SIZE {
            return Err(RuntimeError::MemoryFault);
        }
        let bytes = &self.mem[addr..addr + 8];
        Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
    }

    /// Writes a u64 to memory at the given address.
    fn write_u64(&mut self, addr: usize, val: u64) -> Result<(), RuntimeError> {
        if addr + 8 > MEM_SIZE {
            return Err(RuntimeError::MemoryFault);
        }
        let bytes = val.to_le_bytes();
        self.mem[addr..addr + 8].copy_from_slice(&bytes);
        Ok(())
    }

    /// Reads one instruction at the current PC.
    fn fetch(&self) -> Result<Inst, RuntimeError> {
        let pc = self.pc;
        if pc + 32 > MEM_SIZE {
            return Err(RuntimeError::MemoryFault);
        }
        let op = self.read_u64(pc)?;
        let a = self.read_u64(pc + 8)?;
        let b = self.read_u64(pc + 16)?;
        let c = self.read_u64(pc + 24)?;
        let op = match op {
            0 => Op::Nop,
            1 => Op::Mov,
            2 => Op::Add,
            3 => Op::Sub,
            4 => Op::Load,
            5 => Op::Store,
            6 => Op::Jmp,
            7 => Op::Jz,
            8 => Op::Jnz,
            9 => Op::Call,
            10 => Op::Ret,
            11 => Op::Syscall,
            12 => Op::Halt,
            _ => return Err(RuntimeError::InvalidInst),
        };
        Ok(Inst { op, a, b, c })
    }

    /// Executes one instruction. Returns Ok(()) to continue, Err to stop.
    pub fn step(&mut self) -> Result<(), RuntimeError> {
        if self.exited {
            return Err(RuntimeError::Exit(self.exit_code));
        }
        let inst = self.fetch()?;
        match inst.op {
            Op::Nop => {
                self.pc += 32;
            }
            Op::Mov => {
                // MOV rd, imm (if b==1) or MOV rd, rs (if b==0)
                let rd = inst.a as usize;
                if rd >= NUM_REGS {
                    return Err(RuntimeError::InvalidInst);
                }
                if inst.b == 1 {
                    self.regs[rd] = inst.c;
                } else {
                    let rs = inst.c as usize;
                    if rs >= NUM_REGS {
                        return Err(RuntimeError::InvalidInst);
                    }
                    self.regs[rd] = self.regs[rs];
                }
                self.pc += 32;
            }
            Op::Add => {
                let rd = inst.a as usize;
                let rs = inst.b as usize;
                if rd >= NUM_REGS || rs >= NUM_REGS {
                    return Err(RuntimeError::InvalidInst);
                }
                self.regs[rd] = self.regs[rd].wrapping_add(self.regs[rs]);
                self.pc += 32;
            }
            Op::Sub => {
                let rd = inst.a as usize;
                let rs = inst.b as usize;
                if rd >= NUM_REGS || rs >= NUM_REGS {
                    return Err(RuntimeError::InvalidInst);
                }
                self.regs[rd] = self.regs[rd].wrapping_sub(self.regs[rs]);
                self.pc += 32;
            }
            Op::Load => {
                let rd = inst.a as usize;
                let addr = inst.b as usize;
                if rd >= NUM_REGS {
                    return Err(RuntimeError::InvalidInst);
                }
                self.regs[rd] = self.read_u64(addr)?;
                self.pc += 32;
            }
            Op::Store => {
                let addr = inst.a as usize;
                let rs = inst.b as usize;
                if rs >= NUM_REGS {
                    return Err(RuntimeError::InvalidInst);
                }
                self.write_u64(addr, self.regs[rs])?;
                self.pc += 32;
            }
            Op::Jmp => {
                self.pc = inst.a as usize;
            }
            Op::Jz => {
                let rd = inst.a as usize;
                let addr = inst.b as usize;
                if rd >= NUM_REGS {
                    return Err(RuntimeError::InvalidInst);
                }
                if self.regs[rd] == 0 {
                    self.pc = addr;
                } else {
                    self.pc += 32;
                }
            }
            Op::Jnz => {
                let rd = inst.a as usize;
                let addr = inst.b as usize;
                if rd >= NUM_REGS {
                    return Err(RuntimeError::InvalidInst);
                }
                if self.regs[rd] != 0 {
                    self.pc = addr;
                } else {
                    self.pc += 32;
                }
            }
            Op::Call => {
                self.call_stack.push(self.pc + 32);
                self.pc = inst.a as usize;
            }
            Op::Ret => {
                let ret_addr = self.call_stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                self.pc = ret_addr;
            }
            Op::Syscall => {
                let num = inst.a;
                match num {
                    SYS_PRINT => {
                        let reg = inst.b as usize;
                        if reg >= NUM_REGS {
                            return Err(RuntimeError::InvalidInst);
                        }
                        let val = self.regs[reg];
                        self.stdout.push_back((val & 0xFF) as u8);
                    }
                    SYS_EXIT => {
                        let code = inst.b;
                        self.exited = true;
                        self.exit_code = code;
                        return Err(RuntimeError::Exit(code));
                    }
                    SYS_READ => {
                        // Read one byte from stdin into reg.
                        let reg = inst.b as usize;
                        if reg >= NUM_REGS {
                            return Err(RuntimeError::InvalidInst);
                        }
                        if let Some(byte) = self.stdin.pop_front() {
                            self.regs[reg] = byte as u64;
                        } else {
                            self.regs[reg] = 0;
                        }
                    }
                    SYS_WRITE => {
                        let addr = inst.b as usize;
                        let len = inst.c as usize;
                        if addr + len > MEM_SIZE {
                            return Err(RuntimeError::MemoryFault);
                        }
                        for &b in &self.mem[addr..addr + len] {
                            self.stdout.push_back(b);
                        }
                    }
                    _ => return Err(RuntimeError::UnhandledSyscall),
                }
                self.pc += 32;
            }
            Op::Halt => {
                self.exited = true;
                self.exit_code = 0;
                return Err(RuntimeError::Exit(0));
            }
        }
        Ok(())
    }

    /// Runs until exit or error. Returns the exit code.
    pub fn run(&mut self) -> Result<u64, RuntimeError> {
        loop {
            match self.step() {
                Ok(()) => {}
                Err(RuntimeError::Exit(code)) => return Ok(code),
                Err(e) => return Err(e),
            }
        }
    }

    /// Returns captured stdout as a string (if valid UTF-8).
    pub fn stdout_string(&self) -> String {
        String::from_utf8_lossy(&self.stdout.iter().copied().collect::<Vec<_>>()).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nop_halt() {
        let prog = vec![Inst::new(Op::Halt, 0, 0, 0)];
        let mut p = Process::new(&prog);
        let code = p.run().unwrap();
        assert_eq!(code, 0);
    }

    #[test]
    fn mov_and_add() {
        // MOV r0, 5 (imm); MOV r1, 3 (imm); ADD r0, r1; HALT
        let prog = vec![
            Inst::new(Op::Mov, 0, 1, 5),
            Inst::new(Op::Mov, 1, 1, 3),
            Inst::new(Op::Add, 0, 1, 0),
            Inst::new(Op::Halt, 0, 0, 0),
        ];
        let mut p = Process::new(&prog);
        p.run().unwrap();
        assert_eq!(p.regs[0], 8);
    }

    #[test]
    fn syscall_print() {
        // MOV r0, 65 (imm 'A'); SYSCALL print r0; HALT
        let prog = vec![
            Inst::new(Op::Mov, 0, 1, 65),
            Inst::new(Op::Syscall, SYS_PRINT, 0, 0),
            Inst::new(Op::Halt, 0, 0, 0),
        ];
        let mut p = Process::new(&prog);
        p.run().unwrap();
        assert_eq!(p.stdout_string(), "A");
    }

    #[test]
    fn syscall_exit_code() {
        let prog = vec![Inst::new(Op::Syscall, SYS_EXIT, 42, 0)];
        let mut p = Process::new(&prog);
        let code = p.run().unwrap();
        assert_eq!(code, 42);
    }

    #[test]
    fn jmp_and_jz() {
        // MOV r0, 0 (imm); JZ r0, 3 (skip next); MOV r1, 1 (imm); HALT
        // Actually: index 0: MOV r0,0; index 1: JZ r0, 3; index 2: MOV r1,1 (skipped); index 3: HALT
        let prog = vec![
            Inst::new(Op::Mov, 0, 1, 0),
            Inst::new(Op::Jz, 0, 3 * 32, 0),
            Inst::new(Op::Mov, 1, 1, 1),
            Inst::new(Op::Halt, 0, 0, 0),
        ];
        let mut p = Process::new(&prog);
        p.run().unwrap();
        assert_eq!(p.regs[1], 0); // skipped
    }

    #[test]
    fn call_and_ret() {
        // CALL func (index 3); HALT; func: MOV r0, 7 (imm); RET
        let prog = vec![
            Inst::new(Op::Call, 3 * 32, 0, 0),
            Inst::new(Op::Halt, 0, 0, 0),
            Inst::new(Op::Nop, 0, 0, 0), // padding
            Inst::new(Op::Mov, 0, 1, 7),
            Inst::new(Op::Ret, 0, 0, 0),
        ];
        let mut p = Process::new(&prog);
        p.run().unwrap();
        assert_eq!(p.regs[0], 7);
    }

    #[test]
    fn load_store() {
        // Store 42 at addr 100, load into r0, HALT
        let prog = vec![
            Inst::new(Op::Mov, 1, 1, 42),
            Inst::new(Op::Store, 200, 1, 0),
            Inst::new(Op::Load, 0, 200, 0),
            Inst::new(Op::Halt, 0, 0, 0),
        ];
        let mut p = Process::new(&prog);
        p.run().unwrap();
        assert_eq!(p.regs[0], 42);
    }
}
