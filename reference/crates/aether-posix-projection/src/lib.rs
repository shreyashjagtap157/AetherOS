//! POSIX projection reference implementation (RFC-0020).
//!
//! Maps POSIX filesystem operations onto the State Architecture's object model
//! using a [`StorageProvider`]
//! (RFC-0019) as the underlying storage backend.
//!
//! Core design (RFC-0020):
//! - Every POSIX file or directory is a persistent object (RFC-0015) stored
//!   via a `StorageProvider` (RFC-0019).
//! - POSIX metadata is stored as object metadata (RFC-0020.4).
//! - Directory entries are composite objects: content is ordered
//!   (name, `object_id`) pairs (RFC-0020.5).
//! - Path resolution validates capabilities at each component (RFC-0020.9).
//! - `fsync` commits writes as a RFC-0016 transaction (RFC-0020.6).
//! - `rename` is an atomic RFC-0016 two-phase transaction (RFC-0020.7).
//! - Capability rights take precedence over POSIX mode bits
//!   (RFC-0020.12 / RFC-0020.13).

use aether_event_model::{EventClass, EventEmitter, PlatformEvent};
use aether_object_identity::ObjectIdentity;
use aether_storage_provider::{
    StorageError, StorageProvider, Tier, TransactionalParticipant, apply_staged,
};
use aether_transaction_model::TransactionId;
use aether_types::{DomainId, ObjectId, Rights};
use std::collections::HashMap;

const POSIX_EVENT_CLASS: u64 = EventClass::Storage.id();

/// Open file descriptor flag: read-only.
pub const O_RDONLY: u32 = 0x0001;
/// Open file descriptor flag: write-only.
pub const O_WRONLY: u32 = 0x0002;
/// Open file descriptor flag: read-write.
pub const O_RDWR: u32 = 0x0004;
/// Open file descriptor flag: create if missing.
pub const O_CREAT: u32 = 0x0040;
/// Open file descriptor flag: path must be a directory.
pub const O_DIRECTORY: u32 = 0x0100;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// POSIX projection errors (errno-like semantics).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PosixError {
    /// ENOENT — component of path does not exist.
    NotFound,
    /// ENOTDIR — a path component is not a directory.
    NotADirectory,
    /// EISDIR — operation expects a non-directory object.
    IsADirectory,
    /// EEXIST — target already exists.
    AlreadyExists,
    /// EACCES — capability rights or mode bits deny access.
    PermissionDenied,
    /// EBADF — invalid file descriptor.
    BadF,
    /// EINVAL — invalid argument.
    InvalidArgument,
    /// Underlying storage provider error.
    Storage(StorageError),
}

/// Result type returned by all POSIX projection operations.
pub type PosixResult<T> = Result<T, PosixError>;

impl From<StorageError> for PosixError {
    fn from(e: StorageError) -> Self {
        PosixError::Storage(e)
    }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Type of a POSIX file system object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileType {
    /// Regular file.
    RegularFile,
    /// Directory.
    Directory,
}

/// POSIX metadata stored as object metadata (RFC-0020.4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PosixMetadata {
    /// Object identity of the file or directory.
    pub object_id: ObjectId,
    /// File type.
    pub file_type: FileType,
    /// POSIX mode bits.
    pub mode: u32,
    /// Owner user id.
    pub uid: u32,
    /// Owner group id.
    pub gid: u32,
    /// Size in bytes.
    pub size: u64,
    /// Hard link count.
    pub link_count: u64,
    /// Last access time in scheduling nanoseconds (RFC-0012).
    pub atime_ns: u64,
    /// Last modification time in scheduling nanoseconds.
    pub mtime_ns: u64,
    /// Last status change time in scheduling nanoseconds.
    pub ctime_ns: u64,
}

/// Directory entry: a name bound to an object identity (RFC-0020.5).
#[derive(Clone, Debug, Eq, PartialEq)]
struct DirEntry {
    /// Entry name within the parent directory.
    name: String,
    /// Object identity the name resolves to.
    object_id: ObjectId,
}

/// Open file descriptor state.
struct OpenFile {
    /// Object currently backing this fd (may change after write+fsync).
    object_id: ObjectId,
    /// Parent directory containing the entry for this file.
    #[allow(dead_code)]
    parent_dir_id: ObjectId,
    /// Current read/write offset.
    offset: u64,
    /// Whether reads are permitted.
    readable: bool,
    /// Whether writes are permitted.
    writable: bool,
}

// ---------------------------------------------------------------------------
// Directory serialization
// ---------------------------------------------------------------------------

/// Serializes directory entries: `[u8:name_len][name][u128:object_id_le]` repeated.
fn serialize_dir(entries: &[DirEntry]) -> Vec<u8> {
    let mut buf = Vec::new();
    for entry in entries {
        let name_len =
            u8::try_from(entry.name.len()).expect("directory entry name must fit in a u8");
        buf.push(name_len);
        buf.extend_from_slice(entry.name.as_bytes());
        buf.extend_from_slice(&entry.object_id.0.to_le_bytes());
    }
    buf
}

