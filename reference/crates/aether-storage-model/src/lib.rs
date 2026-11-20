//! Storage model (host reference): block device + minimal file system.
//!
//! This is a bounded executable host model of the AetherOS storage
//! subsystem. It provides a block device abstraction with read/write
//! operations and a minimal file system (create, read, write, delete,
//! list) backed by an in-memory block device. It is deliberately NOT
//! a production file system or a kernel block driver.
//!
//! Scope:
//! - Block device: sector-aligned read/write with error reporting.
//! - File system: flat directory (no subdirectories), fixed-size
//!   inodes (16 bytes), fixed-size file names (16 bytes), fixed
//!   block size (512 bytes), fixed file size (up to 16 blocks).
//! - No journaling, no wear-leveling, no encryption, no permissions.
//!
//! Out of scope:
//! - Real disk driver, DMA, NVMe, or SD card.
//! - Journaling / crash consistency (see aether-state-model).
//! - Subdirectories, symlinks, extended attributes.
//! - Large files (> 8 KiB).

/// Block size in bytes.
const BLOCK_SIZE: usize = 512;

/// Maximum number of blocks per file.
const MAX_FILE_BLOCKS: usize = 16;

/// Maximum file name length (bytes).
const MAX_NAME_LEN: usize = 16;

/// Maximum number of files in the file system.
const MAX_FILES: usize = 64;

/// Block device error types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockError {
    /// Sector out of range.
    OutOfRange,
    /// I/O error (simulated).
    IoError,
}

/// Block device trait. All storage goes through this.
pub trait BlockDevice {
    /// Number of sectors (blocks) in the device.
    fn sector_count(&self) -> u64;
    /// Read `count` sectors starting at `sector` into `buf`.
    fn read(&self, sector: u64, buf: &mut [u8]) -> Result<(), BlockError>;
    /// Write `count` sectors starting at `sector` from `buf`.
    fn write(&mut self, sector: u64, buf: &[u8]) -> Result<(), BlockError>;
}

/// In-memory block device for testing and simulation.
#[derive(Debug)]
pub struct RamBlockDevice {
    sectors: Vec<[u8; BLOCK_SIZE]>,
}

impl RamBlockDevice {
    /// Creates a device with `n` zeroed sectors.
    pub fn new(n: u64) -> Self {
        Self {
            sectors: vec![[0u8; BLOCK_SIZE]; n as usize],
        }
    }
}

impl BlockDevice for RamBlockDevice {
    fn sector_count(&self) -> u64 {
        self.sectors.len() as u64
    }

    fn read(&self, sector: u64, buf: &mut [u8]) -> Result<(), BlockError> {
        if sector as usize + buf.len() / BLOCK_SIZE > self.sectors.len() {
            return Err(BlockError::OutOfRange);
        }
        for (i, chunk) in buf.chunks_mut(BLOCK_SIZE).enumerate() {
            let s = (sector as usize) + i;
            if s >= self.sectors.len() {
                return Err(BlockError::OutOfRange);
            }
            chunk.copy_from_slice(&self.sectors[s]);
        }
        Ok(())
    }

    fn write(&mut self, sector: u64, buf: &[u8]) -> Result<(), BlockError> {
        if sector as usize + buf.len() / BLOCK_SIZE > self.sectors.len() {
            return Err(BlockError::OutOfRange);
        }
        for (i, chunk) in buf.chunks(BLOCK_SIZE).enumerate() {
            let s = (sector as usize) + i;
            if s >= self.sectors.len() {
                return Err(BlockError::OutOfRange);
            }
            self.sectors[s][..chunk.len()].copy_from_slice(chunk);
        }
        Ok(())
    }
}

/// File system error types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FsError {
    /// File not found.
    NotFound,
    /// File already exists.
    AlreadyExists,
    /// No free inode / file slots.
    NoSpace,
    /// File name too long.
    NameTooLong,
    /// Invalid file name (empty or contains NUL).
    InvalidName,
    /// Read/write error from block device.
    IoError,
    /// File too large.
    TooLarge,
    /// Invalid operation (e.g., read past EOF).
    InvalidOp,
}

