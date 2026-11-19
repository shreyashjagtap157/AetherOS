# Handoff: Phase B Step 1 — RISC-V64 Boot Checkpoint (build-verified, boot NOT verified)

Classification: Operational
Authoritative Source: HANDOFF-PHASE-B-RV64.md

**Date:** 2026-11-19
**Purpose:** Snapshot of the Phase B opening increment so a new session can resume.
**Classification:** Operational

---

## 1. Decisions recorded

- The user approved (decision rounds of 2026-11-19) the Phase B reference target:
  **QEMU `virt` RISC-V64 with OpenSBI**, per `docs/VERSIONED-PRODUCT-IMPLEMENTATION-PLAN.md`
  and `docs/IMPLEMENTATION-ROADMAP.md` Phase B.
- RFC-0042 and UASA remain **undecided**; no implementation work on either has started.

## 2. What was done

- New crate `reference/crates/aether-arch-riscv64/` (standalone, excluded from the host
  workspace): a minimal `#![no_std]`/`#![no_main]` payload `_start → rust_main` that
  writes `AetherOS RV64 boot checkpoint\n` via the OpenSBI SBI `console_putchar` ecall
  (ext `0x01`, sub `0x00`) and idle-loops.
- Link script `link.ld`: image origin `0x80200000` (OpenSBI payload load address on QEMU `virt`),
  64 KiB stack.
- Verified artifacts on this host (2026-11-19):
  - `cargo build --release --target riscv64gc-unknown-none-elf` succeeds;
    `rustup target list --installed` shows `riscv64gc-unknown-none-elf` present.
  - Output `reference/crates/aether-arch-riscv64/target/riscv64gc-unknown-none-elf/release/aether-arch-riscv64`:
    5248 bytes; ELF64; little endian; `e_type=EXEC(2)`; `e_machine=EM_RISCV(243)`;
    `e_entry=0x80200000`.
  - Host gate (`./check.sh`, root) re-verified after adding the crate: exit 0; evidence 153 files.
    The crate is excluded from the host workspace build; host gates are unaffected by it.

## 3. What was NOT done and cannot be done on this host

- **Boot has not been executed anywhere.** No QEMU system emulator exists on this host
  (`qemu-system-riscv64` / `qemu-system-x86_64` absent); the WSL distro present is Docker
  Desktop's `docker-desktop` (VERSION=2, STATE=Stopped) and the Docker daemon is not running.
  Installing QEMU or launching Docker/WSL configuration changes on this machine is a
  privileged host action that has NOT been authorized.
- Therefore this crate is a **cross-compile and link checkpoint only**. It proves the
  toolchain and payload framing, not boot-ability, trap handling, timer, UART output,
  OpenSBI handshake, or any runtime behavior.
- No boot test has been recorded; no Phase B milestone is claimed met.

## 4. How to verify the boot checkpoint (when a QEMU-capable host exists)

```sh
rustup target add riscv64gc-unknown-none-elf
cd reference/crates/aether-arch-riscv64
cargo build --release --target riscv64gc-unknown-none-elf
qemu-system-riscv64 -machine virt -m 128M -bios default \
  -kernel target/riscv64gc-unknown-none-elf/release/aether-arch-riscv64 -nographic
# expect: "AetherOS RV64 boot checkpoint" on the console, then idle loop
```

Record the QEMU version, OpenSBI version, machine type, and console transcript as the
qualification evidence for the first Phase B milestone. Boot output appearing for one
run is still not the milestone; reproducible boot + trap/page-table evidence is.

## 5. State and next actions

- RFC-0042 and UASA remain undecided; do not start either.
- Next increments (in roadmap order): OpenSBI/SBI probe at boot, timer interrupt +
  minimal executor, Sv39 identity + user page table, one isolated component, UART claim,
  virtio-block path, then a deterministic boot harness for CI.
- Boot evidence requires QEMU; until a QEMU-capable host is available or installation is
  authorized, Phase B runs are build-verified only. Nothing in this step should be cited
  as a boot result.