/// Deserializes directory entries.
fn deserialize_dir(content: &[u8]) -> PosixResult<Vec<DirEntry>> {
    let mut entries = Vec::new();
    let mut pos = 0;
    while pos < content.len() {
        if pos + 1 > content.len() {
            return Err(PosixError::InvalidArgument);
        }
        let name_len = content[pos] as usize;
        pos += 1;
        if pos + name_len + 16 > content.len() {
            return Err(PosixError::InvalidArgument);
        }
        let name = String::from_utf8_lossy(&content[pos..pos + name_len]).into_owned();
        pos += name_len;
        let mut id_bytes = [0u8; 16];
        id_bytes.copy_from_slice(&content[pos..pos + 16]);
        pos += 16;
        entries.push(DirEntry {
            name,
            object_id: ObjectId(u128::from_le_bytes(id_bytes)),
        });
    }
    Ok(entries)
}

/// Computes an object identity from content bytes (RFC-0020.3).
#[allow(dead_code)]
fn content_to_object_id(content: &[u8]) -> ObjectId {
    let identity = ObjectIdentity::compute(content);
    ObjectId::from(identity)
}

/// Returns the entry named `name` in directory `dir_id`, if present.
fn find_entry(entries: &[DirEntry], name: &str) -> Option<ObjectId> {
    entries.iter().find(|e| e.name == name).map(|e| e.object_id)
}

// ---------------------------------------------------------------------------
// PosixProjection
// ---------------------------------------------------------------------------

/// POSIX projection over a [`StorageProvider`].
///
/// Every POSIX file or directory is a persistent object stored through the
/// provider. Path resolution validates capabilities at each component
/// (RFC-0020.9). Writes are staged and committed via `fsync`
/// (RFC-0020.6); `rename` is atomic (RFC-0020.7).
pub struct PosixProjection<P: StorageProvider> {
    provider: P,
    participant: TransactionalParticipant,
    fds: HashMap<u64, OpenFile>,
    next_fd: u64,
    next_object_id: u64,
    dirty_fd: Option<u64>,
    root_dir_id: ObjectId,
    capabilities: HashMap<(u16, ObjectId), Rights>,
    metadata: HashMap<ObjectId, PosixMetadata>,
    emitter: EventEmitter,
    next_xid_nonce: u64,
}

impl<P: StorageProvider> PosixProjection<P> {
    /// Creates a new projection over `provider` with `root_dir_id` as root.
    #[must_use]
    pub fn new(provider: P, root_dir_id: ObjectId) -> Self {
        Self {
            provider,
            participant: TransactionalParticipant::new(true),
            fds: HashMap::new(),
            next_fd: 3,
            next_object_id: 1,
            dirty_fd: None,
            root_dir_id,
            capabilities: HashMap::new(),
            metadata: HashMap::new(),
            emitter: EventEmitter::new(),
            next_xid_nonce: 1,
        }
    }

    /// Allocates a fresh, unique object id for a newly created object.
    #[must_use]
    fn next_object_id(&mut self) -> ObjectId {
        let id = ObjectId(u128::from(self.next_object_id));
        self.next_object_id += 1;
        id
    }

    /// Begins a fresh transaction on a new participant (avoids the coordinator's
    /// write-write conflict tracking across independent operations).
    fn begin_fresh_txn(&mut self, timestamp_ns: u64) -> PosixResult<TransactionId> {
        self.participant = TransactionalParticipant::new(true);
        let xid = self.next_xid();
        self.participant
            .begin_transaction(xid, timestamp_ns)
            .map_err(PosixError::from)?;
        Ok(xid)
    }

    /// Commits the current transaction, applies staged mutations, and resets the
    /// participant for the next operation.
    fn commit_current_txn(&mut self, timestamp_ns: u64) -> PosixResult<()> {
        self.participant
            .commit_transaction(timestamp_ns)
            .map_err(PosixError::from)?;
        apply_staged(&mut self.provider, &self.participant).map_err(PosixError::from)?;
        self.participant.mark_committed();
        Ok(())
    }

    /// Grants `rights` on `object_id` to `domain` (RFC-0020.9).
    pub fn grant_capability(&mut self, domain: DomainId, object_id: ObjectId, rights: Rights) {
        self.capabilities.insert((domain.get(), object_id), rights);
    }

    /// Returns the root directory object id.
    #[must_use]
    pub fn root_dir_id(&self) -> ObjectId {
        self.root_dir_id
    }

    /// Returns all emitted events.
    #[must_use]
    pub fn events(&self) -> &[PlatformEvent] {
        self.emitter.events()
    }