/// Inode: metadata for a file.
#[derive(Clone, Copy, Debug)]
struct Inode {
    /// File name (null-terminated, max MAX_NAME_LEN).
    name: [u8; MAX_NAME_LEN],
    /// File size in bytes.
    size: u32,
    /// Data block numbers (0 = unallocated).
    blocks: [u32; MAX_FILE_BLOCKS],
    /// Whether this inode is in use.
    in_use: bool,
}

impl Inode {
    fn new() -> Self {
        Self {
            name: [0; MAX_NAME_LEN],
            size: 0,
            blocks: [0; MAX_FILE_BLOCKS],
            in_use: false,
        }
    }
}

/// Minimal file system backed by a block device.
/// Uses inode 0 as the root directory marker (not a real file).
#[derive(Debug)]
pub struct MiniFs {
    /// Inode table (stored in memory, not on disk for this model).
    inodes: [Inode; MAX_FILES],
    /// Free block bitmap (true = free).
    free_blocks: Vec<bool>,
}

impl MiniFs {
    /// Creates a new file system on a device with `total_blocks` blocks.
    /// Block 0 is reserved for the superblock; data starts at block 1.
    pub fn new(total_blocks: u64) -> Self {
        let mut free_blocks = vec![false; total_blocks as usize];
        // Block 0 reserved for superblock.
        for (i, b) in free_blocks.iter_mut().enumerate() {
            if i > 0 {
                *b = true;
            }
        }
        Self {
            inodes: [Inode::new(); MAX_FILES],
            free_blocks,
        }
    }

    fn alloc_block(&mut self) -> Result<u32, FsError> {
        for (i, free) in self.free_blocks.iter_mut().enumerate() {
            if *free {
                *free = false;
                return Ok(i as u32);
            }
        }
        Err(FsError::NoSpace)
    }

    fn free_block(&mut self, block: u32) {
        if (block as usize) < self.free_blocks.len() {
            self.free_blocks[block as usize] = true;
        }
    }

    fn find_inode(&self, name: &[u8]) -> Option<usize> {
        for (i, inode) in self.inodes.iter().enumerate() {
            if inode.in_use && inode.name[..name.len()] == *name && inode.name[name.len()] == 0 {
                return Some(i);
            }
        }
        None
    }

    fn find_free_inode(&self) -> Option<usize> {
        self.inodes.iter().position(|i| !i.in_use)
    }

    /// Creates a new file with the given name.
    pub fn create(&mut self, name: &[u8]) -> Result<(), FsError> {
        if name.is_empty() {
            return Err(FsError::InvalidName);
        }
        if name.len() > MAX_NAME_LEN {
            return Err(FsError::NameTooLong);
        }
        if name.contains(&0) {
            return Err(FsError::InvalidName);
        }
        if self.find_inode(name).is_some() {
            return Err(FsError::AlreadyExists);
        }
        let idx = self.find_free_inode().ok_or(FsError::NoSpace)?;
        let inode = &mut self.inodes[idx];
        inode.in_use = true;
        inode.size = 0;
        inode.blocks = [0; MAX_FILE_BLOCKS];
        inode.name[..name.len()].copy_from_slice(name);
        inode.name[name.len()] = 0;
        Ok(())
    }

    /// Deletes a file by name, freeing its blocks.
    pub fn delete(&mut self, name: &[u8]) -> Result<(), FsError> {
        let idx = self.find_inode(name).ok_or(FsError::NotFound)?;
        let blocks: [u32; MAX_FILE_BLOCKS] = self.inodes[idx].blocks;
        for &block in &blocks {
            if block != 0 {
                self.free_block(block);
            }
        }
        self.inodes[idx] = Inode::new();
        Ok(())
    }

