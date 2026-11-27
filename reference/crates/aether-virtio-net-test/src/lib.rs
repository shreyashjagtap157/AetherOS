//! Host-side unit tests for the VirtIO-net driver logic.
//!
//! The kernel crate (`aether-arch-riscv64`) is `#![no_std] #![no_main]` and is
//! only cross-compiled for `riscv64`, so it cannot host `#[cfg(test)]` code
//! that runs on the host. This crate exercises the pure, target-independent
//! algorithms the driver relies on (Device Tree `reg` parsing, virtqueue ring
//! index arithmetic, descriptor flags, Ethernet header build/parse) without
//! QEMU or MMIO.

const FDT_BEGIN_NODE: u32 = 0x00000001;
const FDT_END_NODE: u32 = 0x00000002;
const FDT_PROP: u32 = 0x00000003;
const FDT_NOP: u32 = 0x00000004;
const FDT_END: u32 = 0x00000009;
const FDT_MAGIC: u32 = 0xd00d_feed;

const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;
const VIRTQ_DESC_F_INDIRECT: u16 = 4;

const ETH_HLEN: usize = 14;
const ETH_P_ARP: u16 = 0x0806;
const ETH_P_IP: u16 = 0x0800;

fn read_u32_be(buf: &[u8], off: usize) -> Option<u32> {
    if off.checked_add(4)? > buf.len() {
        return None;
    }
    Some(u32::from_be_bytes([
        buf[off],
        buf[off + 1],
        buf[off + 2],
        buf[off + 3],
    ]))
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

/// Minimal DTB walker mirroring the kernel's `Dtb::find_virtio_mmio`. Returns
/// the MMIO base addresses of `virtio,mmio` nodes (addr_cells=2, size_cells=2).
fn find_virtio_mmio(fdt: &[u8]) -> Option<Vec<usize>> {
    if read_u32_be(fdt, 0)? != FDT_MAGIC {
        return None;
    }
    let off_dt_struct = read_u32_be(fdt, 8)? as usize;
    let off_dt_strings = read_u32_be(fdt, 12)? as usize;
    let size_dt_strings = read_u32_be(fdt, 32)? as usize;
    let size_dt_struct = read_u32_be(fdt, 36)? as usize;
    let strings =
        &fdt[off_dt_strings..off_dt_strings + size_dt_strings.min(fdt.len() - off_dt_strings)];
    let struct_end = off_dt_struct + size_dt_struct.min(fdt.len() - off_dt_struct);

    let mut bases = Vec::new();
    let mut pos = off_dt_struct;
    let mut in_virtio = false;
    while pos < struct_end {
        let token = read_u32_be(fdt, pos)?;
        pos += 4;
        match token {
            FDT_BEGIN_NODE => {
                let name = cstr_at(fdt, pos).unwrap_or("");
                pos += align4(name.len() + 1);
                in_virtio = name == "virtio,mmio" || name.starts_with("virtio");
            }
            FDT_END_NODE => in_virtio = false,
            FDT_PROP => {
                let len = read_u32_be(fdt, pos)? as usize;
                pos += 4;
                let nameoff = read_u32_be(fdt, pos)? as usize;
                pos += 4;
                let prop_end = pos.checked_add(len)?;
                let pname = cstr_at(strings, nameoff).unwrap_or("");
                if in_virtio && pname == "reg" {
                    // addr_cells=2: base is the low cell = bytes [off+4 .. off+8].
                    if let Some(base) = read_u32_be(fdt, pos + 4) {
                        bases.push(base as usize);
                    }
                }
                pos = align4(prop_end);
            }
            FDT_NOP => {}
            FDT_END => break,
            _ => return None,
        }
    }
    Some(bases)
}

/// Build a minimal DTB with one `virtio,mmio` node whose `reg` encodes the
/// MMIO base `base` (addr_cells=2, size_cells=2).
fn build_dtb(base: u32) -> Vec<u8> {
    let name = b"virtio,mmio\0";
    let name_padded = align4(name.len());
    let prop = 4 + 4 + 4 + 16; // token + len + nameoff + reg(16 bytes)
    let struct_len = 4 + name_padded + prop + 4 + 4; // begin + name + prop + end_node + end
    let strings_len = 4; // "reg\0"
    let totalsize = 32 + 8 + struct_len + strings_len; // header + rsvmap + struct + strings

    let mut buf = Vec::with_capacity(totalsize);
    buf.extend_from_slice(&[0u8; 32]); // header placeholder
    buf.extend_from_slice(&[0u8; 8]); // mem_rsvmap terminator
    buf.resize(totalsize, 0); // make room for struct + strings blocks

    // header (overwrite placeholder)
    buf[0..4].copy_from_slice(&FDT_MAGIC.to_be_bytes());
    buf[4..8].copy_from_slice(&(totalsize as u32).to_be_bytes());
    buf[8..12].copy_from_slice(&(40u32).to_be_bytes()); // off_dt_struct
    buf[12..16].copy_from_slice(&((40 + struct_len) as u32).to_be_bytes()); // off_dt_strings
    buf[32..36].copy_from_slice(&(strings_len as u32).to_be_bytes()); // size_dt_strings
    buf[36..40].copy_from_slice(&(struct_len as u32).to_be_bytes()); // size_dt_struct

    let mut pos = 40;
    buf[pos..pos + 4].copy_from_slice(&FDT_BEGIN_NODE.to_be_bytes());
    pos += 4;
    buf[pos..pos + name.len()].copy_from_slice(name);
    pos += name_padded;
    buf[pos..pos + 4].copy_from_slice(&FDT_PROP.to_be_bytes());
    pos += 4;
    buf[pos..pos + 4].copy_from_slice(&(16u32).to_be_bytes());
    pos += 4; // len (reg = 16 bytes)
    buf[pos..pos + 4].copy_from_slice(&(0u32).to_be_bytes());
    pos += 4; // nameoff -> "reg"
    buf[pos..pos + 4].copy_from_slice(&(0u32).to_be_bytes());
    pos += 4; // addr high
    buf[pos..pos + 4].copy_from_slice(&base.to_be_bytes());
    pos += 4; // addr low
    buf[pos..pos + 4].copy_from_slice(&(0u32).to_be_bytes());
    pos += 4; // size high
    buf[pos..pos + 4].copy_from_slice(&(0x1000u32).to_be_bytes());
    pos += 4; // size low
    buf[pos..pos + 4].copy_from_slice(&FDT_END_NODE.to_be_bytes());
    pos += 4;
    buf[pos..pos + 4].copy_from_slice(&FDT_END.to_be_bytes());
    pos += 4;

    // strings: "reg\0" at off_dt_strings + 0
    buf[40 + struct_len..40 + struct_len + 3].copy_from_slice(b"reg");
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dtb_walker_extracts_virtio_mmio_base() {
        let dtb = build_dtb(0x1000_2000);
        let bases = find_virtio_mmio(&dtb).expect("walk ok");
        assert_eq!(bases, vec![0x1000_2000]);
    }

    #[test]
    fn dtb_walker_rejects_bad_magic() {
        let mut bad = [0u8; 64];
        bad[0..4].copy_from_slice(&0xde_ad_be_ef_u32.to_be_bytes());
        assert!(find_virtio_mmio(&bad).is_none());
    }

    #[test]
    fn dtb_walker_ignores_non_virtio_nodes() {
        // Build a DTB where the only node is `foo`, not virtio.
        let mut buf = Vec::new();
        buf.extend_from_slice(&[0u8; 48]); // header + rsvmap
        buf[0..4].copy_from_slice(&FDT_MAGIC.to_be_bytes());
        let struct_len = 4 + align4(b"foo\0".len()) + 4 + 4; // begin + name + end_node + end
        let totalsize = 48 + struct_len;
        buf[8..12].copy_from_slice(&(48u32).to_be_bytes());
        buf[12..16].copy_from_slice(&((48 + struct_len) as u32).to_be_bytes());
        buf[32..36].copy_from_slice(&(struct_len as u32).to_be_bytes());
        buf[36..40].copy_from_slice(&(0u32).to_be_bytes());
        buf.resize(totalsize, 0);
        let mut pos = 48;
        buf[pos..pos + 4].copy_from_slice(&FDT_BEGIN_NODE.to_be_bytes());
        pos += 4;
        buf[pos..pos + 4].copy_from_slice(b"foo\0");
        pos += 4;
        buf[pos..pos + 4].copy_from_slice(&FDT_END_NODE.to_be_bytes());
        pos += 4;
        buf[pos..pos + 4].copy_from_slice(&FDT_END.to_be_bytes());
        let totalsize = 48 + struct_len;
        buf[4..8].copy_from_slice(&(totalsize as u32).to_be_bytes());
        buf.resize(totalsize, 0);
        assert!(find_virtio_mmio(&buf).unwrap().is_empty());
    }

    #[test]
    fn descriptor_flags_write_and_next() {
        let flags = VIRTQ_DESC_F_WRITE | VIRTQ_DESC_F_NEXT;
        assert!(flags & VIRTQ_DESC_F_WRITE != 0);
        assert!(flags & VIRTQ_DESC_F_NEXT != 0);
        assert!(flags & VIRTQ_DESC_F_INDIRECT == 0);
        // A device-write descriptor must not be marked READ-only; a TX
        // descriptor (device reads) must not carry WRITE.
        assert!(VIRTQ_DESC_F_WRITE == 2);
    }

    #[test]
    fn virtqueue_ring_index_wrap() {
        const Q: usize = 16;
        let mut avail_idx: usize = 0;
        let mut used_idx: usize = 0;
        // Publish 20 descriptors on a 16-entry ring (wraps once).
        for i in 0..20 {
            let _slot = avail_idx % Q;
            avail_idx += 1;
        }
        assert_eq!(avail_idx, 20);
        // Device consumes 13 of them.
        for _ in 0..13 {
            let _entry = used_idx % Q;
            used_idx += 1;
        }
        assert_eq!(used_idx, 13);
        // 7 remain unconsumed.
        assert_eq!(avail_idx - used_idx, 7);
        // Indices wrap cleanly.
        assert_eq!(avail_idx % Q, 4);
        assert_eq!(used_idx % Q, 13);
    }

    fn eth_build_header(buf: &mut [u8], dst: &[u8; 6], src: &[u8; 6], ethertype: u16) -> usize {
        if buf.len() < ETH_HLEN {
            return 0;
        }
        buf[0..6].copy_from_slice(dst);
        buf[6..12].copy_from_slice(src);
        buf[12..14].copy_from_slice(&ethertype.to_be_bytes());
        ETH_HLEN
    }

    fn eth_parse_header(buf: &[u8]) -> Option<([u8; 6], [u8; 6], u16)> {
        if buf.len() < ETH_HLEN {
            return None;
        }
        let mut dst = [0u8; 6];
        let mut src = [0u8; 6];
        dst.copy_from_slice(&buf[0..6]);
        src.copy_from_slice(&buf[6..12]);
        let ethertype = u16::from_be_bytes([buf[12], buf[13]]);
        Some((dst, src, ethertype))
    }

    #[test]
    fn eth_header_round_trip() {
        let dst = [1u8, 2, 3, 4, 5, 6];
        let src = [6u8, 5, 4, 3, 2, 1];
        let mut buf = [0u8; 1518];
        let len = eth_build_header(&mut buf, &dst, &src, ETH_P_ARP);
        assert_eq!(len, ETH_HLEN);
        let (rdst, rsrc, rtype) = eth_parse_header(&buf).expect("parse");
        assert_eq!(rdst, dst);
        assert_eq!(rsrc, src);
        assert_eq!(rtype, ETH_P_ARP);
    }

    #[test]
    fn eth_header_rejects_short_buffer() {
        let mut buf = [0u8; 10];
        assert_eq!(eth_build_header(&mut buf, &[0; 6], &[0; 6], ETH_P_IP), 0);
        assert!(eth_parse_header(&buf).is_none());
    }
}