    /// Resolves `path` to an object identity, validating capabilities at each
    /// component (RFC-0020.8 / RFC-0020.9).
    ///
    /// `domain` is the requesting domain id. Each directory component requires
    /// `READ | EXECUTE` rights for that domain.
    ///
    /// # Errors
    ///
    /// Returns [`PosixError::NotFound`] if any component of `path` does not
    /// exist, [`PosixError::NotADirectory`] if an intermediate component is not
    /// a directory, or [`PosixError::PermissionDenied`] if a capability check
    /// fails.
    fn resolve_path(&self, path: &str, domain: u16) -> PosixResult<ObjectId> {
        if path.is_empty() {
            return Err(PosixError::NotFound);
        }
        let components: Vec<&str> = if let Some(rest) = path.strip_prefix('/') {
            rest.split('/').filter(|c| !c.is_empty()).collect()
        } else {
            path.split('/').filter(|c| !c.is_empty()).collect()
        };
        if components.is_empty() {
            // Root directory.
            self.check_capability(
                domain,
                self.root_dir_id,
                Rights::READ.union(Rights::EXECUTE),
            )?;
            return Ok(self.root_dir_id);
        }
        let mut current = self.root_dir_id;
        for (i, comp) in components.iter().enumerate() {
            // Check capability on the current directory.
            self.check_capability(domain, current, Rights::READ.union(Rights::EXECUTE))?;
            let is_last = i == components.len() - 1;
            let entry = self.lookup(current, comp)?;
            if is_last {
                return Ok(entry);
            }
            // Intermediate component must be a directory.
            let meta_opt = self.metadata.get(&entry);
            if meta_opt.is_some_and(|m| m.file_type == FileType::Directory) {
                current = entry;
            } else {
                return Err(PosixError::NotADirectory);
            }
        }
        Ok(current)
    }

    /// Looks up `name` in directory `dir_id`.
    fn lookup(&self, dir_id: ObjectId, name: &str) -> PosixResult<ObjectId> {
        let content = self.provider.get(dir_id).map_err(PosixError::from)?;
        let entries = deserialize_dir(&content)?;
        find_entry(&entries, name).ok_or(PosixError::NotFound)
    }

    /// Checks that `domain` holds `rights` on `object_id` (RFC-0020.9).
    fn check_capability(
        &self,
        domain: u16,
        object_id: ObjectId,
        rights: Rights,
    ) -> PosixResult<()> {
        let held = self
            .capabilities
            .get(&(domain, object_id))
            .copied()
            .unwrap_or(Rights::NONE);
        if held.contains(rights) {
            Ok(())
        } else {
            Err(PosixError::PermissionDenied)
        }
    }

    /// Sets POSIX metadata for `object_id`.
    fn set_posix_metadata(&mut self, object_id: ObjectId, meta: PosixMetadata) {
        self.metadata.insert(object_id, meta);
    }

    /// Allocates the next transaction id and increments the nonce.
    fn next_xid(&mut self) -> TransactionId {
        let mut bytes = [0u8; 16];
        let n = self.next_xid_nonce.to_le_bytes();
        bytes[..8].copy_from_slice(&n);
        self.next_xid_nonce += 1;
        TransactionId::from_bytes(bytes)
    }

    /// Emits a POSIX projection event.
    fn emit_event(&mut self, event_type: u64, message: &[u8]) {
        let mut payload = Vec::new();
        payload.extend_from_slice(message);
        let event = PlatformEvent {
            event_id: 0,
            event_class: POSIX_EVENT_CLASS,
            event_type,
            timestamp_ns: 0,
            emitter_thread: 1,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: 50,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload,
        };
        let _ = self.emitter.emit(event);
    }

    // ------------------------------------------------------------------
    // Operations
    // ------------------------------------------------------------------

