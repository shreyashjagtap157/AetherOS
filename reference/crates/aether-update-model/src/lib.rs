//! Update/recovery model (host reference): A/B partition OTA updates.
//!
//! This is a bounded executable host model of the AetherOS update and
//! recovery subsystem. It provides A/B partition management, version
//! manifest verification, update application, and rollback. It is
//! deliberately NOT a production update system.
//!
//! Scope:
//! - Two partitions (A and B) with version tracking.
//! - Version manifest with SHA-256 hash (simulated).
//! - Update flow: download → verify → apply → reboot → confirm.
//! - Rollback on boot failure (max 3 retries).
//! - Recovery mode flag.
//!
//! Out of scope:
//! - Real cryptographic verification (SHA-256 is simulated).
//! - Network download, delta updates, compression.
//! - Secure boot, measured boot, attestation.
//! - Flash wear-leveling, bad block management.

#![allow(missing_docs)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::new_without_default)]
#![allow(clippy::missing_errors_doc)]

/// Maximum number of boot retries before rollback.
const MAX_BOOT_RETRIES: u32 = 3;

/// Partition identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Partition {
    A,
    B,
}

impl Partition {
    pub fn other(&self) -> Self {
        match self {
            Partition::A => Partition::B,
            Partition::B => Partition::A,
        }
    }
}

/// Partition state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PartitionState {
    /// Partition is active and booted successfully.
    Active,
    /// Partition is pending (newly applied, not yet confirmed).
    Pending,
    /// Partition is inactive / empty.
    Inactive,
    /// Partition is in recovery mode.
    Recovery,
}

/// Version manifest for an update.
#[derive(Clone, Debug)]
pub struct Manifest {
    pub version: String,
    pub hash: u64, // simulated SHA-256 (first 8 bytes)
    pub size: u64,
}

/// Update system state.
#[derive(Debug)]
pub struct UpdateSystem {
    /// Active partition (currently booted).
    pub active: Partition,
    /// State of partition A.
    pub state_a: PartitionState,
    /// State of partition B.
    pub state_b: PartitionState,
    /// Version of partition A.
    pub version_a: String,
    /// Version of partition B.
    pub version_b: String,
    /// Boot retry counter for the pending partition.
    pub boot_retries: u32,
    /// Whether recovery mode is active.
    pub recovery_mode: bool,
    /// Pending manifest (if an update is in progress).
    pub pending_manifest: Option<Manifest>,
    /// Whether an update is in progress.
    pub update_in_progress: bool,
}

impl UpdateSystem {
    pub fn new() -> Self {
        Self {
            active: Partition::A,
            state_a: PartitionState::Active,
            state_b: PartitionState::Inactive,
            version_a: "1.0.0".to_string(),
            version_b: String::new(),
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

    fn version(&self, p: Partition) -> &str {
        match p {
            Partition::A => &self.version_a,
            Partition::B => &self.version_b,
        }
    }

    fn set_version(&mut self, p: Partition, v: String) {
        match p {
            Partition::A => self.version_a = v,
            Partition::B => self.version_b = v,
        }
    }

    /// Begins an update: verifies the manifest and prepares the inactive partition.
    pub fn begin_update(&mut self, manifest: Manifest) -> Result<(), &'static str> {
        if self.update_in_progress {
            return Err("update already in progress");
        }
        // Verify manifest (simulated hash check).
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

    /// Applies the update to the inactive partition.
    pub fn apply_update(&mut self) -> Result<(), &'static str> {
        if !self.update_in_progress {
            return Err("no update in progress");
        }
        let manifest = self
            .pending_manifest
            .as_ref()
            .ok_or("no pending manifest")?;
        let target = self.active.other();
        self.set_version(target, manifest.version.clone());
        self.boot_retries = 0;
        Ok(())
    }

    /// Confirms a successful boot on the pending partition.
    pub fn confirm_boot(&mut self) -> Result<(), &'static str> {
        let target = self.active.other();
        if self.state(target) != PartitionState::Pending {
            return Err("no pending partition to confirm");
        }
        // Swap: pending becomes active.
        self.set_state(self.active, PartitionState::Inactive);
        self.active = target;
        self.set_state(self.active, PartitionState::Active);
        self.update_in_progress = false;
        self.pending_manifest = None;
        self.boot_retries = 0;
        Ok(())
    }

    /// Records a boot failure on the pending partition. Rolls back after
    /// MAX_BOOT_RETRIES consecutive failures.
    pub fn record_boot_failure(&mut self) -> bool {
        self.boot_retries += 1;
        if self.boot_retries >= MAX_BOOT_RETRIES {
            // Rollback: reactivate old partition.
            let old = self.active;
            let failed = self.active.other();
            self.set_state(failed, PartitionState::Inactive);
            self.set_state(old, PartitionState::Active);
            self.update_in_progress = false;
            self.pending_manifest = None;
            self.boot_retries = 0;
            true // rolled back
        } else {
            false // retry
        }
    }

    /// Enters recovery mode.
    pub fn enter_recovery(&mut self) {
        self.recovery_mode = true;
        self.set_state(self.active, PartitionState::Recovery);
    }

    /// Exits recovery mode.
    pub fn exit_recovery(&mut self) {
        self.recovery_mode = false;
        self.set_state(self.active, PartitionState::Active);
    }