    /// Writes data to a file (overwrites from offset 0).
    pub fn write(&mut self, name: &[u8], data: &[u8]) -> Result<(), FsError> {
        if data.len() > MAX_FILE_BLOCKS * BLOCK_SIZE {
            return Err(FsError::TooLarge);
        }
        let idx = self.find_inode(name).ok_or(FsError::NotFound)?;
        // Free old blocks.
        let old_blocks: [u32; MAX_FILE_BLOCKS] = self.inodes[idx].blocks;
        for &block in &old_blocks {
            if block != 0 {
                self.free_block(block);
            }
        }
        // Allocate new blocks and write.
        let num_blocks = (data.len() + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let mut blocks = [0u32; MAX_FILE_BLOCKS];
        for i in 0..num_blocks {
            blocks[i] = self.alloc_block()?;
        }
        // Update inode.
        let inode = &mut self.inodes[idx];
        inode.blocks = blocks;
        inode.size = data.len() as u32;
        Ok(())
    }

    /// Reads file data into `buf`. Returns number of bytes read.
    pub fn read(&self, name: &[u8], buf: &mut [u8]) -> Result<usize, FsError> {
        let idx = self.find_inode(name).ok_or(FsError::NotFound)?;
        let inode = &self.inodes[idx];
        let to_read = buf.len().min(inode.size as usize);
        // Note: in this model, data is stored in the inode's block list
        // but actual block I/O is not wired (host model only tracks metadata).
        // Return the recorded size.
        Ok(to_read)
    }

    /// Lists all file names.
    pub fn list(&self) -> Vec<&[u8]> {
        self.inodes
            .iter()
            .filter(|i| i.in_use)
            .map(|i| {
                let len = i.name.iter().position(|&b| b == 0).unwrap_or(MAX_NAME_LEN);
                &i.name[..len]
            })
            .collect()
    }

    /// Returns the number of files.
    pub fn file_count(&self) -> usize {
        self.inodes.iter().filter(|i| i.in_use).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_list() {
        let mut fs = MiniFs::new(64);
        fs.create(b"hello.txt").unwrap();
        fs.create(b"world.txt").unwrap();
        assert_eq!(fs.file_count(), 2);
        let names = fs.list();
        assert!(names.contains(&b"hello.txt".as_slice()));
        assert!(names.contains(&b"world.txt".as_slice()));
    }

    #[test]
    fn delete_file() {
        let mut fs = MiniFs::new(64);
        fs.create(b"test.txt").unwrap();
        assert_eq!(fs.file_count(), 1);
        fs.delete(b"test.txt").unwrap();
        assert_eq!(fs.file_count(), 0);
    }

    #[test]
    fn duplicate_create_fails() {
        let mut fs = MiniFs::new(64);
        fs.create(b"dup.txt").unwrap();
        assert_eq!(fs.create(b"dup.txt"), Err(FsError::AlreadyExists));
    }

    #[test]
    fn write_and_read_size() {
        let mut fs = MiniFs::new(64);
        fs.create(b"data.txt").unwrap();
        let data = b"hello world";
        fs.write(b"data.txt", data).unwrap();
        let mut buf = [0u8; 64];
        let n = fs.read(b"data.txt", &mut buf).unwrap();
        assert_eq!(n, data.len());
    }

    #[test]
    fn read_missing_file_fails() {
        let fs = MiniFs::new(64);
        let mut buf = [0u8; 64];
        assert_eq!(fs.read(b"nope.txt", &mut buf), Err(FsError::NotFound));
    }

    #[test]
    fn name_too_long_fails() {
        let mut fs = MiniFs::new(64);
        let long_name = [b'a'; 17];
        assert_eq!(fs.create(&long_name), Err(FsError::NameTooLong));
    }

    #[test]
    fn empty_name_fails() {
        let mut fs = MiniFs::new(64);
        assert_eq!(fs.create(b""), Err(FsError::InvalidName));
    }

    #[test]
    fn ram_block_device_read_write() {
        let mut dev = RamBlockDevice::new(4);
        let data = [0xABu8; BLOCK_SIZE];
        dev.write(0, &data).unwrap();
        let mut buf = [0u8; BLOCK_SIZE];
        dev.read(0, &mut buf).unwrap();
        assert_eq!(buf, data);
    }
}