    /// Opens `path` and returns a file descriptor (RFC-0020.1).
    ///
    /// `flags` may combine `O_RDONLY`, `O_WRONLY`, `O_RDWR`, `O_CREAT`,
    /// `O_DIRECTORY`. `domain` is the requesting domain for capability checks.
    ///
    /// # Errors
    ///
    /// Returns [`PosixError::NotFound`] if the path does not exist and
    /// `O_CREAT` is not set, [`PosixError::AlreadyExists`] if `O_CREAT` is set
    /// and the path exists, [`PosixError::PermissionDenied`] if a capability
    /// check fails, or [`PosixError::InvalidArgument`] if the path is invalid.
    ///
    /// # Panics
    ///
    /// Panics if `DomainId::new(domain)` returns `None` (only domain 0 is
    /// invalid; never happens in practice).
    pub fn open(&mut self, path: &str, flags: u32, domain: u16) -> PosixResult<u64> {
        let readonly = flags & O_RDONLY != 0 || flags & O_RDWR != 0;
        let writeable = flags & O_WRONLY != 0 || flags & O_RDWR != 0;
        let is_dir = flags & O_DIRECTORY != 0;
        let create = flags & O_CREAT != 0;

        match self.resolve_path(path, domain) {
            Ok(oid) => {
                // Object exists.
                let meta_opt = self.metadata.get(&oid);
                if is_dir && meta_opt.is_some_and(|m| m.file_type != FileType::Directory) {
                    return Err(PosixError::NotADirectory);
                }
                if !is_dir && meta_opt.is_some_and(|m| m.file_type == FileType::Directory) {
                    return Err(PosixError::IsADirectory);
                }
                if readonly {
                    self.check_capability(domain, oid, Rights::READ)?;
                }
                if writeable {
                    self.check_capability(domain, oid, Rights::WRITE)?;
                }
                // Find parent dir id for fd tracking.
                let parent = self.parent_of(path)?;
                let fd = self.next_fd;
                self.next_fd += 1;
                self.fds.insert(
                    fd,
                    OpenFile {
                        object_id: oid,
                        parent_dir_id: parent,
                        offset: 0,
                        readable: readonly,
                        writable: writeable,
                    },
                );
                self.emit_event(1, b"file opened");
                Ok(fd)
            }
            Err(PosixError::NotFound) if create => {
                // Create a new regular file.
                let parent_path = Self::parent_path(path);
                let parent_id = self.resolve_path(parent_path, domain)?;
                self.check_capability(
                    domain,
                    parent_id,
                    Rights::READ.union(Rights::WRITE).union(Rights::EXECUTE),
                )?;
                // Grant the creating domain capabilities on the new file.
                let new_id = self.next_object_id();
                let now = 0;
                let meta = PosixMetadata {
                    object_id: new_id,
                    file_type: FileType::RegularFile,
                    mode: 0o644,
                    uid: 0,
                    gid: 0,
                    size: 0,
                    link_count: 1,
                    atime_ns: now,
                    mtime_ns: now,
                    ctime_ns: now,
                };
                self.set_posix_metadata(new_id, meta.clone());
                self.grant_capability(
                    DomainId::new(domain).unwrap(),
                    new_id,
                    Rights::READ.union(Rights::WRITE),
                );
                // Put the empty new file object directly (apply_staged treats
                // empty content as a delete, so new empty objects must be put
                // outside the transaction).
                self.provider
                    .put(new_id, &[], Tier::Hot)
                    .map_err(PosixError::from)?;
                // Commit the creation in one transaction: begin must precede
                // all stage_put calls (RFC-0019.9).
                self.begin_fresh_txn(now)?;
                // Add entry to parent directory (stages parent update).
                let name = path.rsplit('/').next().unwrap_or(path);
                self.add_dir_entry(domain, parent_id, name, new_id)?;
                self.commit_current_txn(now)?;
                // Open the newly created file.
                let fd = self.next_fd;
                self.next_fd += 1;
                self.fds.insert(
                    fd,
                    OpenFile {
                        object_id: new_id,
                        parent_dir_id: parent_id,
                        offset: 0,
                        readable: readonly,
                        writable: writeable,
                    },
                );
                self.emit_event(1, b"file created and opened");
                Ok(fd)
            }
            Err(e) => Err(e),
        }
    }

    /// Reads up to `buf.len()` bytes from `fd` at the current offset.
    ///
    /// # Errors
    ///
    /// Returns [`PosixError::BadF`] if `fd` is not open, or
    /// [`PosixError::PermissionDenied`] if `fd` is not readable.
    pub fn read(&mut self, fd: u64, buf: &mut [u8]) -> PosixResult<usize> {
        let open = self.fds.get(&fd).ok_or(PosixError::BadF)?;
        if !open.readable {
            return Err(PosixError::PermissionDenied);
        }
        let content = self
            .provider
            .get(open.object_id)
            .map_err(PosixError::from)?;
        let start = usize::try_from(open.offset).map_err(|_| PosixError::InvalidArgument)?;
        if start >= content.len() {
            return Ok(0);
        }
        let end = (start + buf.len()).min(content.len());
        let n = end - start;
        buf[..n].copy_from_slice(&content[start..end]);
        // Update offset (need mutable borrow, so re-acquire).
        let open = self.fds.get_mut(&fd).ok_or(PosixError::BadF)?;
        open.offset = end as u64;
        self.emit_event(2, b"file read");
        Ok(n)
    }

    /// Writes `buf` to `fd` at the current offset (staged; committed on fsync).
    ///
    /// # Errors
    ///
    /// Returns [`PosixError::BadF`] if `fd` is not open, or
    /// [`PosixError::PermissionDenied`] if `fd` is not writable.
    pub fn write(&mut self, fd: u64, buf: &[u8]) -> PosixResult<usize> {
        let open = self.fds.get(&fd).ok_or(PosixError::BadF)?;
        if !open.writable {
            return Err(PosixError::PermissionDenied);
        }
        let object_id = open.object_id;
        let offset = open.offset;
        let content = self.provider.get(object_id).map_err(PosixError::from)?;
        let start = usize::try_from(offset).map_err(|_| PosixError::InvalidArgument)?;
        let mut new_content = content.clone();
        if start + buf.len() > new_content.len() {
            new_content.resize(start + buf.len(), 0);
        }
        new_content[start..start + buf.len()].copy_from_slice(buf);
        // Stage the new content under the same object id (write in place;
        // committed on fsync).
        let now = 0;
        self.begin_fresh_txn(now)?;
        self.participant
            .stage_put(object_id, new_content.clone(), Tier::Hot)
            .map_err(PosixError::from)?;
        // Mark this fd as owning the pending transaction.
        self.dirty_fd = Some(fd);
        // Update fd state.
        let open = self.fds.get_mut(&fd).ok_or(PosixError::BadF)?;
        open.offset = (start + buf.len()) as u64;
        // Update metadata size.
        if let Some(meta) = self.metadata.get_mut(&object_id) {
            meta.size = new_content.len() as u64;
            meta.mtime_ns = now;
            meta.ctime_ns = now;
        }
        self.emit_event(3, b"file write staged");
        Ok(buf.len())
    }