    /// Returns the active version string.
    pub fn active_version(&self) -> &str {
        self.version(self.active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let sys = UpdateSystem::new();
        assert_eq!(sys.active, Partition::A);
        assert_eq!(sys.state_a, PartitionState::Active);
        assert_eq!(sys.state_b, PartitionState::Inactive);
        assert_eq!(sys.active_version(), "1.0.0");
        assert!(!sys.update_in_progress);
    }

    #[test]
    fn begin_update() {
        let mut sys = UpdateSystem::new();
        let m = Manifest {
            version: "2.0.0".to_string(),
            hash: 0xDEAD,
            size: 1024,
        };
        sys.begin_update(m).unwrap();
        assert!(sys.update_in_progress);
        assert_eq!(sys.state_b, PartitionState::Pending);
    }

    #[test]
    fn begin_update_invalid_hash() {
        let mut sys = UpdateSystem::new();
        let m = Manifest {
            version: "2.0.0".to_string(),
            hash: 0,
            size: 1024,
        };
        assert!(sys.begin_update(m).is_err());
    }

    #[test]
    fn apply_and_confirm() {
        let mut sys = UpdateSystem::new();
        let m = Manifest {
            version: "2.0.0".to_string(),
            hash: 0xDEAD,
            size: 1024,
        };
        sys.begin_update(m).unwrap();
        sys.apply_update().unwrap();
        assert_eq!(sys.version_b, "2.0.0");
        sys.confirm_boot().unwrap();
        assert_eq!(sys.active, Partition::B);
        assert_eq!(sys.active_version(), "2.0.0");
        assert!(!sys.update_in_progress);
    }

    #[test]
    fn rollback_after_retries() {
        let mut sys = UpdateSystem::new();
        let m = Manifest {
            version: "2.0.0".to_string(),
            hash: 0xDEAD,
            size: 1024,
        };
        sys.begin_update(m).unwrap();
        sys.apply_update().unwrap();
        // Simulate 3 boot failures.
        assert!(!sys.record_boot_failure()); // retry 1
        assert!(!sys.record_boot_failure()); // retry 2
        assert!(sys.record_boot_failure()); // retry 3 -> rollback
        assert_eq!(sys.active, Partition::A);
        assert_eq!(sys.active_version(), "1.0.0");
        assert!(!sys.update_in_progress);
    }

    #[test]
    fn recovery_mode() {
        let mut sys = UpdateSystem::new();
        sys.enter_recovery();
        assert!(sys.recovery_mode);
        sys.exit_recovery();
        assert!(!sys.recovery_mode);
    }

    #[test]
    fn begin_update_invalid_size() {
        let mut sys = UpdateSystem::new();
        let m = Manifest {
            version: "2.0.0".to_string(),
            hash: 0xDEAD,
            size: 0,
        };
        assert!(sys.begin_update(m).is_err());
    }

    #[test]
    fn apply_update_without_begin_fails() {
        let mut sys = UpdateSystem::new();
        assert!(sys.apply_update().is_err());
        assert!(!sys.update_in_progress);
    }

    #[test]
    fn confirm_boot_without_pending_fails() {
        let mut sys = UpdateSystem::new();
        assert!(sys.confirm_boot().is_err());
        assert!(!sys.update_in_progress);
    }

    #[test]
    fn state_preserved_after_rollback() {
        let mut sys = UpdateSystem::new();
        let m = Manifest {
            version: "2.0.0".to_string(),
            hash: 0xDEAD,
            size: 1024,
        };
        sys.begin_update(m).unwrap();
        sys.apply_update().unwrap();
        assert_eq!(sys.state_b, PartitionState::Pending);
        assert_eq!(sys.version_b, "2.0.0");

        assert!(!sys.record_boot_failure());
        assert!(!sys.record_boot_failure());
        assert!(sys.record_boot_failure());

        assert_eq!(sys.active, Partition::A);
        assert_eq!(sys.state_a, PartitionState::Active);
        assert_eq!(sys.state_b, PartitionState::Inactive);
        assert_eq!(sys.active_version(), "1.0.0");
        assert!(!sys.update_in_progress);
        assert!(sys.pending_manifest.is_none());
    }

    #[test]
    fn state_preserved_after_confirm() {
        let mut sys = UpdateSystem::new();
        let m = Manifest {
            version: "2.0.0".to_string(),
            hash: 0xDEAD,
            size: 1024,
        };
        sys.begin_update(m).unwrap();
        sys.apply_update().unwrap();
        sys.confirm_boot().unwrap();

        assert_eq!(sys.active, Partition::B);
        assert_eq!(sys.state_a, PartitionState::Inactive);
        assert_eq!(sys.state_b, PartitionState::Active);
        assert_eq!(sys.active_version(), "2.0.0");
        assert!(!sys.update_in_progress);
        assert!(sys.pending_manifest.is_none());
    }

    #[test]
    fn double_update_fails() {
        let mut sys = UpdateSystem::new();
        let m = Manifest {
            version: "2.0.0".to_string(),
            hash: 0xDEAD,
            size: 1024,
        };
        sys.begin_update(m.clone()).unwrap();
        assert!(sys.begin_update(m).is_err());
    }
}
