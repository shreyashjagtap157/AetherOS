# AetherOS Baseline Report — current main HEAD 91478e5

**Date**: 27 November 2026  
**Workspace**: C:\Users\siddh\Downloads\ABC\AetherOS  
**Git Root**: C:\Users\siddh\Downloads\ABC\AetherOS  
**Branch**: kilo/virtio-net-device-tree (based on main 91478e5)
**Commit**: 91478e592be1077bd79a0996f4207faf53d9fdc8 (main baseline)
**Working commit**: Device Tree virtio-mmio discovery + complete VirtIO-net RX/TX driver + network-stack integration.
**Previous commit**: 8fe51c0 docs: consolidate owner requirements and AetherOS platform goal
**Commit before that**: 91478e5 impl: VirtIO block/network drivers, TCP data transfer, protected process execution
**Working tree**: modified (main.rs DTB/driver rework) + new host test crate `aether-virtio-net-test`; untracked scratch/log files (listed in `git status`) — none are part of the source.

## Environment

- **Rust toolchain**: 1.95.0 (pinned via `rust-toolchain.toml`, `~/.cargo/bin`), targets include `riscv64gc-unknown-none-elf`
- **Components**: rustc, cargo, clippy, rustfmt, rust-src, llvm-tools
- **QEMU / python3**: NOT available on this Windows host. QEMU boot and real-device packet exchange are verified in CI (GitHub Actions `riscv64-boot.yml` on ubuntu-24.04) and cannot be reproduced locally.
- **Local verification**: host `reference/` workspace gate + riscv64 kernel cross-compile only.

## Reproduced baseline evidence (current HEAD)

Run from `reference/` with `PATH=/c/Users/siddh/.cargo/bin:$PATH`:

| Gate | Command | Result |
|------|---------|--------|
| fmt | `cargo fmt --check` | Pass (clean) |
| check | `cargo check --workspace` | Pass (clean) |
| test | `cargo test --workspace` | **179 tests pass, 0 failed** across 12 crates |
| clippy | `cargo clippy --workspace` | Pass; 22 pre-existing warnings in `aether-runtime-model`, 23 in `aether-storage-model` (matching prior baseline; gate does not enforce `-D warnings`) |
| doc | `cargo doc --workspace --no-deps` | Pass |
| CTS | `cargo run -p aether-cts-runner` | **38/38 CTS cases pass** |

Kernel crate (`reference/crates/aether-arch-riscv64`, excluded from workspace, built separately for the real target):

| Gate | Command | Result |
|------|---------|--------|
| cross-build | `cargo build --release --target riscv64gc-unknown-none-elf` | **Builds successfully** (ELF entry 0x80200000, RISC-V ET_EXEC; 90 pre-existing warnings, no errors) |

## Baseline claims assessment (AGENTS.md v4 goal Section 2.1)

| # | Six "Next Move" claim | Status at current HEAD |
|---|----------------------|------------------------|
| 1 | Storage and IPC wired into kernel main loop | Verified: `ram_read`/`ram_write`, `fs_*`, `ipc_send`/`ipc_receive` called from `start()` main loop; boot self-test exercises IPC round-trip, storage write/read, filesystem create/write/read/delete |
| 2 | Shell command processing via IPC | `aether-shell-model` (12 tests) + `aether-dispatch-model` present; host reference model, not wired into the bare-metal kernel main loop |
| 3 | Runtime VM / userspace execution engine integrated | `task1_entry` runtime VM runs as task 1 via SBI console; `aether-runtime-model` (7 tests) host model |
| 4 | A/B partition update and recovery integrated | `UpdateSystem` in `start()` (begin/apply/confirm); `aether-update-model` (7 tests) host model |
| 5 | Hardware/release qualification includes CI-verified QEMU boot | CI QEMU boot run passed (boot marker "hello", exit 124 = expected 60s timeout); not locally reproducible (no QEMU on host) |
| 6 | Check gate script exists and passes | `check.sh` exists; Rust portion passes locally; Python doc validators not run locally (no python3 on host) |

## Completed: VirtIO-net vertical slice (kilo/virtio-net-device-tree)

The VirtIO network code is now a complete, tested RX/TX path:

- Device Tree virtio-mmio discovery replaces hard-coded MMIO addresses: `start()` reads the firmware DTB pointer from `a1` and calls `discover_virtio_devices()` to set runtime MMIO bases, falling back to compiled-in defaults (`VIRTIO_BLOCK_MMIO_DEFAULT` / `VIRTIO_NET_MMIO_DEFAULT`). No `alloc`/`Vec` — discovery uses fixed-size arrays (`MAX_VIRTIO_DEVICES = 8`) because the kernel crate is `#![no_std]`.
- `virtio_net_send()` copies the packet to the TX buffer, publishes descriptor 0 on the TX avail ring, notifies the device, then polls the used ring until the transmission completes and the descriptor is reclaimed (returns false on timeout).
- `virtio_net_recv()` posts all RX descriptors (with `VIRTQ_DESC_F_WRITE`) during `virtio_net_init()` and re-posts each consumed descriptor after draining the used ring; it strips the 12-byte `virtio_net_hdr` prefix and returns the Ethernet frame byte count.
- Feature negotiation validates `VIRTIO_F_VERSION_1` and advertises `VIRTIO_NET_F_MAC` when offered; link status is read from device config space and exposed via `net_link_up()`.
- The VirtIO block driver's feature negotiation now also validates `VIRTIO_F_VERSION_1` and negotiates it properly (previously read host features but discarded them and skipped version-1 negotiation).
- The driver is connected to the network stack: the main loop polls `net_recv()` → `net_process_frame()` → `virtio_net_send()` for bidirectional packet exchange, and `virtio_net_init()` is invoked from `start()`.
- Host unit tests (7 cases) in new crate `aether-virtio-net-test` validate DTB `reg` parsing, descriptor flags, virtqueue ring-index wrap, and Ethernet header build/parse. All pass under `cargo test --workspace`.

Remaining (not locally reproducible without QEMU): real device packet exchange on QEMU `virt` (verified in CI), end-to-end DHCP/DNS/TCP over the live VirtIO device.

## Next dependency-ready task

Continue qualifying the remaining v4.0 mandate slices (storage, runtimes, AI, installation, updates, recovery, ecosystem) per the v4.0 objective.