    /// Closes `fd`.
    ///
    /// # Errors
    ///
    /// Returns [`PosixError::BadF`] if `fd` is not open.
    pub fn close(&mut self, fd: u64) -> PosixResult<()> {
        // Commit any pending staged writes owned by this fd (RFC-0020.6).
        if self.dirty_fd == Some(fd) {
            let now = 0;
            self.commit_current_txn(now)?;
            self.dirty_fd = None;
        }
        if self.fds.remove(&fd).is_some() {
            self.emit_event(4, b"file closed");
            Ok(())
        } else {
            Err(PosixError::BadF)
        }
    }

    /// Returns POSIX metadata for `path` (RFC-0020.1 / RFC-0020.4).
    ///
    /// # Errors
    ///
    /// Returns [`PosixError::NotFound`] if the path does not exist, or
    /// [`PosixError::PermissionDenied`] if a capability check fails.
    pub fn stat(&mut self, path: &str, domain: u16) -> PosixResult<PosixMetadata> {
        let object_id = self.resolve_path(path, domain)?;
        self.check_capability(domain, object_id, Rights::READ)?;
        let meta = self
            .metadata
            .get(&object_id)
            .cloned()
            .ok_or(PosixError::NotFound)?;
        Ok(meta)
    }

    /// Creates a directory at `path` (RFC-0020.1).
    ///
    /// # Errors
    ///
    /// Returns [`PosixError::AlreadyExists`] if `path` already exists, or
    /// [`PosixError::PermissionDenied`] if a capability check fails.
    ///
    /// # Panics
    ///
    /// Panics if `DomainId::new(domain)` returns `None` (only domain 0 is
    /// invalid; never happens in practice).
    pub fn mkdir(&mut self, path: &str, domain: u16, mode: u32) -> PosixResult<()> {
        let parent_path = Self::parent_path(path);
        let parent_id = self.resolve_path(parent_path, domain)?;
        self.check_capability(
            domain,
            parent_id,
            Rights::READ.union(Rights::WRITE).union(Rights::EXECUTE),
        )?;
        let name = path.rsplit('/').next().unwrap_or(path);
        // Check name doesn't already exist.
        let entries = self.provider.get(parent_id).map_err(PosixError::from)?;
        let dir_entries = deserialize_dir(&entries)?;
        if find_entry(&dir_entries, name).is_some() {
            return Err(PosixError::AlreadyExists);
        }
        let new_id = self.next_object_id();
        let now = 0;
        let meta = PosixMetadata {
            object_id: new_id,
            file_type: FileType::Directory,
            mode,
            uid: 0,
            gid: 0,
            size: 0,
            link_count: 2, // . and ..
            atime_ns: now,
            mtime_ns: now,
            ctime_ns: now,
        };
        self.set_posix_metadata(new_id, meta);
        // Grant the creating domain capabilities on the new directory so it
        // can be traversed and modified (RFC-0020.9).
        self.grant_capability(
            DomainId::new(domain).unwrap(),
            new_id,
            Rights::READ.union(Rights::WRITE).union(Rights::EXECUTE),
        );
        // Put the new directory object directly (empty content; apply_staged
        // treats empty content as a delete, so new empty objects must be put
        // outside the transaction).
        self.provider
            .put(new_id, &[], Tier::Hot)
            .map_err(PosixError::from)?;
        // Stage only the parent directory update within one transaction
        // (begin must precede stage_put, RFC-0019.9).
        let now = 0;
        self.begin_fresh_txn(now)?;
        // Add entry to parent directory.
        let mut dir_entries =
            deserialize_dir(&self.provider.get(parent_id).map_err(PosixError::from)?)?;
        dir_entries.push(DirEntry {
            name: name.to_string(),
            object_id: new_id,
        });
        let parent_content = serialize_dir(&dir_entries);
        // Stage the updated directory under its existing object id so the
        // provider can look it up by that id.
        self.participant
            .stage_put(parent_id, parent_content, Tier::Hot)
            .map_err(PosixError::from)?;
        self.commit_current_txn(now)?;
        self.emit_event(5, b"directory created");
        Ok(())
    }

    /// Commits all staged writes for `fd` as a transaction (RFC-0020.6).
    ///
    /// # Errors
    ///
    /// Returns [`PosixError::BadF`] if `fd` is not open.
    pub fn fsync(&mut self, fd: u64) -> PosixResult<()> {
        let _open = self.fds.get(&fd).ok_or(PosixError::BadF)?;
        if !self.participant.in_transaction() {
            // Nothing staged; nothing to commit.
            return Ok(());
        }
        let now = 0;
        self.commit_current_txn(now)?;
        if self.dirty_fd == Some(fd) {
            self.dirty_fd = None;
        }
        self.emit_event(6, b"fsync committed");
        Ok(())
    }

