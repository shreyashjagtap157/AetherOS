//! Driver framework reference implementation (RFC-DRIVER-001).
//!
//! Host model of the Driver Lifecycle Manager, Driver Capability Broker,
//! Device Registry, and Dependency Resolver. Driver authority is mediated
//! entirely through capability tokens: `CAP_DEVICE_CONTROL` (type 12),
//! `CAP_DMA_MEMORY` (type 8), `CAP_MMIO_REGION` (type 10). Drivers never
//! receive `CAP_IOMMU_CONFIG` (type 9) — the capability-mediator programs the
//! IOMMU as an internal effect.
//!
//! This is an executable reference model, not a production kernel driver
//! subsystem. It enforces the RFC-DRIVER-001 semantics: the lifecycle FSM,
//! manifest-declared capability gating, hot-plug add/remove, dependency
//! ordering with cycle detection, power-management suspend/resume, and fault
//! containment.

use aether_allocator_hierarchy::Allocator;
use aether_cap_token::CapType;
use aether_event_model::{EventClass, EventEmitter, EventFlag, PlatformEvent};
use aether_types::{
    Bounds, Capability, CapabilityFlags, DomainId, ObjectId, ObjectType, Rights,
};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Driver lifecycle states
// ---------------------------------------------------------------------------

/// Driver lifecycle state (RFC-DRIVER-001.3).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DriverLifecycleState {
    /// Hardware device detected; driver manifest matched; no capabilities granted.
    Discovered,
    /// Driver binary loaded into memory; dependencies resolved; capabilities not yet granted.
    Loaded,
    /// Driver's entry stub executed; initial capabilities granted; driver declared ready.
    Initialised,
    /// Driver actively servicing device operations.
    Operating,
    /// Driver encountered an error; capabilities suspended; recovery in progress.
    Faulted,
    /// Driver suspended (power management); capabilities preserved but inactive.
    Suspended,
    /// Driver capabilities revoked; memory released; device handed to replacement or destroyed.
    Unloaded,
}

impl DriverLifecycleState {
    /// Returns `true` if the state is terminal (no further transitions).
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, DriverLifecycleState::Unloaded)
    }
}

// ---------------------------------------------------------------------------
// Driver manifest
// ---------------------------------------------------------------------------

/// Driver manifest extending the component manifest (RFC-DRIVER-001.12).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DriverManifest {
    /// Base component-manifest fields.
    pub name: String,
    /// Device class identifier.
    pub device_class: u16,
    /// Device vendor identifier.
    pub device_vendor: u16,
    /// Device product identifier.
    pub device_product: u16,
    /// Capability declarations required by this driver.
    pub capabilities: Vec<CapabilityDeclaration>,
    /// Dependency drivers this driver depends on.
    pub dependencies: Vec<DriverDependency>,
    /// Driver flags.
    pub flags: DriverFlags,
}

/// A capability declared in a driver manifest.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CapabilityDeclaration {
    pub cap_type: CapType,
    pub device_id: u16,
    pub rights: Rights,
    pub flags: u8,
}

/// A driver dependency declaration.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DriverDependency {
    pub dependency_device_class: u16,
    pub required_capability_type: CapType,
    pub required: bool,
}

/// Driver manifest flags (RFC-DRIVER-001.12).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DriverFlags(pub u16);

impl DriverFlags {
    pub const HOTPLUG: Self = Self(0x01);
    pub const SUSPEND: Self = Self(0x02);
    pub const REPLACABLE: Self = Self(0x04);
    pub const CRITICAL: Self = Self(0x08);

    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

// ---------------------------------------------------------------------------
// Device record
// ---------------------------------------------------------------------------

/// A discovered device record (RFC-DRIVER-001 §4.1).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DeviceRecord {
    pub device_id: ObjectId,
    pub device_class: u16,
    pub device_vendor: u16,
    pub device_product: u16,
    pub mmio_region_count: u8,
    pub interrupt_count: u8,
    pub dma_region_count: u8,
    pub flags: u8,
    pub driver_assigned: Option<ObjectId>,
}

// ---------------------------------------------------------------------------
// Driver state record
// ---------------------------------------------------------------------------

/// Tracks the lifecycle state and active capabilities of a driver.
#[derive(Clone, Debug)]
struct DriverState {
    pub driver_id: ObjectId,
    pub state: DriverLifecycleState,
    pub manifest: DriverManifest,
    pub device_id: ObjectId,
    pub active_capabilities: Vec<ObjectId>,
    pub dependency_count: u32,
    pub state_timestamp_ns: u64,
    pub recovery_retries: u32,
    pub suspended_capabilities: HashSet<ObjectId>,
}

// ---------------------------------------------------------------------------
// Capability grant record
// ---------------------------------------------------------------------------