    /// Atomically renames `old_path` to `new_path` (RFC-0020.7).
    ///
    /// Implemented as a two-phase transaction: the source entry is removed and
    /// the target entry is added atomically. If the target exists, it is
    /// replaced.
    ///
    /// # Errors
    ///
    /// Returns [`PosixError::NotFound`] if `old_path` does not exist, or
    /// [`PosixError::PermissionDenied`] if a capability check fails.
    pub fn rename(&mut self, old_path: &str, new_path: &str, domain: u16) -> PosixResult<()> {
        let old_obj_id = self.resolve_path(old_path, domain)?;
        let new_parent_id = self.resolve_path(Self::parent_path(new_path), domain)?;
        let new_name = new_path.rsplit('/').next().unwrap_or(new_path);

        // Read the old parent directory.
        let old_parent_id = self.resolve_path(Self::parent_path(old_path), domain)?;
        let old_entries_content = self.provider.get(old_parent_id).map_err(PosixError::from)?;
        // Remove old entry from old parent.
        let old_name = old_path.rsplit('/').next().unwrap_or(old_path);
        let mut old_entries = deserialize_dir(&old_entries_content)?;
        let old_len = old_entries.len();
        old_entries.retain(|e| e.name != old_name);
        if old_entries.len() == old_len {
            return Err(PosixError::NotFound);
        }

        // Read the new parent directory.
        let new_entries_content = self.provider.get(new_parent_id).map_err(PosixError::from)?;
        let mut new_entries = deserialize_dir(&new_entries_content)?;

        // Remove existing target entry if present (replace semantics).
        new_entries.retain(|e| e.name != new_name);
        new_entries.push(DirEntry {
            name: new_name.to_string(),
            object_id: old_obj_id,
        });

        let now = 0;
        self.begin_fresh_txn(now)?;

        // Compute final entries for old parent: remove old_name.
        let mut old_final: Vec<DirEntry> = old_entries;
        old_final.retain(|e| e.name != old_name);

        if old_parent_id == new_parent_id {
            // Same directory: also remove new_name (replace semantics) and add
            // new_name -> old_obj_id, all in one staged put.
            old_final.retain(|e| e.name != new_name);
            old_final.push(DirEntry {
                name: new_name.to_string(),
                object_id: old_obj_id,
            });
            let content = serialize_dir(&old_final);
            self.participant
                .stage_put(old_parent_id, content, Tier::Hot)
                .map_err(PosixError::from)?;
        } else {
            // Different directories: stage each parent separately.
            let old_parent_content = serialize_dir(&old_final);
            self.participant
                .stage_put(old_parent_id, old_parent_content, Tier::Hot)
                .map_err(PosixError::from)?;

            let mut new_final: Vec<DirEntry> = new_entries;
            new_final.retain(|e| e.name != new_name);
            new_final.push(DirEntry {
                name: new_name.to_string(),
                object_id: old_obj_id,
            });
            let new_parent_content = serialize_dir(&new_final);
            self.participant
                .stage_put(new_parent_id, new_parent_content, Tier::Hot)
                .map_err(PosixError::from)?;
        }

        self.commit_current_txn(now)?;
        self.emit_event(7, b"rename committed");
        Ok(())
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// Returns the parent directory path of `path`.
    fn parent_path(path: &str) -> &str {
        let trimmed = path.trim_start_matches('/');
        match trimmed.rfind('/') {
            Some(pos) => &trimmed[..pos],
            // Top-level path: parent is the root directory.
            None => "/",
        }
    }

    /// Returns the parent directory object id for `path`.
    fn parent_of(&self, path: &str) -> PosixResult<ObjectId> {
        let parent = Self::parent_path(path);
        if parent.is_empty() {
            Ok(self.root_dir_id)
        } else {
            self.resolve_path(parent, 1)
        }
    }

    /// Adds `(name, object_id)` as an entry of directory `parent_id`, staging
    /// the updated directory object via the transactional participant.
    fn add_dir_entry(
        &mut self,
        domain: u16,
        parent_id: ObjectId,
        name: &str,
        object_id: ObjectId,
    ) -> PosixResult<()> {
        self.check_capability(
            domain,
            parent_id,
            Rights::READ.union(Rights::WRITE).union(Rights::EXECUTE),
        )?;
        let content = self.provider.get(parent_id).map_err(PosixError::from)?;
        let mut entries = deserialize_dir(&content)?;
        if find_entry(&entries, name).is_some() {
            return Err(PosixError::AlreadyExists);
        }
        entries.push(DirEntry {
            name: name.to_string(),
            object_id,
        });
        let parent_content = serialize_dir(&entries);
        // Stage the updated directory under its existing object id.
        self.participant
            .stage_put(parent_id, parent_content, Tier::Hot)
            .map_err(PosixError::from)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Unit tests (CTS-STATE-POSIX-1..5 coverage)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_provider() -> aether_storage_provider::InMemoryProvider {
        aether_storage_provider::InMemoryProvider::new(
            "posix-test",
            aether_storage_provider::ProviderType::Block,
            1024 * 1024,
            aether_storage_provider::LatencyBounds::new(100, 1000, 10_000).unwrap(),
            0.999,
            0x01,
        )
    }

    fn make_root_id() -> ObjectId {
        // Root directory object id from empty content.
        content_to_object_id(&Vec::new())
    }

    fn make_projection() -> PosixProjection<aether_storage_provider::InMemoryProvider> {
        let mut provider = make_provider();
        let root_id = make_root_id();
        // Seed the root directory object so it exists in the provider.
        provider.put(root_id, &[], Tier::Hot).unwrap();
        let mut proj = PosixProjection::new(provider, root_id);
        // Root directory metadata.
        let now = 0;
        let root_meta = PosixMetadata {
            object_id: root_id,
            file_type: FileType::Directory,
            mode: 0o755,
            uid: 0,
            gid: 0,
            size: 0,
            link_count: 2,
            atime_ns: now,
            mtime_ns: now,
            ctime_ns: now,
        };
        proj.set_posix_metadata(root_id, root_meta);
        // Grant full capabilities to domain 1 on everything.
        proj.grant_capability(
            DomainId::new(1).unwrap(),
            root_id,
            Rights::READ.union(Rights::WRITE).union(Rights::EXECUTE),
        );
        proj
    }

    /// CTS-STATE-POSIX-1 (RFC-0020.1): required operations — open, write,
    /// read, close, stat, mkdir work end to end.
    #[test]
    fn cts_posix_1_required_operations() {
        let mut proj = make_projection();

        // mkdir /tmp
        proj.mkdir("/tmp", 1, 0o755).unwrap();

        // Create and open /tmp/hello.txt
        let fd = proj.open("/tmp/hello.txt", O_CREAT | O_WRONLY, 1).unwrap();
        assert!(fd > 0);

        // Write
        let n = proj.write(fd, b"hello world").unwrap();
        assert_eq!(n, 11);

        // Close
        proj.close(fd).unwrap();

        // Re-open and read
        let fd = proj.open("/tmp/hello.txt", O_RDONLY, 1).unwrap();
        let mut buf = [0u8; 16];
        let n = proj.read(fd, &mut buf).unwrap();
        assert_eq!(n, 11);
        assert_eq!(&buf[..n], b"hello world");
        proj.close(fd).unwrap();

        // Stat
        let meta = proj.stat("/tmp/hello.txt", 1).unwrap();
        assert_eq!(meta.file_type, FileType::RegularFile);
        assert_eq!(meta.size, 11);
        assert_eq!(meta.link_count, 1);
    }

    /// CTS-STATE-POSIX-1: open on missing path without O_CREAT fails.
    #[test]
    fn cts_posix_1_open_missing_fails() {
        let mut proj = make_projection();
        assert_eq!(
            proj.open("/nonexistent", O_RDONLY, 1),
            Err(PosixError::NotFound)
        );
    }

    /// CTS-STATE-POSIX-1: mkdir on existing path fails.
    #[test]
    fn cts_posix_1_mkdir_exists() {
        let mut proj = make_projection();
        proj.mkdir("/tmp", 1, 0o755).unwrap();
        assert_eq!(proj.mkdir("/tmp", 1, 0o755), Err(PosixError::AlreadyExists));
    }

    /// CTS-STATE-POSIX-2: object mapping — every file/dir is an object.
    #[test]
    fn cts_posix_2_object_mapping() {
        let mut proj = make_projection();
        proj.mkdir("/tmp", 1, 0o755).unwrap();
        let fd = proj.open("/tmp/obj.txt", O_CREAT | O_WRONLY, 1).unwrap();
        proj.write(fd, b"data").unwrap();
        proj.close(fd).unwrap();

        // The object is retrievable via the provider using its object id.
        let meta = proj.stat("/tmp/obj.txt", 1).unwrap();
        let content = proj.provider.get(meta.object_id).unwrap();
        assert_eq!(content, b"data");

        // Directories are also objects.
        let root_meta = proj.stat("/", 1).unwrap();
        let _root_content = proj.provider.get(root_meta.object_id).unwrap();
    }

    /// CTS-STATE-POSIX-3 (RFC-0020.6): transactional fsync — writes are
    /// invisible until fsync commits.
    #[test]
    fn cts_posix_3_transactional_fsync() {
        let mut proj = make_projection();
        proj.mkdir("/tmp", 1, 0o755).unwrap();
        let fd = proj.open("/tmp/txn.txt", O_CREAT | O_WRONLY, 1).unwrap();

        // Write without fsync.
        proj.write(fd, b"uncommitted").unwrap();

        // A new open (which resolves via the provider, not staged state)
        // should not see the staged write.
        let fd2 = proj.open("/tmp/txn.txt", O_RDONLY, 1).unwrap();
        let mut buf = [0u8; 16];
        let n = proj.read(fd2, &mut buf).unwrap();
        assert_eq!(n, 0); // empty file, uncommitted write not visible
        proj.close(fd2).unwrap();

        // fsync commits the write.
        proj.fsync(fd).unwrap();

        // Now the committed data is visible.
        let fd3 = proj.open("/tmp/txn.txt", O_RDONLY, 1).unwrap();
        let mut buf = [0u8; 16];
        let n = proj.read(fd3, &mut buf).unwrap();
        assert_eq!(n, 11);
        assert_eq!(&buf[..n], b"uncommitted");
        proj.close(fd3).unwrap();
    }

    /// CTS-STATE-POSIX-4 (RFC-0020.7): atomic rename — source removed,
    /// target added atomically.
    #[test]
    fn cts_posix_4_atomic_rename() {
        let mut proj = make_projection();
        proj.mkdir("/tmp", 1, 0o755).unwrap();

        let fd = proj.open("/tmp/a.txt", O_CREAT | O_WRONLY, 1).unwrap();
        proj.write(fd, b"content-a").unwrap();
        proj.close(fd).unwrap();

        let fd = proj.open("/tmp/b.txt", O_CREAT | O_WRONLY, 1).unwrap();
        proj.write(fd, b"content-b").unwrap();
        proj.close(fd).unwrap();

        // Rename a.txt -> c.txt (atomic).
        proj.rename("/tmp/a.txt", "/tmp/c.txt", 1).unwrap();

        // a.txt no longer exists.
        assert_eq!(proj.stat("/tmp/a.txt", 1), Err(PosixError::NotFound));
        // c.txt exists with the content.
        let meta = proj.stat("/tmp/c.txt", 1).unwrap();
        let fd = proj.open("/tmp/c.txt", O_RDONLY, 1).unwrap();
        let mut buf = [0u8; 16];
        let n = proj.read(fd, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"content-a");
        proj.close(fd).unwrap();
        let _ = meta;

        // b.txt still exists.
        let meta = proj.stat("/tmp/b.txt", 1).unwrap();
        let _ = meta;
    }

    /// CTS-STATE-POSIX-5 (RFC-0020.12 / RFC-0020.13): capability rights take
    /// precedence over POSIX mode bits. A file with mode 0o644 but no READ
    /// capability for the domain is denied.
    #[test]
    fn cts_posix_5_capability_precedence() {
        let mut proj = make_projection();
        proj.mkdir("/tmp", 1, 0o755).unwrap();

        // Create a file.
        let fd = proj.open("/tmp/secret.txt", O_CREAT | O_WRONLY, 1).unwrap();
        proj.write(fd, b"sensitive").unwrap();
        proj.close(fd).unwrap();

        // Domain 2 has no capability on this file, even though mode is 0o644.
        assert_eq!(
            proj.open("/tmp/secret.txt", O_RDONLY, 2),
            Err(PosixError::PermissionDenied)
        );

        // Grant READ to domain 2; access succeeds.
        let meta = proj.stat("/tmp/secret.txt", 1).unwrap();
        let tmp_meta = proj.stat("/tmp", 1).unwrap();
        let root_meta = proj.stat("/", 1).unwrap();
        proj.grant_capability(
            DomainId::new(2).unwrap(),
            root_meta.object_id,
            Rights::READ.union(Rights::EXECUTE),
        );
        proj.grant_capability(
            DomainId::new(2).unwrap(),
            tmp_meta.object_id,
            Rights::READ.union(Rights::EXECUTE),
        );
        proj.grant_capability(DomainId::new(2).unwrap(), meta.object_id, Rights::READ);
        let fd = proj.open("/tmp/secret.txt", O_RDONLY, 2).unwrap();
        let mut buf = [0u8; 16];
        let n = proj.read(fd, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"sensitive");
        proj.close(fd).unwrap();
    }

    /// CTS-STATE-POSIX-5: directory traversal requires READ|EXECUTE capability.
    #[test]
    fn cts_posix_5_traversal_capability() {
        let mut proj = make_projection();
        proj.mkdir("/tmp", 1, 0o755).unwrap();
        proj.mkdir("/tmp/sub", 1, 0o755).unwrap();

        // Domain 2 lacks EXECUTE on /tmp/sub -> traversal denied.
        assert_eq!(
            proj.open("/tmp/sub/inside.txt", O_RDONLY, 2),
            Err(PosixError::PermissionDenied)
        );
    }

    /// CTS-STATE-POSIX-1: close on invalid fd returns BadF.
    #[test]
    fn cts_posix_1_close_bad_fd() {
        let mut proj = make_projection();
        assert_eq!(proj.close(999), Err(PosixError::BadF));
    }

    /// CTS-STATE-POSIX-1: read on write-only fd returns PermissionDenied.
    #[test]
    fn cts_posix_1_read_write_only_fd() {
        let mut proj = make_projection();
        proj.mkdir("/tmp", 1, 0o755).unwrap();
        let fd = proj.open("/tmp/wo.txt", O_CREAT | O_WRONLY, 1).unwrap();
        assert_eq!(
            proj.read(fd, &mut [0; 4]),
            Err(PosixError::PermissionDenied)
        );
        proj.close(fd).unwrap();
    }
}