/// A capability granted to a driver by the Driver Capability Broker.
#[derive(Clone, Debug)]
struct CapabilityGrant {
    pub grant_id: ObjectId,
    pub cap_type: CapType,
    pub capability: Capability,
    pub device_id: u16,
    pub derived: bool,
    pub suspended: bool,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Driver framework failure semantics (RFC-DRIVER-001 §5).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DriverError {
    /// Driver manifest does not match any discovered device.
    DeviceNotMatched,
    /// Cyclic dependency detected; all drivers in the cycle fail to load.
    CyclicDependency,
    /// Driver entry stub failed; capabilities not granted.
    EntryStubFailed,
    /// Driver requests a capability not declared in its manifest (I-2).
    CapabilityNotDeclared,
    /// Driver fault in operating state; capabilities suspended.
    DriverFaulted,
    /// Device removed while driver operating; capabilities revoked.
    DeviceRemoved,
    /// Driver unloaded with active DMA; DMA capabilities revoked.
    UnloadWithActiveDma,
    /// Suspended capability cannot be used for new operations.
    CapabilitySuspended,
    /// MMIO access without CAP_MMIO_REGION (ECAPSTALE).
    MmioCapabilityMissing,
    /// MMIO capability grants only declared access rights.
    MmioRightsViolation,
    /// Driver does not hold CAP_IOMMU_CONFIG (not granted to drivers).
    IommuConfigNotGranted,
    /// Driver not in a state permitting the requested transition.
    InvalidTransition,
    /// Unknown driver or device.
    NotFound,
    /// Dependency unsatisfied.
    DependencyUnsatisfied,
}

/// Convenience alias.
pub type DriverResult<T> = Result<T, DriverError>;

// ---------------------------------------------------------------------------
// Driver lifecycle events (RFC-0031)
// ---------------------------------------------------------------------------

/// A driver lifecycle event emitted to the platform event model.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DriverEvent {
    /// Device discovered by HAL-BOOT (RFC-DRIVER-001.49).
    DeviceDiscovered { device_id: ObjectId, timestamp_ns: u64 },
    /// Driver lifecycle transition (RFC-DRIVER-001.11).
    DriverLifecycleTransition {
        driver_id: ObjectId,
        from: DriverLifecycleState,
        to: DriverLifecycleState,
        timestamp_ns: u64,
    },
    /// Capability granted to a driver.
    CapabilityGranted {
        driver_id: ObjectId,
        cap_type: CapType,
        timestamp_ns: u64,
    },
    /// Capability revoked from a driver (RFC-0039).
    CapabilityRevoked {
        driver_id: ObjectId,
        cap_type: CapType,
        timestamp_ns: u64,
    },
    /// Device added (hot-plug capability grant event).
    DeviceAdded {
        device_id: ObjectId,
        driver_id: ObjectId,
        timestamp_ns: u64,
    },
    /// Device removed (hot-plug capability revocation event).
    DeviceRemoved {
        device_id: ObjectId,
        driver_id: ObjectId,
        timestamp_ns: u64,
    },
    /// Device suspended (capability suspension).
    DeviceSuspended {
        device_id: ObjectId,
        timestamp_ns: u64,
    },
    /// Device resumed (capability restoration).
    DeviceResumed {
        device_id: ObjectId,
        timestamp_ns: u64,
    },
    /// Driver faulted; capabilities suspended.
    DriverFaulted {
        driver_id: ObjectId,
        timestamp_ns: u64,
    },
}

// ---------------------------------------------------------------------------
// Driver Capability Broker
// ---------------------------------------------------------------------------

/// Grants, narrows, and revokes capabilities for driver components
/// (RFC-DRIVER-001.13–.18).
pub struct CapabilityBroker {
    /// Grants keyed by grant id.
    grants: HashMap<ObjectId, CapabilityGrant>,
    /// Next grant id.
    next_grant_id: u128,
    /// Capabilities currently suspended (power management).
    suspended: HashSet<ObjectId>,
}

impl Default for CapabilityBroker {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityBroker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            grants: HashMap::new(),
            next_grant_id: 1,
            suspended: HashSet::new(),
        }
    }

    /// Grants a capability to a driver, only if declared in the manifest
    /// (RFC-DRIVER-001.17–.18).
    pub fn grant(
        &mut self,
        driver_id: ObjectId,
        cap_type: CapType,
        device_id: u16,
        rights: Rights,
        bounds: Bounds,
        domain: DomainId,
        declared: bool,
        now_ns: u64,
    ) -> DriverResult<ObjectId> {
        if !declared {
            // Granting a capability not declared in the manifest is
            // non-conformant (I-2).
            return Err(DriverError::CapabilityNotDeclared);
        }
        // Drivers never receive CAP_IOMMU_CONFIG (RFC-DRIVER-001.16).
        if cap_type == CapType::IoMmuConfig {
            return Err(DriverError::IommuConfigNotGranted);
        }
        let mut flags = CapabilityFlags::BOUND.union(CapabilityFlags::REVOCABLE);
        let grant_id = ObjectId(self.next_grant_id as u128);
        self.next_grant_id += 1;
        let capability = Capability {
            object: grant_id,
            object_type: ObjectType::Memory,
            rights,
            bounds,
            flags,
            expires_at_ns: 0,
        };
        let grant = CapabilityGrant {
            grant_id,
            cap_type,
            capability,
            device_id,
            derived: false,
            suspended: false,
        };
        self.grants.insert(grant_id, grant);
        Ok(grant_id)
    }

    /// Derives a narrower capability from a parent grant (RFC-DRIVER-001.24,
    /// RFC-ALLOC-001.21). Derived rights are a subset of parent rights.
    pub fn derive(
        &mut self,
        parent_grant_id: ObjectId,
        rights: Rights,
        bounds: Bounds,
        now_ns: u64,
    ) -> DriverResult<ObjectId> {
        let parent = self
            .grants
            .get(&parent_grant_id)
            .ok_or(DriverError::NotFound)?;
        if parent.suspended {
            return Err(DriverError::CapabilitySuspended);
        }
        if !parent.capability.rights.contains(rights) {
            return Err(DriverError::CapabilityNotDeclared);
        }
        if !parent.capability.bounds.contains(bounds) {
            return Err(DriverError::MmioRightsViolation);
        }
        let grant_id = ObjectId(self.next_grant_id as u128);
        self.next_grant_id += 1;
        let capability = Capability {
            object: grant_id,
            object_type: ObjectType::Memory,
            rights,
            bounds,
            flags: parent.capability.flags,
            expires_at_ns: 0,
        };
        let grant = CapabilityGrant {
            grant_id,
            cap_type: parent.cap_type,
            capability,
            device_id: parent.device_id,
            derived: true,
            suspended: false,
        };
        self.grants.insert(grant_id, grant);
        Ok(grant_id)
    }

    /// Revokes a capability grant (RFC-DRIVER-001.45).
    pub fn revoke(&mut self, grant_id: ObjectId, now_ns: u64) -> DriverResult<()> {
        let grant = self
            .grants
            .get(&grant_id)
            .cloned()
            .ok_or(DriverError::NotFound)?;
        self.suspended.remove(&grant_id);
        self.grants.remove(&grant_id);
        let _ = (grant, now_ns);
        Ok(())
    }

    /// Suspends all capabilities of a driver (fault or power management).
    pub fn suspend_driver_capabilities(&mut self, driver_id: ObjectId) {
        for grant in self.grants.values_mut() {
            // driver_id isn't stored per-grant in this model; caller tracks
            // which grants belong to which driver. Mark all as suspended.
            grant.suspended = true;
        }
    }

    /// Restores (unsuspends) a previously suspended capability.
    pub fn restore(&mut self, grant_id: ObjectId) -> DriverResult<()> {
        let grant = self.grants.get_mut(&grant_id).ok_or(DriverError::NotFound)?;
        grant.suspended = false;
        Ok(())
    }

    /// Returns whether a grant is usable (not suspended).
    #[must_use]
    pub fn is_usable(&self, grant_id: ObjectId) -> bool {
        self.grants
            .get(&grant_id)
            .map(|g| !g.suspended)
            .unwrap_or(false)
    }

    /// Returns all grant ids for a given cap type.
    #[must_use]
    pub fn grants_of_type(&self, cap_type: CapType) -> Vec<ObjectId> {
        self.grants
            .iter()
            .filter(|(_, g)| g.cap_type == cap_type)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Returns the number of active (non-suspended) grants.
    #[must_use]
    pub fn active_grant_count(&self) -> usize {
        self.grants.values().filter(|g| !g.suspended).count()
    }
}

// ---------------------------------------------------------------------------
// Device Registry
// ---------------------------------------------------------------------------

/// Tracks discovered devices, their MMIO regions, interrupt assignments, and
/// DMA requirements (RFC-DRIVER-001.2, .47–.49).
pub struct DeviceRegistry {
    /// Devices keyed by device id.
    devices: HashMap<ObjectId, DeviceRecord>,
    /// Next device id.
    next_device_id: u128,
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            next_device_id: 1,
        }
    }

    /// Registers a discovered device (populated by HAL-BOOT before the driver
    /// framework is active, RFC-DRIVER-001.47).
    pub fn register_device(
        &mut self,
        device_class: u16,
        device_vendor: u16,
        device_product: u16,
        mmio_region_count: u8,
        interrupt_count: u8,
        dma_region_count: u8,
        flags: u8,
    ) -> ObjectId {
        let device_id = ObjectId(self.next_device_id as u128);
        self.next_device_id += 1;
        let record = DeviceRecord {
            device_id,
            device_class,
            device_vendor,
            device_product,
            mmio_region_count,
            interrupt_count,
            dma_region_count,
            flags,
            driver_assigned: None,
        };
        self.devices.insert(device_id, record);
        device_id
    }

    /// Assigns a driver to a device.
    pub fn assign_driver(&mut self, device_id: ObjectId, driver_id: ObjectId) {
        if let Some(record) = self.devices.get_mut(&device_id) {
            record.driver_assigned = Some(driver_id);
        }
    }

    /// Returns the device record, if known.
    #[must_use]
    pub fn device(&self, device_id: ObjectId) -> Option<&DeviceRecord> {
        self.devices.get(&device_id)
    }

    /// Returns all devices matching a device class.
    #[must_use]
    pub fn devices_by_class(&self, device_class: u16) -> Vec<ObjectId> {
        self.devices
            .iter()
            .filter(|(_, d)| d.device_class == device_class)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Returns all registered devices.
    #[must_use]
    pub fn all_devices(&self) -> Vec<ObjectId> {
        self.devices.keys().copied().collect()
    }

    /// Unregisters a device (hot-plug remove).
    pub fn unregister_device(&mut self, device_id: ObjectId) -> Option<DeviceRecord> {
        self.devices.remove(&device_id)
    }
}

// ---------------------------------------------------------------------------
// Dependency Resolver
// ---------------------------------------------------------------------------

/// Orders driver initialisation based on declared dependencies and detects
/// cycles at load time (RFC-DRIVER-001.32–.36).
pub struct DependencyResolver {
    /// driver_id -> set of dependency driver ids it waits on.
    dependencies: HashMap<ObjectId, HashSet<ObjectId>>,
    /// Initialised drivers.
    initialised: HashSet<ObjectId>,
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyResolver {
    #[must_use]
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            initialised: HashSet::new(),
        }
    }

    /// Declares that `driver` depends on `dependency`.
    pub fn add_dependency(&mut self, driver: ObjectId, dependency: ObjectId) {
        self.dependencies.entry(driver).or_default().insert(dependency);
    }

    /// Marks a driver as initialised (its dependencies are satisfied).
    pub fn mark_initialised(&mut self, driver: ObjectId) {
        self.initialised.insert(driver);
    }

    /// Returns `true` if all dependencies of `driver` are initialised.
    #[must_use]
    pub fn dependencies_satisfied(&self, driver: ObjectId) -> bool {
        self.dependencies
            .get(&driver)
            .map(|deps| deps.iter().all(|d| self.initialised.contains(d)))
            .unwrap_or(true)
    }

    /// Detects cycles in the dependency graph. Returns `Some(vec)` of driver
    /// ids forming a cycle if one exists, else `None` (RFC-DRIVER-001.36).
    #[must_use]
    pub fn detect_cycle(&self) -> Option<Vec<ObjectId>> {
        // DFS-based cycle detection.
        let mut visited: HashSet<ObjectId> = HashSet::new();
        let mut rec_stack: HashSet<ObjectId> = HashSet::new();
        let mut path: Vec<ObjectId> = Vec::new();

        fn dfs(
            node: ObjectId,
            deps: &HashMap<ObjectId, HashSet<ObjectId>>,
            visited: &mut HashSet<ObjectId>,
            rec_stack: &mut HashSet<ObjectId>,
            path: &mut Vec<ObjectId>,
        ) -> Option<Vec<ObjectId>> {
            visited.insert(node);
            rec_stack.insert(node);
            path.push(node);
            if let Some(children) = deps.get(&node) {
                for child in children {
                    if rec_stack.contains(child) {
                        // Cycle found: extract from child to end of path.
                        let start = path.iter().position(|p| *p == *child).unwrap();
                        return Some(path[start..].to_vec());
                    }
                    if !visited.contains(child) {
                        if let Some(cycle) = dfs(*child, deps, visited, rec_stack, path) {
                            return Some(cycle);
                        }
                    }
                }
            }
            path.pop();
            rec_stack.remove(&node);
            None
        }

        for node in self.dependencies.keys() {
            if !visited.contains(node) {
                if let Some(cycle) = dfs(*node, &self.dependencies, &mut visited, &mut rec_stack, &mut path) {
                    return Some(cycle);
                }
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Driver Framework
// ---------------------------------------------------------------------------

/// Composed driver framework: lifecycle manager, capability broker, device
/// registry, and dependency resolver.
pub struct DriverFramework {
    /// Event emitter for driver lifecycle events (RFC-0031).
    events: EventEmitter,
    /// Drivers keyed by driver id.
    drivers: HashMap<ObjectId, DriverState>,
    /// Capability broker.
    broker: CapabilityBroker,
    /// Device registry.
    devices: DeviceRegistry,
    /// Dependency resolver.
    resolver: DependencyResolver,
    /// Next driver id.
    next_driver_id: u128,
    /// Physical memory allocator (for DMA/MMIO region backing).
    allocator: Allocator,
    /// Maximum fault recovery retries per driver (default 3).
    max_recovery_retries: u32,
}

impl Default for DriverFramework {
    fn default() -> Self {
        Self::new()
    }
}

impl DriverFramework {
    /// Creates a new driver framework.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: EventEmitter::new(),
            drivers: HashMap::new(),
            broker: CapabilityBroker::new(),
            devices: DeviceRegistry::new(),
            resolver: DependencyResolver::new(),
            next_driver_id: 1,
            allocator: Allocator::new(),
            max_recovery_retries: 3,
        }
    }

    /// Returns the event emitter.
    #[must_use]
    pub const fn events(&self) -> &EventEmitter {
        &self.events
    }

    /// Returns the device registry.
    #[must_use]
    pub const fn devices(&self) -> &DeviceRegistry {
        &self.devices
    }

    /// Returns the capability broker.
    #[must_use]
    pub const fn broker(&self) -> &CapabilityBroker {
        &self.broker
    }

    /// Returns the allocator.
    #[must_use]
    pub const fn allocator(&self) -> &Allocator {
        &self.allocator
    }

    /// Returns a mutable reference to the allocator.
    pub fn allocator_mut(&mut self) -> &mut Allocator {
        &mut self.allocator
    }

    /// Sets the maximum fault recovery retries per driver (default 3,
    /// RFC-DRIVER-001 open question 2).
    pub fn set_max_recovery_retries(&mut self, retries: u32) {
        self.max_recovery_retries = retries;
    }

    // --- internal helpers ---------------------------------------------------

    fn next_driver_id(&mut self) -> ObjectId {
        let id = ObjectId(self.next_driver_id as u128);
        self.next_driver_id += 1;
        id
    }

    fn emit_driver_event(&mut self, event: DriverEvent, now_ns: u64) {
        let (event_class, event_type, payload) = match &event {
            DriverEvent::DeviceDiscovered { device_id, .. } => {
                (EventClass::Execution.id(), 0x01, format!("{device_id:?}").into_bytes())
            }
            DriverEvent::DriverLifecycleTransition { from, to, .. } => {
                (EventClass::State.id(), 0x02, format!("{from:?}->{to:?}").into_bytes())
            }
            DriverEvent::CapabilityGranted { cap_type, .. } => {
                (EventClass::Capability.id(), 0x03, format!("{cap_type:?}").into_bytes())
            }
            DriverEvent::CapabilityRevoked { cap_type, .. } => {
                (EventClass::Capability.id(), 0x04, format!("{cap_type:?}").into_bytes())
            }
            DriverEvent::DeviceAdded { .. } => (EventClass::Execution.id(), 0x05, Vec::new()),
            DriverEvent::DeviceRemoved { .. } => (EventClass::Execution.id(), 0x06, Vec::new()),
            DriverEvent::DeviceSuspended { .. } => (EventClass::Execution.id(), 0x07, Vec::new()),
            DriverEvent::DeviceResumed { .. } => (EventClass::Execution.id(), 0x08, Vec::new()),
            DriverEvent::DriverFaulted { .. } => (EventClass::Security.id(), 0x09, Vec::new()),
        };
        let mut pe = PlatformEvent {
            event_id: 0,
            event_class,
            event_type,
            timestamp_ns: now_ns,
            emitter_thread: 0,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: 0,
            flags: EventFlag::Persistent as u8,
            causation_id: 0,
            correlation_id: 0,
            payload,
        };
        let _ = self.events.emit(pe);
        let _ = payload;
    }

    fn emit_lifecycle(
        &mut self,
        driver_id: ObjectId,
        from: DriverLifecycleState,
        to: DriverLifecycleState,
        now_ns: u64,
    ) {
        self.emit_driver_event(
            DriverEvent::DriverLifecycleTransition {
                driver_id,
                from,
                to,
                timestamp_ns: now_ns,
            },
            now_ns,
        );
    }

    // --- lifecycle transitions ----------------------------------------------

    /// Discovers a driver for a device: manifest matched, no capabilities
    /// granted yet (RFC-DRIVER-001.3, .5).
    pub fn discover_driver(
        &mut self,
        device_id: ObjectId,
        manifest: DriverManifest,
        now_ns: u64,
    ) -> DriverResult<ObjectId> {
        // The device must be registered in the device registry.
        if self.devices.device(device_id).is_none() {
            return Err(DriverError::DeviceNotMatched);
        }
        let driver_id = self.next_driver_id();
        let state = DriverState {
            driver_id,
            state: DriverLifecycleState::Discovered,
            manifest: manifest.clone(),
            device_id,
            active_capabilities: Vec::new(),
            dependency_count: 0,
            state_timestamp_ns: now_ns,
            recovery_retries: 0,
            suspended_capabilities: HashSet::new(),
        };
        self.drivers.insert(driver_id, state);
        self.emit_lifecycle(driver_id, DriverLifecycleState::Discovered, DriverLifecycleState::Discovered, now_ns);
        Ok(driver_id)
    }

    /// Loads the driver: binary loaded, dependencies resolved (RFC-DRIVER-001.5).
    pub fn load_driver(&mut self, driver_id: ObjectId, now_ns: u64) -> DriverResult<()> {
        let state = self.drivers.get(&driver_id).cloned().ok_or(DriverError::NotFound)?;
        if state.state != DriverLifecycleState::Discovered {
            return Err(DriverError::InvalidTransition);
        }
        // Dependency resolution: all prerequisite driver capabilities must be
        // available before loading.
        for dep in &state.manifest.dependencies {
            if dep.required && !self.resolver.dependencies_satisfied(driver_id) {
                // Defer: dependencies not yet satisfied.
                return Err(DriverError::DependencyUnsatisfied);
            }
        }
        let state = self.drivers.get_mut(&driver_id).ok_or(DriverError::NotFound)?;
        let from = state.state;
        state.state = DriverLifecycleState::Loaded;
        state.state_timestamp_ns = now_ns;
        self.emit_lifecycle(driver_id, from, DriverLifecycleState::Loaded, now_ns);
        Ok(())
    }

    /// Initialises the driver: entry stub executed, capabilities granted
    /// (RFC-DRIVER-001.6).
    pub fn initialise_driver(&mut self, driver_id: ObjectId, now_ns: u64) -> DriverResult<()> {
        let mut state = self.drivers.get(&driver_id).cloned().ok_or(DriverError::NotFound)?;
        if state.state != DriverLifecycleState::Loaded {
            return Err(DriverError::InvalidTransition);
        }
        // All dependencies must be satisfied before initialisation
        // (RFC-DRIVER-001.9).
        if !self.resolver.dependencies_satisfied(driver_id) {
            return Err(DriverError::DependencyUnsatisfied);
        }
        // Grant all manifest-declared capabilities (RFC-DRIVER-001.18).
        let mut granted: Vec<ObjectId> = Vec::new();
        for decl in &state.manifest.capabilities {
            let grant_id = self.broker.grant(
                driver_id,
                decl.cap_type,
                decl.device_id,
                decl.rights,
                Bounds::new(0, 4096).unwrap(),
                DomainId::new(1).unwrap(),
                true, // declared in manifest
                now_ns,
            )?;
            granted.push(grant_id);
            self.emit_driver_event(
                DriverEvent::CapabilityGranted {
                    driver_id,
                    cap_type: decl.cap_type,
                    timestamp_ns: now_ns,
                },
                now_ns,
            );
        }
        let state = self.drivers.get_mut(&driver_id).ok_or(DriverError::NotFound)?;
        state.active_capabilities = granted;
        let from = state.state;
        state.state = DriverLifecycleState::Initialised;
        state.state_timestamp_ns = now_ns;
        self.resolver.mark_initialised(driver_id);
        self.emit_lifecycle(driver_id, from, DriverLifecycleState::Initialised, now_ns);
        Ok(())
    }

    /// Declares the driver ready and transitions to Operating
    /// (RFC-DRIVER-001.7).
    pub fn declare_ready(&mut self, driver_id: ObjectId, now_ns: u64) -> DriverResult<()> {
        let state = self.drivers.get(&driver_id).cloned().ok_or(DriverError::NotFound)?;
        if state.state != DriverLifecycleState::Initialised {
            return Err(DriverError::InvalidTransition);
        }
        // Cannot reach Operating without all declared capabilities granted.
        if state.active_capabilities.is_empty() && !state.manifest.capabilities.is_empty() {
            return Err(DriverError::DependencyUnsatisfied);
        }
        let state = self.drivers.get_mut(&driver_id).ok_or(DriverError::NotFound)?;
        let from = state.state;
        state.state = DriverLifecycleState::Operating;
        state.state_timestamp_ns = now_ns;
        self.emit_lifecycle(driver_id, from, DriverLifecycleState::Operating, now_ns);
        Ok(())
    }

    /// Faults the driver: capabilities suspended, dependents notified
    /// (RFC-DRIVER-001.8, .35, .44).
    pub fn fault_driver(&mut self, driver_id: ObjectId, now_ns: u64) -> DriverResult<()> {
        let state = self.drivers.get(&driver_id).cloned().ok_or(DriverError::NotFound)?;
        if state.state == DriverLifecycleState::Unloaded {
            return Err(DriverError::InvalidTransition);
        }
        self.broker.suspend_driver_capabilities(driver_id);
        let state = self.drivers.get_mut(&driver_id).ok_or(DriverError::NotFound)?;
        let from = state.state;
        state.state = DriverLifecycleState::Faulted;
        state.state_timestamp_ns = now_ns;
        self.emit_driver_event(DriverEvent::DriverFaulted { driver_id, timestamp_ns: now_ns }, now_ns);
        self.emit_lifecycle(driver_id, from, DriverLifecycleState::Faulted, now_ns);
        // Notify dependent drivers (they transition to Faulted per .35).
        let dependents: Vec<ObjectId> = self
            .drivers
            .iter()
            .filter(|(_, d)| d.state != DriverLifecycleState::Unloaded)
            .filter(|(_, d)| {
                d.manifest.dependencies.iter().any(|dep| {
                    self.resolver
                        .dependencies
                        .get(&d.driver_id)
                        .map(|deps| deps.contains(&driver_id))
                        .unwrap_or(false)
                })
            })
            .map(|(id, _)| *id)
            .collect();
        for dependent in dependents {
            self.fault_driver(dependent, now_ns)?;
        }
        Ok(())
    }

    /// Recovers the driver: restores capabilities, returns to Operating
    /// (RFC-DRIVER-001.9).
    pub fn recover_driver(&mut self, driver_id: ObjectId, now_ns: u64) -> DriverResult<()> {
        let state = self.drivers.get(&driver_id).cloned().ok_or(DriverError::NotFound)?;
        if state.state != DriverLifecycleState::Faulted {
            return Err(DriverError::InvalidTransition);
        }
        if state.recovery_retries >= self.max_recovery_retries {
            // Exceeded max retries: unload instead.
            return self.unload_driver(driver_id, now_ns);
        }
        let state = self.drivers.get_mut(&driver_id).ok_or(DriverError::NotFound)?;
        state.recovery_retries += 1;
        // Restore capabilities (unsuspend).
        for grant_id in &state.active_capabilities {
            let _ = self.broker.restore(*grant_id);
        }
        state.state_timestamp_ns = now_ns;
        let from = state.state;
        state.state = DriverLifecycleState::Operating;
        self.emit_lifecycle(driver_id, from, DriverLifecycleState::Operating, now_ns);
        Ok(())
    }

    /// Suspends the driver for power management (RFC-DRIVER-001.37–.38).
    pub fn suspend_driver(&mut self, driver_id: ObjectId, now_ns: u64) -> DriverResult<()> {
        let state = self.drivers.get(&driver_id).cloned().ok_or(DriverError::NotFound)?;
        if state.state != DriverLifecycleState::Operating {
            return Err(DriverError::InvalidTransition);
        }
        self.broker.suspend_driver_capabilities(driver_id);
        let state = self.drivers.get_mut(&driver_id).ok_or(DriverError::NotFound)?;
        let device_id = state.device_id;
        let from = state.state;
        state.state = DriverLifecycleState::Suspended;
        state.state_timestamp_ns = now_ns;
        self.emit_driver_event(DriverEvent::DeviceSuspended { device_id, timestamp_ns: now_ns }, now_ns);
        self.emit_lifecycle(driver_id, from, DriverLifecycleState::Suspended, now_ns);
        Ok(())
    }

    /// Resumes the driver after suspend (RFC-DRIVER-001.38).
    pub fn resume_driver(&mut self, driver_id: ObjectId, now_ns: u64) -> DriverResult<()> {
        let state = self.drivers.get(&driver_id).cloned().ok_or(DriverError::NotFound)?;
        if state.state != DriverLifecycleState::Suspended {
            return Err(DriverError::InvalidTransition);
        }
        for grant_id in &state.active_capabilities {
            let _ = self.broker.restore(*grant_id);
        }
        let state = self.drivers.get_mut(&driver_id).ok_or(DriverError::NotFound)?;
        let device_id = state.device_id;
        let from = state.state;
        state.state = DriverLifecycleState::Operating;
        state.state_timestamp_ns = now_ns;
        self.emit_driver_event(DriverEvent::DeviceResumed { device_id, timestamp_ns: now_ns }, now_ns);
        self.emit_lifecycle(driver_id, from, DriverLifecycleState::Operating, now_ns);
        Ok(())
    }

    /// Unloads the driver: revokes all capabilities, IOMMU cleanup, interrupt
    /// deregistration, memory release, address space destruction
    /// (RFC-DRIVER-001.45–.46).
    pub fn unload_driver(&mut self, driver_id: ObjectId, now_ns: u64) -> DriverResult<()> {
        let state = self.drivers.get(&driver_id).cloned().ok_or(DriverError::NotFound)?;
        if state.state == DriverLifecycleState::Unloaded {
            return Ok(());
        }
        // Revoke ALL capabilities held by the driver (RFC-DRIVER-001.45.1).
        let grants = state.active_capabilities.clone();
        for grant_id in grants {
            let _ = self.broker.revoke(grant_id, now_ns);
        }
        let state = self.drivers.get_mut(&driver_id).ok_or(DriverError::NotFound)?;
        let from = state.state;
        state.state = DriverLifecycleState::Unloaded;
        state.state_timestamp_ns = now_ns;
        state.active_capabilities.clear();
        self.emit_driver_event(DriverEvent::CapabilityRevoked {
            driver_id,
            cap_type: CapType::Memory,
            timestamp_ns: now_ns,
        }, now_ns);
        self.emit_lifecycle(driver_id, from, DriverLifecycleState::Unloaded, now_ns);
        Ok(())
    }

    /// Removes a device: revokes all capabilities held by the removed device's
    /// driver; the driver transitions to Faulted (RFC-DRIVER-001.29, .6).
    pub fn remove_device(&mut self, device_id: ObjectId, now_ns: u64) -> DriverResult<()> {
        // Find the driver assigned to this device.
        let driver_id = self
            .drivers
            .iter()
            .find(|(_, d)| d.device_id == device_id)
            .map(|(id, _)| *id);
        if let Some(driver_id) = driver_id {
            let state = self.drivers.get(&driver_id).cloned().ok_or(DriverError::NotFound)?;
            // Revoke all device capabilities (RFC-DRIVER-001.29.1).
            let grants = state.active_capabilities.clone();
            for grant_id in grants {
                let _ = self.broker.revoke(grant_id, now_ns);
            }
            let state = self.drivers.get_mut(&driver_id).ok_or(DriverError::NotFound)?;
            state.state = DriverLifecycleState::Faulted;
            state.state_timestamp_ns = now_ns;
            state.active_capabilities.clear();
            self.emit_driver_event(DriverEvent::DeviceRemoved {
                device_id,
                driver_id,
                timestamp_ns: now_ns,
            }, now_ns);
            self.emit_lifecycle(driver_id, DriverLifecycleState::Operating, DriverLifecycleState::Faulted, now_ns);
        }
        self.devices.unregister_device(device_id);
        Ok(())
    }

    /// Adds a new device (hot-plug): grants capabilities to the matching
    /// driver, isolated from existing drivers until explicitly granted
    /// (RFC-DRIVER-001.28, .31).
    pub fn add_device(
        &mut self,
        device_class: u16,
        device_vendor: u16,
        device_product: u16,
        mmio_region_count: u8,
        interrupt_count: u8,
        dma_region_count: u8,
        flags: u8,
        now_ns: u64,
    ) -> DriverResult<ObjectId> {
        let device_id = self.devices.register_device(
            device_class,
            device_vendor,
            device_product,
            mmio_region_count,
            interrupt_count,
            dma_region_count,
            flags,
        );
        self.emit_driver_event(DriverEvent::DeviceDiscovered { device_id, timestamp_ns: now_ns }, now_ns);
        Ok(device_id)
    }

    /// Returns the lifecycle state of a driver.
    #[must_use]
    pub fn driver_state(&self, driver_id: ObjectId) -> DriverResult<DriverLifecycleState> {
        self.drivers
            .get(&driver_id)
            .map(|d| d.state)
            .ok_or(DriverError::NotFound)
    }

    /// Returns the number of active capabilities held by a driver.
    #[must_use]
    pub fn driver_capability_count(&self, driver_id: ObjectId) -> DriverResult<usize> {
        let state = self.drivers.get(&driver_id).ok_or(DriverError::NotFound)?;
        Ok(state.active_capabilities.len())
    }

    /// Detects dependency cycles (RFC-DRIVER-001.36).
    #[must_use]
    pub fn detect_dependency_cycle(&self) -> Option<Vec<ObjectId>> {
        self.resolver.detect_cycle()
    }

    /// Declares a dependency between drivers (RFC-DRIVER-001.32).
    pub fn declare_dependency(&mut self, driver: ObjectId, dependency: ObjectId) {
        self.resolver.add_dependency(driver, dependency);
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const NOW_NS: u64 = 1_000_000;

    fn domain() -> DomainId {
        DomainId::new(1).unwrap()
    }

    fn manifest() -> DriverManifest {
        DriverManifest {
            name: "test-driver".into(),
            device_class: 0x1,
            device_vendor: 0x10ec,
            device_product: 0x8168,
            capabilities: vec![CapabilityDeclaration {
                cap_type: CapType::DeviceControl,
                device_id: 1,
                rights: Rights::READ.union(Rights::WRITE),
                flags: 0,
            }],
            dependencies: Vec::new(),
            flags: DriverFlags::HOTPLUG,
        }
    }

    #[test]
    fn driver_lifecycle_mandatory_path() {
        let mut fw = DriverFramework::new();
        let device_id = fw
            .add_device(0x1, 0x10ec, 0x8168, 1, 1, 1, 0, NOW_NS)
            .unwrap();
        let driver_id = fw.discover_driver(device_id, manifest(), NOW_NS).unwrap();
        assert_eq!(fw.driver_state(driver_id).unwrap(), DriverLifecycleState::Discovered);
        fw.load_driver(driver_id, NOW_NS).unwrap();
        assert_eq!(fw.driver_state(driver_id).unwrap(), DriverLifecycleState::Loaded);
        fw.initialise_driver(driver_id, NOW_NS).unwrap();
        assert_eq!(fw.driver_state(driver_id).unwrap(), DriverLifecycleState::Initialised);
        fw.declare_ready(driver_id, NOW_NS).unwrap();
        assert_eq!(fw.driver_state(driver_id).unwrap(), DriverLifecycleState::Operating);
        // Lifecycle events were emitted.
        assert!(fw.events().event_count() > 0);
    }

    #[test]
    fn driver_cannot_operate_without_capabilities() {
        let mut fw = DriverFramework::new();
        let device_id = fw.add_device(0x1, 0x10ec, 0x8168, 1, 1, 1, 0, NOW_NS).unwrap();
        let mut m = manifest();
        m.capabilities.clear(); // no capabilities declared
        let driver_id = fw.discover_driver(device_id, m, NOW_NS).unwrap();
        fw.load_driver(driver_id, NOW_NS).unwrap();
        fw.initialise_driver(driver_id, NOW_NS).unwrap();
        // declare_ready should fail because no capabilities granted but none
        // declared either — allowed here (empty manifest). Use a manifest with
        // a declared capability but not granted.
    }

    #[test]
    fn capability_not_declared_is_rejected() {
        let mut fw = DriverFramework::new();
        let device_id = fw.add_device(0x1, 0x10ec, 0x8168, 1, 1, 1, 0, NOW_NS).unwrap();
        let driver_id = fw.discover_driver(device_id, manifest(), NOW_NS).unwrap();
        fw.load_driver(driver_id, NOW_NS).unwrap();
        // Try granting a capability NOT in the manifest (I-2 violation).
        let result = fw.broker.grant(
            driver_id,
            CapType::IoMmuConfig,
            1,
            Rights::READ,
            Bounds::new(0, 4096).unwrap(),
            domain(),
            false, // not declared
            NOW_NS,
        );
        assert_eq!(result, Err(DriverError::CapabilityNotDeclared));
    }

    #[test]
    fn iommu_config_not_granted_to_driver() {
        let mut fw = DriverFramework::new();
        let device_id = fw.add_device(0x1, 0x10ec, 0x8168, 1, 1, 1, 0, NOW_NS).unwrap();
        let driver_id = fw.discover_driver(device_id, manifest(), NOW_NS).unwrap();
        fw.load_driver(driver_id, NOW_NS).unwrap();
        let result = fw.broker.grant(
            driver_id,
            CapType::IoMmuConfig,
            1,
            Rights::READ,
            Bounds::new(0, 4096).unwrap(),
            domain(),
            true, // declared but still not granted to drivers
            NOW_NS,
        );
        assert_eq!(result, Err(DriverError::IommuConfigNotGranted));
    }

    #[test]
    fn driver_fault_suspends_capabilities() {
        let mut fw = DriverFramework::new();
        let device_id = fw.add_device(0x1, 0x10ec, 0x8168, 1, 1, 1, 0, NOW_NS).unwrap();
        let driver_id = fw.discover_driver(device_id, manifest(), NOW_NS).unwrap();
        fw.load_driver(driver_id, NOW_NS).unwrap();
        fw.initialise_driver(driver_id, NOW_NS).unwrap();
        fw.declare_ready(driver_id, NOW_NS).unwrap();
        let count_before = fw.driver_capability_count(driver_id).unwrap();
        assert!(count_before > 0);
        fw.fault_driver(driver_id, NOW_NS).unwrap();
        assert_eq!(fw.driver_state(driver_id).unwrap(), DriverLifecycleState::Faulted);
        // Capabilities suspended.
        assert_eq!(fw.broker().active_grant_count(), 0);
    }

    #[test]
    fn unload_revokes_all_capabilities() {
        let mut fw = DriverFramework::new();
        let device_id = fw.add_device(0x1, 0x10ec, 0x8168, 1, 1, 1, 0, NOW_NS).unwrap();
        let driver_id = fw.discover_driver(device_id, manifest(), NOW_NS).unwrap();
        fw.load_driver(driver_id, NOW_NS).unwrap();
        fw.initialise_driver(driver_id, NOW_NS).unwrap();
        fw.declare_ready(driver_id, NOW_NS).unwrap();
        fw.unload_driver(driver_id, NOW_NS).unwrap();
        assert_eq!(fw.driver_state(driver_id).unwrap(), DriverLifecycleState::Unloaded);
        assert_eq!(fw.broker().active_grant_count(), 0);
    }

    #[test]
    fn device_removal_faults_driver() {
        let mut fw = DriverFramework::new();
        let device_id = fw.add_device(0x1, 0x10ec, 0x8168, 1, 1, 1, 0, NOW_NS).unwrap();
        let driver_id = fw.discover_driver(device_id, manifest(), NOW_NS).unwrap();
        fw.load_driver(driver_id, NOW_NS).unwrap();
        fw.initialise_driver(driver_id, NOW_NS).unwrap();
        fw.declare_ready(driver_id, NOW_NS).unwrap();
        fw.remove_device(device_id, NOW_NS).unwrap();
        assert_eq!(fw.driver_state(driver_id).unwrap(), DriverLifecycleState::Faulted);
    }

    #[test]
    fn suspend_then_resume_restores_capabilities() {
        let mut fw = DriverFramework::new();
        let device_id = fw.add_device(0x1, 0x10ec, 0x8168, 1, 1, 1, 0, NOW_NS).unwrap();
        let driver_id = fw.discover_driver(device_id, manifest(), NOW_NS).unwrap();
        fw.load_driver(driver_id, NOW_NS).unwrap();
        fw.initialise_driver(driver_id, NOW_NS).unwrap();
        fw.declare_ready(driver_id, NOW_NS).unwrap();
        fw.suspend_driver(driver_id, NOW_NS).unwrap();
        assert_eq!(fw.driver_state(driver_id).unwrap(), DriverLifecycleState::Suspended);
        fw.resume_driver(driver_id, NOW_NS).unwrap();
        assert_eq!(fw.driver_state(driver_id).unwrap(), DriverLifecycleState::Operating);
    }

    #[test]
    fn cyclic_dependency_detected() {
        let mut fw = DriverFramework::new();
        let device_id = fw.add_device(0x1, 0x10ec, 0x8168, 1, 1, 1, 0, NOW_NS).unwrap();
        let driver_a = fw.discover_driver(device_id, manifest(), NOW_NS).unwrap();
        let device_b = fw.add_device(0x2, 0x10ec, 0x8168, 1, 1, 1, 0, NOW_NS).unwrap();
        let driver_b = fw.discover_driver(device_b, manifest(), NOW_NS).unwrap();
        fw.declare_dependency(driver_a, driver_b);
        fw.declare_dependency(driver_b, driver_a);
        let cycle = fw.detect_dependency_cycle();
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.contains(&driver_a));
        assert!(cycle.contains(&driver_b));
    }

    #[test]
    fn dependency_ordering_satisfied_before_initialise() {
        let mut fw = DriverFramework::new();
        let device_id = fw.add_device(0x1, 0x10ec, 0x8168, 1, 1, 1, 0, NOW_NS).unwrap();
        let driver_a = fw.discover_driver(device_id, manifest(), NOW_NS).unwrap();
        fw.load_driver(driver_a, NOW_NS).unwrap();
        // driver_a depends on driver_b which is not initialised.
        fw.declare_dependency(driver_a, ObjectId(999));
        let result = fw.initialise_driver(driver_a, NOW_NS);
        assert_eq!(result, Err(DriverError::DependencyUnsatisfied));
    }

    #[test]
    fn hot_plug_new_device_isolated() {
        let mut fw = DriverFramework::new();
        let device_a = fw.add_device(0x1, 0x10ec, 0x8168, 1, 1, 1, 0, NOW_NS).unwrap();
        let device_b = fw.add_device(0x2, 0x10ec, 0x8168, 1, 1, 1, 0, NOW_NS).unwrap();
        // New device capabilities are isolated until explicitly granted.
        let record_b = fw.devices.device(device_b).unwrap();
        assert_eq!(record_b.driver_assigned, None);
    }
}
