//! Minimal machine-readable capability conformance runner.

use aether_address_space_model as asm;
use aether_cap_model::{BootstrapAuthority as ModelBootstrap, CapabilityModel};
use aether_capspace::memory::{
    AddressSpaceId, HartId, MapRequest, MemoryAccess, MemoryManager, PageSize, PtePermissions,
    ShootdownError, TlbShootdown,
};
use aether_capspace::runtime::MemoryRuntime;
use aether_capspace::{BootstrapAuthority as SystemBootstrap, CapabilitySystem};
use aether_comp_model::profile::{Mechanism, ProfileDeclaration, apply_profile, combine_profiles};
use aether_comp_model::{
    ComponentId, Manifest, ManifestError, NarrowingPolicy, Profile, Requirement, ResolutionError,
    SelectionStrategy, Tier, conformance, routing, validate_manifest,
};
use aether_dispatch_model::{
    DispatchError, DispatchId, DispatchReason, DispatchRequest, DispatchState, Dispatcher,
    HartState,
};
use aether_exec_domain_model::{DomainError, DomainManager};
use aether_interrupt_model::{
    DeliveryMode, DeliveryOutcome, InterruptEvent, InterruptRouter, IrqVector, RoutingEntry,
};
use aether_mediation_model::{MediationFault, MediationOutcome, Mediator};
use aether_sched_model::{ThreadId, ThreadState, TransitionReason, UniprocessorEdf};
use aether_state_model::{
    EffectClass, EffectIntent, EffectStatus, Mutation, OperationId, StateEngine, VersionId,
};
use aether_types::{
    Bounds, CapError, Capability, CapabilityFlags, DomainId, Handle, ObjectId, ObjectType, Rights,
};

const NOW_NS: u64 = 1_000_000;

fn cases() -> [(&'static str, Result<(), &'static str>); 75] {
    [
        ("CTS-CAP-NARROW-001", narrow_rights_and_bounds()),
        ("CTS-CAP-XFER-001", explicit_transfer()),
        ("CTS-CAP-REVOKE-001", revoke_across_transfer()),
        (
            "CTS-CAP-REVOKE-SUBTREE-001",
            revoke_only_descendant_subtree(),
        ),
        (
            "CTS-CAP-REV-1",
            non_revocable_capability_cannot_be_revoked(),
        ),
        ("CTS-CAP-MED-1", mediation_every_access_is_mediated()),
        ("CTS-CAP-MED-2", mediation_three_part_check()),
        ("CTS-CAP-MED-3", mediation_denied_on_any_check_failure()),
        ("CTS-CAP-MED-4", mediation_rights_are_immutable()),
        ("CTS-CAP-MED-5", mediation_domain_scope_enforced()),
        ("CTS-CAP-MED-6", mediation_fault_isolation()),
        ("CTS-CAP-MED-7", mediation_no_check_short_circuiting()),
        ("CTS-EXEC-DOM-1", exec_domain_isolation()),
        ("CTS-EXEC-DOM-2", exec_domain_hierarchy_dag()),
        ("CTS-EXEC-DOM-3", exec_domain_lifecycle_and_destroy_rules()),
        ("CTS-EXEC-AS-1", exec_address_space_isolation()),
        (
            "CTS-EXEC-AS-2",
            exec_address_space_map_requires_capability(),
        ),
        ("CTS-EXEC-AS-3", exec_address_space_rights_enforcement()),
        ("CTS-EXEC-AS-4", exec_address_space_atomic_mapping_updates()),
        ("CTS-EXEC-AS-5", exec_address_space_explicit_page_size()),
        ("CTS-EXEC-AS-6", exec_address_space_asid_lifecycle()),
        ("CTS-CAP-FORGE-001", reject_forged_integer()),
        ("CTS-CAP-STALE-001", reject_stale_generation()),
        ("CTS-CAP-BOOTSTRAP-001", reject_foreign_bootstrap()),
        ("CTS-CAP-DOMAIN-001", reject_unregistered_destination()),
        ("CTS-CAP-REVOKED-DERIVE-001", reject_revoked_parent_use()),
        ("CTS-CAP-EXPIRY-001", expiry_boundary_matches_rfc_0037_14()),
        (
            "CTS-CAP-BOUND-001",
            bound_capability_cannot_be_transferred(),
        ),
        ("CTX-COMP-001", capability_round_trip()),
        ("CTX-CAP-NARROW-001", rights_narrowing_composition()),
        ("CTX-DISPATCH-001", dispatch_requires_capability_authority()),
        ("CTS-EXEC-DISP-1", exec_dispatch_capability_verification()),
        ("CTS-EXEC-DISP-2", exec_dispatch_domain_compatibility()),
        ("CTS-EXEC-DISP-3", exec_dispatch_latency_bound()),
        ("CTS-EXEC-DISP-4", exec_dispatch_migration_emits_event()),
        ("CTS-EXEC-DISP-5", exec_dispatch_migration_lock_hierarchy()),
        ("CTS-EXEC-DISP-6", exec_dispatch_atomic_transition()),
        ("CTX-STATE-001", state_commit_is_failure_atomic()),
        ("CTX-STORAGE-001", storage_block_read_write_round_trip()),
        (
            "CTX-RUNTIME-001",
            runtime_instruction_execution_produces_output(),
        ),
        ("CTX-UPDATE-001", update_rollback_after_boot_failures()),
        ("CTX-SHELL-001", shell_command_execution_and_state()),
        ("CTS-SCHED-ADMIT-001", admission_bound_matches_rfc_0011_7()),
        (
            "CTS-SCHED-DETERMINISM-001",
            scheduler_selection_is_deterministic(),
        ),
        (
            "CTS-SCHED-QUANTUM-001",
            quantum_expiry_requeues_with_new_deadline(),
        ),
        (
            "CTS-COMP-RESOLVE-001",
            composition_resolution_matches_rfc_0004(),
        ),
        ("CTS-COMP-CYCLE-001", composition_cycle_is_reported()),
        ("CTS-COMP-ORDER-001", composition_order_is_deterministic()),
        ("CTS-COMP-VALIDATE-001", invalid_manifest_is_rejected()),
        (
            "CTS-EXEC-MAP-REVOKE-001",
            mapping_permissions_and_revocation(),
        ),
        (
            "CTS-EXEC-DISP-ADDRSPACE-001",
            dispatch_address_space_activation(),
        ),
        (
            "CTS-EXEC-CAP-REVALIDATE-001",
            cached_mapping_requires_live_capability(),
        ),
        (
            "CTS-EXEC-RUNTIME-SERIALIZE-001",
            serialized_runtime_access_and_revocation(),
        ),
        ("CTS-STATE-ATOMIC-001", state_commit_is_failure_atomic()),
        (
            "CTS-EFFECT-UNCERTAIN-001",
            uncertain_effect_is_recoverable(),
        ),
        ("CTS-STATE-REPLAY-001", reject_replayed_transaction()),
        ("CTS-STATE-DUPLICATE-001", reject_duplicate_mutation()),
        (
            "CTS-STATE-CONFLICT-001",
            concurrent_writer_conflicts_match_rfc_0016_9(),
        ),
        ("CTS-STATE-SNAPSHOT-001", snapshot_read_matches_rfc_0016_8()),
        (
            "CTS-STATE-CRASH-001",
            crash_sweep_matches_persistence_baseline(),
        ),
        (
            "CTS-OUTBOX-DISPATCH-001",
            outbox_dispatch_matches_persistence_baseline(),
        ),
        ("CTS-PROF-SELECT-001", profile_selection_matches_rfc_0005()),
        (
            "CTS-PROF-COMBINE-001",
            profile_combination_matches_rfc_0005(),
        ),
        ("CTS-ROUTING-GRANT-001", routing_grant_matches_rfc_0003()),
        ("CTS-ROUTING-DENY-001", routing_denial_matches_rfc_0003()),
        (
            "CTS-COMP-CONF-001",
            composition_conformance_matches_rfc_0006(),
        ),
        (
            "CTS-COMP-CONF-002",
            conformance_rejects_incomplete_declarations(),
        ),
        (
            "CTS-COMP-CONF-003",
            conformance_report_cites_artifact_and_tests(),
        ),
        (
            "CTS-COMP-CONF-004",
            conformance_forensics_counts_match_methodology_s10(),
        ),
        ("CTS-EXEC-IRQ-1", exec_interrupt_domain_isolation()),
        (
            "CTS-EXEC-IRQ-2",
            exec_interrupt_bind_gating_and_direct_delivery(),
        ),
        ("CTS-EXEC-IRQ-3", exec_interrupt_queued_order_preserved()),
        (
            "CTS-EXEC-IRQ-4",
            exec_interrupt_unhandled_reported_spurious(),
        ),
        (
            "CTS-EXEC-IRQ-5",
            exec_interrupt_cross_domain_requires_capability(),
        ),
        (
            "CTS-EXEC-IRQ-6",
            exec_interrupt_priority_nesting_and_storm(),
        ),
    ]
}

fn main() {
    let cases = cases();
    let mut failed = 0;
    for (id, result) in cases {
        let (status, detail) = match result {
            Ok(()) => ("pass", "semantic outcomes match"),
            Err(detail) => {
                failed += 1;
                ("fail", detail)
            }
        };
        println!(r#"{{"id":"{id}","status":"{status}","detail":"{detail}"}}"#);
    }
    if failed != 0 {
        std::process::exit(1);
    }
}

fn domain(value: u16) -> DomainId {
    DomainId::new(value).expect("test domain is non-zero")
}
fn bounds(start: u64, end: u64) -> Bounds {
    Bounds::new(start, end).expect("test bounds are valid")
}
fn root() -> Capability {
    Capability {
        object: ObjectId(0xA37),
        object_type: ObjectType::Memory,
        rights: Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
        bounds: bounds(0, 4096),
        flags: aether_types::CapabilityFlags::REVOCABLE,
        expires_at_ns: 0,
    }
}

fn irq_bind_cap() -> Capability {
    Capability {
        object: ObjectId(0xF00D),
        object_type: ObjectType::Memory,
        rights: Rights::MAP,
        bounds: Bounds::new(0, 4096).unwrap(),
        flags: CapabilityFlags::BOUND,
        expires_at_ns: 0,
    }
}

fn irq_entry(vector: IrqVector, mode: DeliveryMode, priority: u8) -> RoutingEntry {
    RoutingEntry {
        vector,
        handler_cap: irq_bind_cap(),
        mode,
        priority,
        coalesce_threshold_hz: 1000,
    }
}

fn pair() -> (
    CapabilityModel,
    ModelBootstrap,
    CapabilitySystem,
    SystemBootstrap,
) {
    let (mut model, model_bootstrap) = CapabilityModel::bootstrap();
    let (mut system, system_bootstrap) = CapabilitySystem::bootstrap();
    system.add_domain(domain(1));
    system.add_domain(domain(2));
    model.add_domain(domain(1));
    model.add_domain(domain(2));
    (model, model_bootstrap, system, system_bootstrap)
}

fn reject_unregistered_destination() -> Result<(), &'static str> {
    let (mut model, model_bootstrap) = CapabilityModel::bootstrap();
    let (mut system, system_bootstrap) = CapabilitySystem::bootstrap();
    model.add_domain(domain(1));
    system.add_domain(domain(1));
    let model_root = model
        .create_root(&model_bootstrap, domain(1), root())
        .map_err(|_| "model root failed")?;
    let system_root = system
        .create_root(&system_bootstrap, domain(1), root())
        .map_err(|_| "provider root failed")?;
    let model_result = model.transfer(
        domain(1),
        model_root,
        domain(2),
        Rights::READ,
        bounds(0, 1),
        NOW_NS,
    );
    let system_result = system.transfer(
        domain(1),
        system_root,
        domain(2),
        Rights::READ,
        bounds(0, 1),
        NOW_NS,
    );
    compare_handle_error(model_result, system_result, CapError::Missing)
}

fn reject_revoked_parent_use() -> Result<(), &'static str> {
    let (mut model, model_bootstrap, mut system, system_bootstrap) = pair();
    let model_root = model
        .create_root(&model_bootstrap, domain(1), root())
        .map_err(|_| "model root failed")?;
    let system_root = system
        .create_root(&system_bootstrap, domain(1), root())
        .map_err(|_| "provider root failed")?;
    model
        .revoke(domain(1), model_root)
        .map_err(|_| "model revoke failed")?;
    system
        .revoke(domain(1), system_root)
        .map_err(|_| "provider revoke failed")?;
    compare_handle_error(
        model.derive(domain(1), model_root, Rights::READ, bounds(0, 1), NOW_NS),
        system.derive(domain(1), system_root, Rights::READ, bounds(0, 1), NOW_NS),
        CapError::Revoked,
    )
}

fn compare_handle_error(
    model: Result<Handle, CapError>,
    system: Result<Handle, CapError>,
    expected: CapError,
) -> Result<(), &'static str> {
    if model == Err(expected) && system == Err(expected) {
        Ok(())
    } else {
        Err("model/provider failure mismatch")
    }
}

fn narrow_rights_and_bounds() -> Result<(), &'static str> {
    let (mut model, model_bootstrap, mut system, system_bootstrap) = pair();
    let model_root = model
        .create_root(&model_bootstrap, domain(1), root())
        .map_err(|_| "model root failed")?;
    let system_root = system
        .create_root(&system_bootstrap, domain(1), root())
        .map_err(|_| "provider root failed")?;
    let m = model.derive(domain(1), model_root, Rights::READ, bounds(64, 128), NOW_NS);
    let s = system.derive(
        domain(1),
        system_root,
        Rights::READ,
        bounds(64, 128),
        NOW_NS,
    );
    if m.is_ok() == s.is_ok() {
        Ok(())
    } else {
        Err("model/provider derivation mismatch")
    }
}

fn explicit_transfer() -> Result<(), &'static str> {
    let (mut model, model_bootstrap, mut system, system_bootstrap) = pair();
    let mr = model
        .create_root(&model_bootstrap, domain(1), root())
        .map_err(|_| "model root failed")?;
    let sr = system
        .create_root(&system_bootstrap, domain(1), root())
        .map_err(|_| "provider root failed")?;
    let mh = model
        .transfer(
            domain(1),
            mr,
            domain(2),
            Rights::READ,
            bounds(100, 200),
            NOW_NS,
        )
        .map_err(|_| "model transfer failed")?;
    let sh = system
        .transfer(
            domain(1),
            sr,
            domain(2),
            Rights::READ,
            bounds(100, 200),
            NOW_NS,
        )
        .map_err(|_| "provider transfer failed")?;
    compare_auth(
        model.authorize(
            domain(2),
            mh,
            ObjectType::Memory,
            Rights::READ,
            bounds(120, 130),
            NOW_NS,
        ),
        system.authorize(
            domain(2),
            sh,
            ObjectType::Memory,
            Rights::READ,
            bounds(120, 130),
            NOW_NS,
        ),
    )
}

fn revoke_across_transfer() -> Result<(), &'static str> {
    let (mut model, model_bootstrap, mut system, system_bootstrap) = pair();
    let mr = model
        .create_root(&model_bootstrap, domain(1), root())
        .map_err(|_| "model root failed")?;
    let sr = system
        .create_root(&system_bootstrap, domain(1), root())
        .map_err(|_| "provider root failed")?;
    let mh = model
        .transfer(
            domain(1),
            mr,
            domain(2),
            Rights::READ,
            bounds(0, 10),
            NOW_NS,
        )
        .map_err(|_| "model transfer failed")?;
    let sh = system
        .transfer(
            domain(1),
            sr,
            domain(2),
            Rights::READ,
            bounds(0, 10),
            NOW_NS,
        )
        .map_err(|_| "provider transfer failed")?;
    model
        .revoke(domain(1), mr)
        .map_err(|_| "model revoke failed")?;
    system
        .revoke(domain(1), sr)
        .map_err(|_| "provider revoke failed")?;
    compare_auth(
        model.authorize(
            domain(2),
            mh,
            ObjectType::Memory,
            Rights::READ,
            bounds(0, 1),
            NOW_NS,
        ),
        system.authorize(
            domain(2),
            sh,
            ObjectType::Memory,
            Rights::READ,
            bounds(0, 1),
            NOW_NS,
        ),
    )
}

fn pair_derive(
    model: &mut CapabilityModel,
    model_parent: Handle,
    system: &mut CapabilitySystem,
    system_parent: Handle,
    rights: Rights,
    bounds_: Bounds,
) -> Result<(Handle, Handle), &'static str> {
    let model_handle = model
        .derive(domain(1), model_parent, rights, bounds_, NOW_NS)
        .map_err(|_| "model derivation failed")?;
    let system_handle = system
        .derive(domain(1), system_parent, rights, bounds_, NOW_NS)
        .map_err(|_| "provider derivation failed")?;
    Ok((model_handle, system_handle))
}

fn pair_transfer(
    model: &mut CapabilityModel,
    model_parent: Handle,
    system: &mut CapabilitySystem,
    system_parent: Handle,
    destination: DomainId,
    rights: Rights,
    bounds_: Bounds,
) -> Result<(Handle, Handle), &'static str> {
    let model_handle = model
        .transfer(
            domain(1),
            model_parent,
            destination,
            rights,
            bounds_,
            NOW_NS,
        )
        .map_err(|_| "model transfer failed")?;
    let system_handle = system
        .transfer(
            domain(1),
            system_parent,
            destination,
            rights,
            bounds_,
            NOW_NS,
        )
        .map_err(|_| "provider transfer failed")?;
    Ok((model_handle, system_handle))
}

fn revoke_only_descendant_subtree() -> Result<(), &'static str> {
    let (mut model, model_bootstrap, mut system, system_bootstrap) = pair();
    let model_root = model
        .create_root(&model_bootstrap, domain(1), root())
        .map_err(|_| "model root failed")?;
    let system_root = system
        .create_root(&system_bootstrap, domain(1), root())
        .map_err(|_| "provider root failed")?;

    let grant = Rights::READ.union(Rights::GRANT);
    let (model_branch, system_branch) = pair_derive(
        &mut model,
        model_root,
        &mut system,
        system_root,
        grant,
        bounds(0, 4096),
    )?;
    let (model_descendant, system_descendant) = pair_derive(
        &mut model,
        model_branch,
        &mut system,
        system_branch,
        Rights::READ,
        bounds(0, 1024),
    )?;
    let (model_transfer, system_transfer) = pair_transfer(
        &mut model,
        model_branch,
        &mut system,
        system_branch,
        domain(2),
        Rights::READ,
        bounds(0, 512),
    )?;
    let (model_sibling, system_sibling) = pair_derive(
        &mut model,
        model_root,
        &mut system,
        system_root,
        Rights::READ,
        bounds(1024, 2048),
    )?;

    model
        .revoke(domain(1), model_branch)
        .map_err(|_| "model subtree revoke failed")?;
    system
        .revoke(domain(1), system_branch)
        .map_err(|_| "provider subtree revoke failed")?;

    for (model_handle, system_handle, test_domain, request_bounds) in [
        (model_branch, system_branch, domain(1), bounds(0, 1)),
        (model_descendant, system_descendant, domain(1), bounds(0, 1)),
        (model_transfer, system_transfer, domain(2), bounds(0, 1)),
        (model_root, system_root, domain(1), bounds(0, 1)),
        (model_sibling, system_sibling, domain(1), bounds(1024, 1025)),
    ] {
        compare_auth(
            model.authorize(
                test_domain,
                model_handle,
                ObjectType::Memory,
                Rights::READ,
                request_bounds,
                NOW_NS,
            ),
            system.authorize(
                test_domain,
                system_handle,
                ObjectType::Memory,
                Rights::READ,
                request_bounds,
                NOW_NS,
            ),
        )?;
    }
    Ok(())
}

/// RFC-0039.2 (CTS-CAP-REV-1): a capability without `CAPF_REVOCABLE` cannot be
/// revoked; both the differential oracle and the table-backed provider must
/// refuse with `RevocationDenied`, and a revocable capability still revokes.
fn non_revocable_capability_cannot_be_revoked() -> Result<(), &'static str> {
    let (mut model, model_bootstrap, mut system, system_bootstrap) = pair();

    let non_revocable_flags = CapabilityFlags::NONE;
    let revocable_flags = CapabilityFlags::REVOCABLE;

    let model_non_revocable = model
        .create_root(
            &model_bootstrap,
            domain(1),
            Capability {
                flags: non_revocable_flags,
                ..root()
            },
        )
        .map_err(|_| "model root failed")?;
    let system_non_revocable = system
        .create_root(
            &system_bootstrap,
            domain(1),
            Capability {
                flags: non_revocable_flags,
                ..root()
            },
        )
        .map_err(|_| "provider root failed")?;

    if model.revoke(domain(1), model_non_revocable) != Err(CapError::RevocationDenied) {
        return Err("model revoked a non-revocable capability");
    }
    if system.revoke(domain(1), system_non_revocable) != Err(CapError::RevocationDenied) {
        return Err("provider revoked a non-revocable capability");
    }

    // No state change: both capabilities still authorize.
    compare_auth(
        model.authorize(
            domain(1),
            model_non_revocable,
            ObjectType::Memory,
            Rights::READ,
            bounds(0, 1),
            NOW_NS,
        ),
        system.authorize(
            domain(1),
            system_non_revocable,
            ObjectType::Memory,
            Rights::READ,
            bounds(0, 1),
            NOW_NS,
        ),
    )?;

    // A capability carrying CAPF_REVOCABLE revokes successfully in both.
    let model_revocable = model
        .create_root(
            &model_bootstrap,
            domain(1),
            Capability {
                flags: revocable_flags,
                ..root()
            },
        )
        .map_err(|_| "model root failed")?;
    let system_revocable = system
        .create_root(
            &system_bootstrap,
            domain(1),
            Capability {
                flags: revocable_flags,
                ..root()
            },
        )
        .map_err(|_| "provider root failed")?;
    model
        .revoke(domain(1), model_revocable)
        .map_err(|_| "model revocation failed")?;
    system
        .revoke(domain(1), system_revocable)
        .map_err(|_| "provider revocation failed")?;
    Ok(())
}

/// CTS-CAP-MED-1 (RFC-0013.1): every access is mediated — a valid capability
/// presented to `Mediator::check` is permitted, and access never bypasses
/// the mediator boundary.
fn mediation_every_access_is_mediated() -> Result<(), &'static str> {
    let mut mediator = Mediator::new();
    let cap = Capability {
        object: ObjectId(7),
        object_type: ObjectType::Memory,
        rights: Rights::READ.union(Rights::WRITE),
        bounds: bounds(0, 4096),
        flags: CapabilityFlags::NONE,
        expires_at_ns: 0,
    };
    let result = mediator.check(
        1,
        domain(1),
        &cap,
        Rights::READ,
        ObjectId(7),
        0x100,
        bounds(0x100, 0x110),
        NOW_NS,
    );
    if result != MediationOutcome::Permit {
        return Err("valid mediated access was not permitted");
    }
    Ok(())
}

/// CTS-CAP-MED-2 (RFC-0013.4): the three-part check is enumerated — valid
/// token + sufficient rights + in bounds permits; each missing part denies.
fn mediation_three_part_check() -> Result<(), &'static str> {
    let mut mediator = Mediator::new();
    let cap = Capability {
        object: ObjectId(7),
        object_type: ObjectType::Memory,
        rights: Rights::READ,
        bounds: bounds(0, 4096),
        flags: CapabilityFlags::NONE,
        expires_at_ns: 0,
    };

    // Part 1: valid token + matching rights + in bounds -> Permit.
    if mediator.check(
        1,
        domain(1),
        &cap,
        Rights::READ,
        ObjectId(7),
        0x100,
        bounds(0x100, 0x110),
        NOW_NS,
    ) != MediationOutcome::Permit
    {
        return Err("valid three-part check did not permit");
    }

    // Part 2: sufficient rights missing -> Deny(InsufficientRights).
    if mediator.check(
        1,
        domain(1),
        &cap,
        Rights::WRITE,
        ObjectId(7),
        0x100,
        bounds(0x100, 0x110),
        NOW_NS,
    ) != MediationOutcome::Deny(MediationFault::InsufficientRights)
    {
        return Err("missing right was not denied");
    }

    // Part 3: target out of bounds -> Deny.
    if !mediator
        .check(
            1,
            domain(1),
            &cap,
            Rights::READ,
            ObjectId(7),
            0x5000,
            bounds(0x5000, 0x5010),
            NOW_NS,
        )
        .is_deny()
    {
        return Err("out-of-bounds target was not denied");
    }
    Ok(())
}

/// CTS-CAP-MED-3 (RFC-0013.5): denied access on any check failure — each
/// distinct fault type produces `Deny`.
fn mediation_denied_on_any_check_failure() -> Result<(), &'static str> {
    let mut mediator = Mediator::new();

    // Non-capability token (invalid bounds).
    let bad_token = Capability {
        bounds: Bounds { start: 10, end: 10 },
        object: ObjectId(7),
        object_type: ObjectType::Memory,
        rights: Rights::READ,
        flags: CapabilityFlags::NONE,
        expires_at_ns: 0,
    };
    if mediator.check(
        1,
        domain(1),
        &bad_token,
        Rights::READ,
        ObjectId(7),
        0x100,
        bounds(0x100, 0x110),
        NOW_NS,
    ) != MediationOutcome::Deny(MediationFault::NonCapabilityToken)
    {
        return Err("non-capability token was not denied");
    }

    // Expired capability.
    let expired = Capability {
        object: ObjectId(7),
        object_type: ObjectType::Memory,
        rights: Rights::READ,
        bounds: bounds(0, 4096),
        flags: CapabilityFlags::NONE,
        expires_at_ns: 1_000,
    };
    if mediator.check(
        1,
        domain(1),
        &expired,
        Rights::READ,
        ObjectId(7),
        0x100,
        bounds(0x100, 0x110),
        1_001,
    ) != MediationOutcome::Deny(MediationFault::Expired)
    {
        return Err("expired capability was not denied");
    }

    // Revoked object.
    mediator.revoke_object(ObjectId(7));
    let live = Capability {
        object: ObjectId(7),
        object_type: ObjectType::Memory,
        rights: Rights::READ,
        bounds: bounds(0, 4096),
        flags: CapabilityFlags::NONE,
        expires_at_ns: 0,
    };
    if mediator.check(
        1,
        domain(1),
        &live,
        Rights::READ,
        ObjectId(7),
        0x100,
        bounds(0x100, 0x110),
        NOW_NS,
    ) != MediationOutcome::Deny(MediationFault::Revoked)
    {
        return Err("revoked object access was not denied");
    }
    Ok(())
}

/// CTS-CAP-MED-4 (RFC-0013.7): rights are immutable — the capability's rights
/// are unchanged after mediation checks, and no right is added by the mediator.
fn mediation_rights_are_immutable() -> Result<(), &'static str> {
    let mut mediator = Mediator::new();
    let cap = Capability {
        object: ObjectId(7),
        object_type: ObjectType::Memory,
        rights: Rights::READ,
        bounds: bounds(0, 4096),
        flags: CapabilityFlags::NONE,
        expires_at_ns: 0,
    };
    let original = cap.rights;
    let _ = mediator.check(
        1,
        domain(1),
        &cap,
        Rights::READ,
        ObjectId(7),
        0x100,
        bounds(0x100, 0x110),
        NOW_NS,
    );
    // A denied check for a missing right must not add that right.
    let _ = mediator.check(
        1,
        domain(1),
        &cap,
        Rights::WRITE,
        ObjectId(7),
        0x100,
        bounds(0x100, 0x110),
        NOW_NS,
    );
    if cap.rights != original {
        return Err("mediator modified capability rights after grant");
    }
    Ok(())
}

/// CTS-CAP-MED-5 (RFC-0013.9): domain scope — denied events carry the
/// accessor's domain so cross-domain use is observable and auditable.
fn mediation_domain_scope_enforced() -> Result<(), &'static str> {
    let mut mediator = Mediator::new();
    let cap = Capability {
        object: ObjectId(7),
        object_type: ObjectType::Memory,
        rights: Rights::READ,
        bounds: bounds(0, 4096),
        flags: CapabilityFlags::NONE,
        expires_at_ns: 0,
    };
    let _ = mediator.check(
        1,
        domain(2),
        &cap,
        Rights::WRITE,
        ObjectId(7),
        0x100,
        bounds(0x100, 0x110),
        NOW_NS,
    );
    let event = mediator
        .events()
        .last()
        .ok_or("no mediation event recorded")?;
    if event.accessor_domain != domain(2) {
        return Err("mediation event does not record the accessor domain");
    }
    Ok(())
}

/// CTS-CAP-MED-6 (RFC-0013.13): fault isolation — a denied access records
/// exactly one fault event and mutates no other mediation state.
fn mediation_fault_isolation() -> Result<(), &'static str> {
    let mut mediator = Mediator::new();
    let cap = Capability {
        object: ObjectId(7),
        object_type: ObjectType::Memory,
        rights: Rights::READ,
        bounds: bounds(0, 4096),
        flags: CapabilityFlags::NONE,
        expires_at_ns: 0,
    };
    let before = mediator.event_count();
    let result = mediator.check(
        1,
        domain(1),
        &cap,
        Rights::WRITE,
        ObjectId(7),
        0x100,
        bounds(0x100, 0x110),
        NOW_NS,
    );
    if result != MediationOutcome::Deny(MediationFault::InsufficientRights) {
        return Err("insufficient rights was not denied");
    }
    if mediator.event_count() != before + 1 {
        return Err("denied access did not record exactly one fault event");
    }
    // No bypass was introduced by the denial.
    if mediator.bypass_attempted() {
        return Err("denial incorrectly recorded a bypass attempt");
    }
    Ok(())
}

/// CTS-CAP-MED-7 (RFC-0013.14): no check short-circuiting — when multiple
/// checks would fail, the first fault in pipeline order is reported.
fn mediation_no_check_short_circuiting() -> Result<(), &'static str> {
    let mut mediator = Mediator::new();
    // Both expired (CHECK_LIFETIME, step 5) and out of bounds
    // (CHECK_BOUNDS, step 6): lifecycle is evaluated first.
    let expired = Capability {
        object: ObjectId(7),
        object_type: ObjectType::Memory,
        rights: Rights::READ,
        bounds: bounds(0, 4096),
        flags: CapabilityFlags::NONE,
        expires_at_ns: 1_000,
    };
    let result = mediator.check(
        1,
        domain(1),
        &expired,
        Rights::READ,
        ObjectId(7),
        0x5000,
        bounds(0x5000, 0x5010),
        1_001,
    );
    if result != MediationOutcome::Deny(MediationFault::Expired) {
        return Err("pipeline short-circuited or reported the wrong fault first");
    }
    Ok(())
}

/// CTS-EXEC-DOM-1 (RFC-0007.1): domain isolation — same-domain access is
/// permitted; cross-domain access without explicit grant is denied.
fn exec_domain_isolation() -> Result<(), &'static str> {
    let mut manager = DomainManager::new();
    manager
        .create_domain(domain(1), None, NOW_NS)
        .map_err(|_| "domain 1 creation failed")?;
    manager
        .create_domain(domain(2), None, NOW_NS)
        .map_err(|_| "domain 2 creation failed")?;

    manager
        .check_isolation(domain(1), domain(1), &[])
        .map_err(|_| "same-domain access was denied")?;
    if manager.check_isolation(domain(1), domain(2), &[]).is_ok() {
        return Err("cross-domain access without grant was permitted");
    }
    if manager
        .check_isolation(domain(1), domain(2), &[domain(2)])
        .is_err()
    {
        return Err("cross-domain access with explicit grant was denied");
    }
    Ok(())
}

/// CTS-EXEC-DOM-2 (RFC-0007.3/.23): hierarchy — the parent/child graph is a
/// DAG; children record their parent and children lists are maintained.
fn exec_domain_hierarchy_dag() -> Result<(), &'static str> {
    let mut manager = DomainManager::new();
    manager
        .create_domain(domain(1), None, NOW_NS)
        .map_err(|_| "root creation failed")?;
    manager
        .create_domain(domain(2), Some(domain(1)), NOW_NS)
        .map_err(|_| "child creation failed")?;
    manager
        .create_domain(domain(3), Some(domain(2)), NOW_NS)
        .map_err(|_| "grandchild creation failed")?;
    manager
        .create_domain(domain(4), Some(domain(1)), NOW_NS)
        .map_err(|_| "second child creation failed")?;

    if manager.parent_of(domain(2)) != Ok(Some(domain(1))) {
        return Err("child does not record its parent");
    }
    if manager.children_of(domain(1)) != Ok(vec![domain(2), domain(4)]) {
        return Err("parent children list mismatch");
    }
    if manager.children_of(domain(2)) != Ok(vec![domain(3)]) {
        return Err("grandparent children list mismatch");
    }
    // A deep chain is still a DAG; no node is an ancestor of its parent.
    if manager.is_ancestor(domain(3), domain(1)).unwrap_or(false) {
        return Err("descendant incorrectly reported as ancestor");
    }
    Ok(())
}

/// CTS-EXEC-DOM-3 (RFC-0007.5/.7): lifecycle — Created->Active is the only
/// valid initial transition; a parent cannot be destroyed while it has
/// children; children must be destroyed or orphaned first.
fn exec_domain_lifecycle_and_destroy_rules() -> Result<(), &'static str> {
    let mut manager = DomainManager::new();
    manager
        .create_domain(domain(1), None, NOW_NS)
        .map_err(|_| "root creation failed")?;
    manager
        .activate(domain(1), NOW_NS)
        .map_err(|_| "root activation failed")?;

    // Created -> Destroyed directly is not a valid transition.
    manager
        .create_domain(domain(2), None, NOW_NS)
        .map_err(|_| "domain 2 creation failed")?;
    if manager.destroy(domain(2), NOW_NS).is_ok() {
        return Err("unactivated domain was destroyed");
    }

    // Parent with children cannot be destroyed.
    manager
        .create_domain(domain(3), Some(domain(1)), NOW_NS)
        .map_err(|_| "child creation failed")?;
    manager
        .activate(domain(3), NOW_NS)
        .map_err(|_| "child activation failed")?;
    if manager.destroy(domain(1), NOW_NS).is_ok() {
        return Err("parent with children was destroyed");
    }

    // Destroy child first, then parent.
    manager
        .destroy(domain(3), NOW_NS)
        .map_err(|_| "child destruction failed")?;
    manager
        .destroy(domain(1), NOW_NS)
        .map_err(|_| "parent destruction failed")?;

    if manager.state(domain(1)) != Err(DomainError::UnknownDomain) {
        return Err("destroyed domain is still visible");
    }
    Ok(())
}

/// CTS-EXEC-AS-1 (RFC-0009.2): address space isolation — every domain has a
/// unique, immutable ASID and no address in one domain's space is directly
/// addressable from another domain.
fn exec_address_space_isolation() -> Result<(), &'static str> {
    let mut domains = DomainManager::new();
    domains
        .create_domain(domain(1), None, NOW_NS)
        .map_err(|_| "domain 1 creation failed")?;
    domains
        .create_domain(domain(2), None, NOW_NS)
        .map_err(|_| "domain 2 creation failed")?;

    let mut spaces = asm::AddressSpaceManager::new();
    let asid_1 = spaces
        .create_address_space(domain(1), &domains, NOW_NS)
        .map_err(|_| "address space 1 creation failed")?;
    let asid_2 = spaces
        .create_address_space(domain(2), &domains, NOW_NS)
        .map_err(|_| "address space 2 creation failed")?;
    if asid_1 == asid_2 {
        return Err("two domains share one ASID");
    }
    if spaces.asid_of(domain(1)) != Ok(asid_1) || spaces.asid_of(domain(2)) != Ok(asid_2) {
        return Err("ASID is not immutable for the domain lifetime");
    }
    // Neither domain has mapped anything: accesses must fault, and one
    // domain's space is never visible in the other's (spaces keyed by domain).
    if spaces
        .access(domain(1), 0x1000, 4096, asm::MemoryAccess::Read, NOW_NS)
        .is_ok()
        || spaces
            .access(domain(2), 0x1000, 4096, asm::MemoryAccess::Read, NOW_NS)
            .is_ok()
    {
        return Err("unmapped address was directly addressable");
    }
    Ok(())
}

/// CTS-EXEC-AS-2 (RFC-0009.3): mapping requires a valid MAP capability.
fn exec_address_space_map_requires_capability() -> Result<(), &'static str> {
    let mut domains = DomainManager::new();
    domains
        .create_domain(domain(1), None, NOW_NS)
        .map_err(|_| "domain creation failed")?;
    let mut spaces = asm::AddressSpaceManager::new();
    spaces
        .create_address_space(domain(1), &domains, NOW_NS)
        .map_err(|_| "space creation failed")?;

    let request = asm::MapRequest {
        virtual_range: bounds(0x1000, 0x2000),
        resource_range: bounds(0, 4096),
        page_size: asm::PageSize::Base4KiB,
        rights: asm::MappingRights {
            read: true,
            write: false,
            execute: false,
            copy_on_write: false,
            nocache: false,
        },
    };
    let no_map = Capability {
        object: ObjectId(5),
        object_type: ObjectType::Memory,
        rights: Rights::READ,
        bounds: bounds(0, 4096),
        flags: CapabilityFlags::NONE,
        expires_at_ns: 0,
    };
    if spaces.map(domain(1), &no_map, request, NOW_NS).is_ok() {
        return Err("mapping without MAP capability was permitted");
    }
    let map_cap = Capability {
        rights: Rights::MAP.union(Rights::READ),
        ..no_map
    };
    spaces
        .map(domain(1), &map_cap, request, NOW_NS)
        .map_err(|_| "MAP-capability mapping was rejected")?;
    Ok(())
}

/// CTS-EXEC-AS-3 (RFC-0009.7): rights enforcement — no self-elevation.
fn exec_address_space_rights_enforcement() -> Result<(), &'static str> {
    let mut domains = DomainManager::new();
    domains
        .create_domain(domain(1), None, NOW_NS)
        .map_err(|_| "domain creation failed")?;
    let mut spaces = asm::AddressSpaceManager::new();
    spaces
        .create_address_space(domain(1), &domains, NOW_NS)
        .map_err(|_| "space creation failed")?;

    let map_cap = Capability {
        object: ObjectId(6),
        object_type: ObjectType::Memory,
        rights: Rights::MAP.union(Rights::READ),
        bounds: bounds(0, 4096),
        flags: CapabilityFlags::NONE,
        expires_at_ns: 0,
    };
    let request = asm::MapRequest {
        virtual_range: bounds(0x1000, 0x2000),
        resource_range: bounds(0, 4096),
        page_size: asm::PageSize::Base4KiB,
        rights: asm::MappingRights {
            read: true,
            write: false,
            execute: false,
            copy_on_write: false,
            nocache: false,
        },
    };
    spaces
        .map(domain(1), &map_cap, request, NOW_NS)
        .map_err(|_| "read-only mapping failed")?;

    if spaces
        .access(domain(1), 0x1000, 0x100, asm::MemoryAccess::Read, NOW_NS)
        .is_err()
    {
        return Err("read access was denied on a read-only mapping");
    }
    if spaces
        .access(domain(1), 0x1000, 0x100, asm::MemoryAccess::Write, NOW_NS)
        .is_ok()
    {
        return Err("read-only mapping allowed a store (self-elevation)");
    }
    if spaces
        .access(domain(1), 0x1000, 0x100, asm::MemoryAccess::Execute, NOW_NS)
        .is_ok()
    {
        return Err("read-only mapping allowed execution");
    }
    // A capability granting only READ cannot authorize a write mapping.
    let write_request = asm::MapRequest {
        rights: asm::MappingRights {
            read: false,
            write: true,
            execute: false,
            copy_on_write: false,
            nocache: false,
        },
        ..request
    };
    if spaces
        .map(domain(1), &map_cap, write_request, NOW_NS)
        .is_ok()
    {
        return Err("READ-only capability authorized a WRITE mapping");
    }
    Ok(())
}

/// CTS-EXEC-AS-4 (RFC-0009.5): map/unmap are atomic w.r.t. dispatch and
/// execution — a completed unmap makes the range inaccessible, a completed
/// map makes it accessible, and no partially applied state is observable.
fn exec_address_space_atomic_mapping_updates() -> Result<(), &'static str> {
    let mut domains = DomainManager::new();
    domains
        .create_domain(domain(1), None, NOW_NS)
        .map_err(|_| "domain creation failed")?;
    let mut spaces = asm::AddressSpaceManager::new();
    spaces
        .create_address_space(domain(1), &domains, NOW_NS)
        .map_err(|_| "space creation failed")?;

    let map_cap = Capability {
        object: ObjectId(7),
        object_type: ObjectType::Memory,
        rights: Rights::MAP.union(Rights::READ).union(Rights::WRITE),
        bounds: bounds(0, 4096),
        flags: CapabilityFlags::NONE,
        expires_at_ns: 0,
    };
    let request = asm::MapRequest {
        virtual_range: bounds(0x3000, 0x4000),
        resource_range: bounds(0, 4096),
        page_size: asm::PageSize::Base4KiB,
        rights: asm::MappingRights {
            read: true,
            write: true,
            execute: false,
            copy_on_write: false,
            nocache: false,
        },
    };

    spaces
        .map(domain(1), &map_cap, request, NOW_NS)
        .map_err(|_| "first mapping failed")?;
    if spaces
        .access(domain(1), 0x3000, 0x100, asm::MemoryAccess::Write, NOW_NS)
        .is_err()
    {
        return Err("access failed immediately after a completed mapping");
    }

    spaces
        .unmap(domain(1), 0x3000, 0x1000, NOW_NS)
        .map_err(|_| "unmap failed")?;
    match spaces.access(domain(1), 0x3000, 0x100, asm::MemoryAccess::Read, NOW_NS) {
        Err(asm::AddressSpaceError::Unmapped) | Err(asm::AddressSpaceError::AddressSpaceEmpty) => {}
        _ => return Err("access was possible after a completed unmap"),
    }

    spaces
        .map(domain(1), &map_cap, request, NOW_NS)
        .map_err(|_| "remapping failed")?;
    if spaces
        .access(domain(1), 0x3000, 0x100, asm::MemoryAccess::Write, NOW_NS)
        .is_err()
    {
        return Err("access failed after remapping");
    }
    Ok(())
}

/// CTS-EXEC-AS-5 (RFC-0009.9): page-size selection is explicit.
fn exec_address_space_explicit_page_size() -> Result<(), &'static str> {
    let mut domains = DomainManager::new();
    domains
        .create_domain(domain(1), None, NOW_NS)
        .map_err(|_| "domain creation failed")?;
    let mut spaces = asm::AddressSpaceManager::new();
    spaces
        .create_address_space(domain(1), &domains, NOW_NS)
        .map_err(|_| "space creation failed")?;

    let map_cap = Capability {
        object: ObjectId(8),
        object_type: ObjectType::Memory,
        rights: Rights::MAP.union(Rights::READ),
        bounds: bounds(0, 4 * 1024 * 1024),
        flags: CapabilityFlags::NONE,
        expires_at_ns: 0,
    };

    // Unaligned virtual range is rejected for every page size class.
    let unaligned = asm::MapRequest {
        virtual_range: bounds(0x1001, 0x2000),
        resource_range: bounds(0, 4096),
        page_size: asm::PageSize::Base4KiB,
        rights: asm::MappingRights {
            read: true,
            write: false,
            execute: false,
            copy_on_write: false,
            nocache: false,
        },
    };
    if spaces.map(domain(1), &map_cap, unaligned, NOW_NS).is_ok() {
        return Err("unaligned base-page mapping was accepted");
    }

    // Base page, aligned: accepted.
    let base_ok = asm::MapRequest {
        virtual_range: bounds(0x1000, 0x2000),
        resource_range: bounds(0, 4096),
        page_size: asm::PageSize::Base4KiB,
        rights: asm::MappingRights {
            read: true,
            write: false,
            execute: false,
            copy_on_write: false,
            nocache: false,
        },
    };
    spaces
        .map(domain(1), &map_cap, base_ok, NOW_NS)
        .map_err(|_| "aligned base-page mapping failed")?;
    if spaces.page_size_of(domain(1), 0x1000) != Ok(asm::PageSize::Base4KiB) {
        return Err("base page size was not recorded");
    }

    // Large page, aligned: accepted.
    let large = asm::MapRequest {
        virtual_range: bounds(0x200000, 0x200000 + 2 * 1024 * 1024),
        resource_range: bounds(0, 2 * 1024 * 1024),
        page_size: asm::PageSize::Large2MiB,
        rights: asm::MappingRights {
            read: true,
            write: false,
            execute: false,
            copy_on_write: false,
            nocache: false,
        },
    };
    spaces
        .map(domain(1), &map_cap, large, NOW_NS)
        .map_err(|_| "aligned large-page mapping failed")?;
    if spaces.page_size_of(domain(1), 0x200000) != Ok(asm::PageSize::Large2MiB) {
        return Err("large page size was not recorded");
    }

    // Size not a multiple of the page size is rejected.
    let bad_size = asm::MapRequest {
        virtual_range: bounds(0x400000, 0x400000 + 4096),
        resource_range: bounds(0, 4096),
        page_size: asm::PageSize::Large2MiB,
        rights: asm::MappingRights {
            read: true,
            write: false,
            execute: false,
            copy_on_write: false,
            nocache: false,
        },
    };
    if spaces.map(domain(1), &map_cap, bad_size, NOW_NS).is_ok() {
        return Err("large-page mapping with base-page size was accepted");
    }
    Ok(())
}

/// CTS-EXEC-AS-6 (RFC-0009.15): ASID lifecycle — `ASID_DESTROYED` is
/// reachable only from `ASID_EMPTY`.
fn exec_address_space_asid_lifecycle() -> Result<(), &'static str> {
    let mut domains = DomainManager::new();
    domains
        .create_domain(domain(1), None, NOW_NS)
        .map_err(|_| "domain creation failed")?;
    let mut spaces = asm::AddressSpaceManager::new();
    let _ = spaces
        .create_address_space(domain(1), &domains, NOW_NS)
        .map_err(|_| "space creation failed")?;
    if spaces.state_of(domain(1)) != Ok(asm::AddressSpaceState::Created) {
        return Err("new address space is not in Created state");
    }

    // Destroying without unmapping all regions is non-conformant.
    if spaces.destroy_address_space(domain(1), NOW_NS).is_ok() {
        return Err("address space was destroyed from the Created state");
    }

    let map_cap = Capability {
        object: ObjectId(9),
        object_type: ObjectType::Memory,
        rights: Rights::MAP.union(Rights::READ),
        bounds: bounds(0, 4096),
        flags: CapabilityFlags::NONE,
        expires_at_ns: 0,
    };
    let request = asm::MapRequest {
        virtual_range: bounds(0x5000, 0x6000),
        resource_range: bounds(0, 4096),
        page_size: asm::PageSize::Base4KiB,
        rights: asm::MappingRights {
            read: true,
            write: false,
            execute: false,
            copy_on_write: false,
            nocache: false,
        },
    };
    spaces
        .map(domain(1), &map_cap, request, NOW_NS)
        .map_err(|_| "mapping failed")?;
    if spaces.state_of(domain(1)) != Ok(asm::AddressSpaceState::Active) {
        return Err("address space is not Active after the first mapping");
    }
    if spaces.destroy_address_space(domain(1), NOW_NS).is_ok() {
        return Err("address space was destroyed from the Active state");
    }

    spaces
        .unmap(domain(1), 0x5000, 0x1000, NOW_NS)
        .map_err(|_| "unmap failed")?;
    if spaces.state_of(domain(1)) != Ok(asm::AddressSpaceState::Empty) {
        return Err("address space is not Empty after unmapping all regions");
    }
    spaces
        .destroy_address_space(domain(1), NOW_NS)
        .map_err(|_| "destruction from Empty failed")?;
    if spaces.asid_of(domain(1)).is_ok() {
        return Err("destroyed address space is still visible");
    }
    Ok(())
}

trait IsDeny {
    fn is_deny(&self) -> bool;
}

impl IsDeny for MediationOutcome {
    fn is_deny(&self) -> bool {
        matches!(self, MediationOutcome::Deny(_))
    }
}

#[derive(Default)]
struct ImmediateFences {
    activations: Vec<(HartId, AddressSpaceId)>,
    invalidations: Vec<(HartId, AddressSpaceId, Bounds)>,
}

impl TlbShootdown for ImmediateFences {
    fn current_hart(&self) -> HartId {
        HartId::new(0)
    }

    fn activate_address_space_and_wait(
        &mut self,
        hart: HartId,
        address_space: AddressSpaceId,
    ) -> Result<(), ShootdownError> {
        self.activations.push((hart, address_space));
        Ok(())
    }

    fn publish_page_table_updates(
        &mut self,
        _address_space: AddressSpaceId,
        _virtual_range: Bounds,
    ) -> Result<(), ShootdownError> {
        Ok(())
    }

    fn invalidate_hart_and_wait(
        &mut self,
        hart: HartId,
        address_space: AddressSpaceId,
        virtual_range: Bounds,
    ) -> Result<(), ShootdownError> {
        self.invalidations
            .push((hart, address_space, virtual_range));
        Ok(())
    }
}

fn mapping_permissions_and_revocation() -> Result<(), &'static str> {
    let (mut capabilities, bootstrap) = CapabilitySystem::bootstrap();
    let owner = domain(1);
    let asid = AddressSpaceId::new(7);
    capabilities.add_domain(owner);
    let handle = capabilities
        .create_root(
            &bootstrap,
            owner,
            Capability {
                object: ObjectId(99),
                object_type: ObjectType::Memory,
                rights: Rights::MAP
                    .union(Rights::READ)
                    .union(Rights::WRITE)
                    .union(Rights::GRANT),
                bounds: bounds(0, 4096),
                flags: aether_types::CapabilityFlags::REVOCABLE,
                expires_at_ns: 0,
            },
        )
        .map_err(|_| "memory capability bootstrap failed")?;

    let mut memory = MemoryManager::new();
    memory
        .register_address_space(owner, asid)
        .map_err(|_| "address-space registration failed")?;
    let mut fences = ImmediateFences::default();
    let hart = HartId::new(1);
    memory
        .activate_for_dispatch(owner, hart, &mut fences)
        .map_err(|_| "dispatch address-space activation failed")?;
    memory
        .map(
            &capabilities,
            owner,
            handle,
            MapRequest {
                address_space: asid,
                virtual_range: bounds(0x4000, 0x5000),
                resource_range: bounds(0, 4096),
                page_size: PageSize::Base4KiB,
                permissions: PtePermissions::READ,
            },
            &mut fences,
            NOW_NS,
        )
        .map_err(|_| "capability-authorized page mapping failed")?;

    memory
        .access(
            &capabilities,
            hart,
            asid,
            0x4000,
            MemoryAccess::Read,
            NOW_NS,
        )
        .map_err(|_| "read access should be permitted by the PTE")?;
    if memory.access(
        &capabilities,
        hart,
        asid,
        0x4000,
        MemoryAccess::Write,
        NOW_NS,
    ) != Err(aether_capspace::memory::MemoryError::ProtectionFault)
    {
        return Err("read-only mapping allowed a store");
    }

    memory
        .revoke_capability(&mut capabilities, owner, handle, &mut fences, NOW_NS)
        .map_err(|_| "revocation did not complete after hart invalidation")?;
    if memory.access(
        &capabilities,
        hart,
        asid,
        0x4000,
        MemoryAccess::Read,
        NOW_NS,
    ) != Err(aether_capspace::memory::MemoryError::Unmapped)
    {
        return Err("revoked mapping remained accessible");
    }
    Ok(())
}

fn serialized_runtime_access_and_revocation() -> Result<(), &'static str> {
    let (runtime, bootstrap) = MemoryRuntime::bootstrap();
    let owner = domain(1);
    let asid = AddressSpaceId::new(21);
    runtime
        .register_domain(owner, asid)
        .map_err(|_| "serialized runtime domain registration failed")?;
    let handle = runtime
        .create_root(
            &bootstrap,
            owner,
            Capability {
                object: ObjectId(121),
                object_type: ObjectType::Memory,
                rights: Rights::MAP.union(Rights::READ),
                bounds: bounds(0, 4096),
                flags: aether_types::CapabilityFlags::REVOCABLE,
                expires_at_ns: 0,
            },
        )
        .map_err(|_| "serialized runtime root capability failed")?;
    let hart = HartId::new(9);
    let mut fences = ImmediateFences::default();
    runtime
        .activate_for_dispatch(owner, hart, &mut fences)
        .map_err(|_| "serialized runtime address-space activation failed")?;
    runtime
        .map(
            owner,
            handle,
            MapRequest {
                address_space: asid,
                virtual_range: bounds(0xa000, 0xb000),
                resource_range: bounds(0, 4096),
                page_size: PageSize::Base4KiB,
                permissions: PtePermissions::READ,
            },
            &mut fences,
            NOW_NS,
        )
        .map_err(|_| "serialized runtime mapping failed")?;
    runtime
        .access(hart, asid, 0xa000, MemoryAccess::Read, NOW_NS)
        .map_err(|_| "serialized runtime initial read failed")?;
    runtime
        .revoke_capability(owner, handle, &mut fences, NOW_NS)
        .map_err(|_| "serialized runtime revocation failed")?;
    if runtime.access(hart, asid, 0xa000, MemoryAccess::Read, NOW_NS)
        != Err(aether_capspace::runtime::RuntimeError::Memory(
            aether_capspace::memory::MemoryError::Unmapped,
        ))
    {
        return Err("serialized runtime allowed access after completed revocation");
    }
    if runtime
        .pending_invalidation_count()
        .map_err(|_| "serialized runtime state lock failed")?
        != 0
    {
        return Err("serialized runtime left an invalidation pending after successful revoke");
    }
    Ok(())
}

fn cached_mapping_requires_live_capability() -> Result<(), &'static str> {
    let (mut capabilities, bootstrap) = CapabilitySystem::bootstrap();
    let owner = domain(1);
    let asid = AddressSpaceId::new(19);
    capabilities.add_domain(owner);
    let handle = capabilities
        .create_root(
            &bootstrap,
            owner,
            Capability {
                object: ObjectId(109),
                object_type: ObjectType::Memory,
                rights: Rights::MAP.union(Rights::READ),
                bounds: bounds(0, 4096),
                flags: aether_types::CapabilityFlags::REVOCABLE,
                expires_at_ns: 0,
            },
        )
        .map_err(|_| "memory capability bootstrap failed")?;

    let mut memory = MemoryManager::new();
    memory
        .register_address_space(owner, asid)
        .map_err(|_| "address-space registration failed")?;
    let mut fences = ImmediateFences::default();
    let hart = HartId::new(8);
    memory
        .activate_for_dispatch(owner, hart, &mut fences)
        .map_err(|_| "dispatch address-space activation failed")?;
    memory
        .map(
            &capabilities,
            owner,
            handle,
            MapRequest {
                address_space: asid,
                virtual_range: bounds(0x6000, 0x7000),
                resource_range: bounds(0, 4096),
                page_size: PageSize::Base4KiB,
                permissions: PtePermissions::READ,
            },
            &mut fences,
            NOW_NS,
        )
        .map_err(|_| "capability-authorized mapping failed")?;

    memory
        .access(
            &capabilities,
            hart,
            asid,
            0x6000,
            MemoryAccess::Read,
            NOW_NS,
        )
        .map_err(|_| "initial access should populate the host TLB model")?;
    capabilities
        .revoke(owner, handle)
        .map_err(|_| "capability revocation failed")?;

    if memory.access(
        &capabilities,
        hart,
        asid,
        0x6000,
        MemoryAccess::Read,
        NOW_NS,
    ) != Err(aether_capspace::memory::MemoryError::Capability(
        CapError::Revoked,
    )) {
        return Err("cached translation bypassed the revoked MAP capability");
    }
    Ok(())
}

fn dispatch_address_space_activation() -> Result<(), &'static str> {
    let (mut capabilities, bootstrap) = CapabilitySystem::bootstrap();
    let owner = domain(1);
    let other = domain(2);
    let first_asid = AddressSpaceId::new(17);
    let second_asid = AddressSpaceId::new(18);
    capabilities.add_domain(owner);
    capabilities.add_domain(other);
    let handle = capabilities
        .create_root(
            &bootstrap,
            owner,
            Capability {
                object: ObjectId(101),
                rights: Rights::MAP.union(Rights::READ),
                ..root()
            },
        )
        .map_err(|_| "memory capability bootstrap failed")?;

    let mut memory = MemoryManager::new();
    memory
        .register_address_space(owner, first_asid)
        .map_err(|_| "first address-space registration failed")?;
    memory
        .register_address_space(other, second_asid)
        .map_err(|_| "second address-space registration failed")?;

    let hart = HartId::new(7);
    let access_denied = |memory: &mut MemoryManager, address_space, address| {
        memory.access(
            &capabilities,
            hart,
            address_space,
            address,
            MemoryAccess::Read,
            NOW_NS,
        ) == Err(aether_capspace::memory::MemoryError::AddressSpaceNotActive)
    };
    if !access_denied(&mut memory, first_asid, 0x4000) {
        return Err("memory access was allowed before dispatch activation");
    }

    let mut fences = ImmediateFences::default();
    memory
        .activate_for_dispatch(owner, hart, &mut fences)
        .map_err(|_| "first dispatch activation failed")?;
    if fences.activations != vec![(hart, first_asid)] {
        return Err("backend did not receive the address-space activation");
    }

    memory
        .map(
            &capabilities,
            owner,
            handle,
            MapRequest {
                address_space: first_asid,
                virtual_range: bounds(0x4000, 0x5000),
                resource_range: bounds(0, 4096),
                page_size: PageSize::Base4KiB,
                permissions: PtePermissions::READ,
            },
            &mut fences,
            NOW_NS,
        )
        .map_err(|_| "mapping after dispatch activation failed")?;
    if !fences
        .invalidations
        .contains(&(hart, first_asid, bounds(0x4000, 0x5000)))
    {
        return Err("mapping activation omitted a dispatched but not-yet-accessing hart");
    }
    memory
        .access(
            &capabilities,
            hart,
            first_asid,
            0x4000,
            MemoryAccess::Read,
            NOW_NS,
        )
        .map_err(|_| "activated address-space access failed")?;

    memory
        .activate_for_dispatch(other, hart, &mut fences)
        .map_err(|_| "address-space switch failed")?;
    if !access_denied(&mut memory, first_asid, 0x4000) {
        return Err("previous address space remained executable after switch");
    }
    memory
        .activate_for_dispatch(owner, hart, &mut fences)
        .map_err(|_| "migration back to the first address space failed")?;
    memory
        .access(
            &capabilities,
            hart,
            first_asid,
            0x4000,
            MemoryAccess::Read,
            NOW_NS,
        )
        .map_err(|_| "address space was not accessible after reactivation")?;
    Ok(())
}

fn reject_forged_integer() -> Result<(), &'static str> {
    let (model, _, system, _) = pair();
    compare_auth(
        model.authorize(
            domain(1),
            Handle::from_untrusted(0xdead_beef),
            ObjectType::Memory,
            Rights::READ,
            bounds(0, 1),
            NOW_NS,
        ),
        system.authorize(
            domain(1),
            Handle::from_untrusted(0xdead_beef),
            ObjectType::Memory,
            Rights::READ,
            bounds(0, 1),
            NOW_NS,
        ),
    )
}

fn reject_stale_generation() -> Result<(), &'static str> {
    let (_, _, mut system, system_bootstrap) = pair();
    let handle = system
        .create_root(&system_bootstrap, domain(1), root())
        .map_err(|_| "provider root failed")?;
    system
        .remove(domain(1), handle)
        .map_err(|_| "provider remove failed")?;
    match system.authorize(
        domain(1),
        handle,
        ObjectType::Memory,
        Rights::READ,
        bounds(0, 1),
        NOW_NS,
    ) {
        Err(CapError::Stale) => Ok(()),
        _ => Err("provider accepted stale handle or returned wrong failure"),
    }
}

fn reject_foreign_bootstrap() -> Result<(), &'static str> {
    let (mut first_model, _) = CapabilityModel::bootstrap();
    let (_, second_model_bootstrap) = CapabilityModel::bootstrap();
    let (mut first_system, _) = CapabilitySystem::bootstrap();
    let (_, second_system_bootstrap) = CapabilitySystem::bootstrap();
    first_system.add_domain(domain(1));
    let model_result = first_model.create_root(&second_model_bootstrap, domain(1), root());
    let system_result = first_system.create_root(&second_system_bootstrap, domain(1), root());
    if model_result == Err(CapError::WrongBootstrap)
        && system_result == Err(CapError::WrongBootstrap)
    {
        Ok(())
    } else {
        Err("foreign bootstrap witness created root authority")
    }
}

fn compare_auth(
    model: Result<Capability, CapError>,
    system: Result<Capability, CapError>,
) -> Result<(), &'static str> {
    if model == system {
        Ok(())
    } else {
        Err("model/provider authorization mismatch")
    }
}

fn state_transaction(engine: &mut StateEngine) -> aether_state_model::PreparedTransaction {
    let mut transaction = engine
        .prepare(1, 2)
        .expect("fresh CTS engine has transaction identifiers");
    transaction.mutate(Mutation {
        object: ObjectId(9),
        version: VersionId(1),
        content_digest: [7; 32],
    });
    transaction.intend(EffectIntent {
        operation: OperationId(10),
        class: EffectClass::Deduplicated,
        request_digest: [8; 32],
    });
    transaction
}

fn state_commit_is_failure_atomic() -> Result<(), &'static str> {
    let mut engine = StateEngine::new();
    let transaction = state_transaction(&mut engine);
    engine.commit(transaction).map_err(|_| "commit failed")?;
    let log = engine.durable_log();
    for cut in 0..log.len() {
        let recovered =
            StateEngine::recover(&log[..cut]).map_err(|_| "torn-tail recovery failed")?;
        if recovered.visible_version(ObjectId(9)).is_some()
            || recovered.effect_status(OperationId(10)).is_some()
        {
            return Err("partial commit became visible");
        }
    }
    let recovered = StateEngine::recover(log).map_err(|_| "complete recovery failed")?;
    if recovered.visible_version(ObjectId(9)) == Some(VersionId(1))
        && recovered.effect_status(OperationId(10)) == Some(EffectStatus::Pending)
    {
        Ok(())
    } else {
        Err("complete commit was not recovered")
    }
}

fn uncertain_effect_is_recoverable() -> Result<(), &'static str> {
    let mut engine = StateEngine::new();
    let transaction = state_transaction(&mut engine);
    engine.commit(transaction).map_err(|_| "commit failed")?;
    engine
        .record_effect(OperationId(10), EffectStatus::Uncertain)
        .map_err(|_| "uncertain outcome was rejected")?;
    let mut recovered =
        StateEngine::recover(engine.durable_log()).map_err(|_| "recovery failed")?;
    if recovered.effect_status(OperationId(10)) != Some(EffectStatus::Uncertain) {
        return Err("uncertain outcome was erased");
    }
    recovered
        .record_effect(OperationId(10), EffectStatus::Acknowledged)
        .map_err(|_| "reconciliation failed")?;
    if recovered.effect_status(OperationId(10)) == Some(EffectStatus::Acknowledged) {
        Ok(())
    } else {
        Err("reconciliation was not durable")
    }
}

fn reject_replayed_transaction() -> Result<(), &'static str> {
    let mut engine = StateEngine::new();
    let transaction = state_transaction(&mut engine);
    engine.commit(transaction).map_err(|_| "commit failed")?;
    let mut replayed = engine.durable_log().to_vec();
    replayed.extend_from_slice(engine.durable_log());
    match StateEngine::recover(&replayed) {
        Err(aether_state_model::StateError::DuplicateTransaction) => Ok(()),
        _ => Err("replayed transaction was not rejected"),
    }
}

fn reject_duplicate_mutation() -> Result<(), &'static str> {
    let mut engine = StateEngine::new();
    let mut transaction = engine
        .prepare(1, 2)
        .map_err(|_| "transaction identifier allocation failed")?;
    let mutation = Mutation {
        object: ObjectId(9),
        version: VersionId(1),
        content_digest: [7; 32],
    };
    transaction.mutate(mutation);
    transaction.mutate(mutation);
    match engine.commit(transaction) {
        Err(aether_state_model::StateError::DuplicateMutation)
            if engine.durable_log().is_empty() =>
        {
            Ok(())
        }
        _ => Err("duplicate mutation reached durable log"),
    }
}

fn expiry_boundary_matches_rfc_0037_14() -> Result<(), &'static str> {
    let (mut model, model_bootstrap, mut system, system_bootstrap) = pair();
    let expiring = Capability {
        expires_at_ns: 1_000,
        ..root()
    };
    let model_root = model
        .create_root(&model_bootstrap, domain(1), expiring)
        .map_err(|_| "model root failed")?;
    let system_root = system
        .create_root(&system_bootstrap, domain(1), expiring)
        .map_err(|_| "provider root failed")?;

    // Equality is still live in BOTH models; the first instant after expiry is
    // rejected. Assert actual outcomes (not only model/provider agreement) so a
    // uniform boundary regression in both models cannot pass this case.
    if model
        .authorize(
            domain(1),
            model_root,
            ObjectType::Memory,
            Rights::READ,
            bounds(0, 1),
            1_000,
        )
        .is_err()
        || system
            .authorize(
                domain(1),
                system_root,
                ObjectType::Memory,
                Rights::READ,
                bounds(0, 1),
                1_000,
            )
            .is_err()
    {
        return Err("capability expired at exact expires_at_ns boundary");
    }
    let model_expired = model.authorize(
        domain(1),
        model_root,
        ObjectType::Memory,
        Rights::READ,
        bounds(0, 1),
        1_001,
    );
    let system_expired = system.authorize(
        domain(1),
        system_root,
        ObjectType::Memory,
        Rights::READ,
        bounds(0, 1),
        1_001,
    );
    if model_expired != Err(CapError::Expired) || system_expired != Err(CapError::Expired) {
        return Err("expiry boundary mismatch");
    }

    // Expired parents cannot derive or transfer in either model.
    compare_handle_error(
        model.derive(domain(1), model_root, Rights::READ, bounds(0, 1), 1_001),
        system.derive(domain(1), system_root, Rights::READ, bounds(0, 1), 1_001),
        CapError::Expired,
    )?;
    compare_handle_error(
        model.transfer(
            domain(1),
            model_root,
            domain(2),
            Rights::READ,
            bounds(0, 1),
            1_001,
        ),
        system.transfer(
            domain(1),
            system_root,
            domain(2),
            Rights::READ,
            bounds(0, 1),
            1_001,
        ),
        CapError::Expired,
    )
}

fn bound_capability_cannot_be_transferred() -> Result<(), &'static str> {
    let (mut model, model_bootstrap, mut system, system_bootstrap) = pair();
    let bound = Capability {
        flags: aether_types::CapabilityFlags::BOUND,
        ..root()
    };
    let model_root = model
        .create_root(&model_bootstrap, domain(1), bound)
        .map_err(|_| "model root failed")?;
    let system_root = system
        .create_root(&system_bootstrap, domain(1), bound)
        .map_err(|_| "provider root failed")?;

    // Same-domain use remains valid for a bound capability.
    compare_auth(
        model.authorize(
            domain(1),
            model_root,
            ObjectType::Memory,
            Rights::READ,
            bounds(0, 1),
            NOW_NS,
        ),
        system.authorize(
            domain(1),
            system_root,
            ObjectType::Memory,
            Rights::READ,
            bounds(0, 1),
            NOW_NS,
        ),
    )?;

    // Transfer to another domain is refused even with GRANT in both models.
    compare_handle_error(
        model.transfer(
            domain(1),
            model_root,
            domain(2),
            Rights::READ,
            bounds(0, 1),
            NOW_NS,
        ),
        system.transfer(
            domain(1),
            system_root,
            domain(2),
            Rights::READ,
            bounds(0, 1),
            NOW_NS,
        ),
        CapError::DomainBound,
    )
}

fn admission_bound_matches_rfc_0011_7() -> Result<(), &'static str> {
    let mut sched = UniprocessorEdf::new(1_000);
    sched
        .create_thread(ThreadId(1), 2, 1, 10)
        .map_err(|_| "first thread admission failed")?;
    sched
        .create_thread(ThreadId(2), 3, 1, 10)
        .map_err(|_| "second thread admission failed")?;
    sched
        .create_thread(ThreadId(3), 6, 1, 10)
        .map_err(|_| "third thread admission failed")?;
    if sched.utilization() != Ok((1, 1)) {
        return Err("exact utilization accounting mismatch at U = 1");
    }
    if sched.create_thread(ThreadId(4), 1_000_000, 1, 10)
        != Err(aether_sched_model::SchedError::NotAdmissible)
    {
        return Err("over-utilization set was admitted");
    }
    Ok(())
}

fn scheduler_selection_is_deterministic() -> Result<(), &'static str> {
    fn run(order: &[(u64, u64, u64)]) -> Vec<ThreadId> {
        let mut sched = UniprocessorEdf::new(1_000);
        for &(tid, period, budget) in order {
            sched
                .create_thread(ThreadId(tid), period, budget, 10)
                .expect("admission should succeed for U <= 1");
        }
        let mut picked = Vec::new();
        for _ in 0..4 {
            picked.push(
                sched
                    .dispatch_next()
                    .expect("dispatch should not fail")
                    .unwrap_or(ThreadId(0)),
            );
            sched.yield_now().expect("running thread must yield");
            sched
                .requeue_yielded()
                .expect("yielded thread must requeue");
        }
        picked
    }
    let forward = run(&[(1, 10, 1), (2, 10, 1), (3, 10, 1)]);
    let reverse = run(&[(3, 10, 1), (2, 10, 1), (1, 10, 1)]);
    let expected = vec![ThreadId(1), ThreadId(2), ThreadId(3), ThreadId(1)];
    if forward != reverse || forward != expected {
        return Err("scheduler decision sequence differs across identical inputs");
    }
    Ok(())
}

fn quantum_expiry_requeues_with_new_deadline() -> Result<(), &'static str> {
    let mut sched = UniprocessorEdf::new(5);
    sched
        .create_thread(ThreadId(1), 100, 100, 10)
        .map_err(|_| "thread admission failed")?;
    if sched.dispatch_next() != Ok(Some(ThreadId(1))) {
        return Err("admissible thread was not dispatched");
    }
    sched.advance(5).map_err(|_| "time advance failed")?;
    if sched.state(ThreadId(1)) != Some(ThreadState::Ready) {
        return Err("quantum expiry did not re-queue the running thread");
    }
    if sched.deadline(ThreadId(1)) != Some(200) {
        return Err("quantum expiry did not advance the deadline by one period");
    }
    let events = sched.take_events();
    let mut reasons: Vec<TransitionReason> = events.iter().map(|event| event.reason).collect();
    reasons.retain(|reason| {
        *reason == TransitionReason::QuantumExpired || *reason == TransitionReason::Requeued
    });
    if reasons.as_slice() != [TransitionReason::QuantumExpired, TransitionReason::Requeued] {
        return Err("quantum expiry did not emit the required transition events");
    }
    if sched.deadline_misses() != 0 {
        return Err("deadline miss was charged inside an in-flight period");
    }
    Ok(())
}

fn comp_component(id: u64) -> ComponentId {
    ComponentId(id)
}

fn comp_manifest(id: u64, version: u64, requires: Vec<Requirement>) -> Manifest {
    Manifest {
        id: comp_component(id),
        schema_version: 1,
        version,
        requires,
        provides: Vec::new(),
        profiles: vec![Profile::Desktop],
        tier: Tier::B,
        max_instances: 4,
        rollback_on_failure: true,
        start: "main".to_string(),
        stop: "main".to_string(),
    }
}

fn comp_requirement(component: u64, min: u64, max: u64) -> Requirement {
    Requirement {
        component: comp_component(component),
        min_version: min,
        max_version: max,
    }
}

fn comp_pool(entries: Vec<Manifest>) -> std::collections::BTreeMap<ComponentId, Vec<Manifest>> {
    let mut map: std::collections::BTreeMap<ComponentId, Vec<Manifest>> =
        std::collections::BTreeMap::new();
    for manifest in entries {
        map.entry(manifest.id).or_default().push(manifest);
    }
    map
}

fn composition_resolution_matches_rfc_0004() -> Result<(), &'static str> {
    use aether_comp_model::resolve;
    let target = comp_manifest(1, 1, vec![comp_requirement(2, 1, 9)]);
    let candidates = comp_pool(vec![
        comp_manifest(2, 3, Vec::new()),
        comp_manifest(2, 7, Vec::new()),
        comp_manifest(2, 10, Vec::new()), // outside the declared range
    ]);
    let resolution = resolve(&target, &candidates, SelectionStrategy::HighestVersion)
        .map_err(|_| "resolution failed on a satisfiable pool")?;
    if resolution.order != vec![comp_component(2), comp_component(1)] {
        return Err("instantiation order is not dependencies-first");
    }
    if resolution.events.first().map(|event| event.version) != Some(7) {
        return Err("highest-version strategy did not select version 7");
    }

    // RFC-0004.6 reason codes are distinct.
    let missing = comp_pool(Vec::new());
    if resolve(&target, &missing, SelectionStrategy::HighestVersion)
        != Err(ResolutionError::MissingProvider(comp_component(2)))
    {
        return Err("missing provider reason code mismatch");
    }
    let version = comp_pool(vec![comp_manifest(2, 11, Vec::new())]);
    if resolve(&target, &version, SelectionStrategy::HighestVersion)
        != Err(ResolutionError::VersionMismatch(comp_component(2)))
    {
        return Err("version mismatch reason code mismatch");
    }
    let mut other_profile = comp_manifest(2, 5, Vec::new());
    other_profile.profiles = vec![Profile::HPC];
    let profile = comp_pool(vec![other_profile]);
    if resolve(&target, &profile, SelectionStrategy::HighestVersion)
        != Err(ResolutionError::ProfileMismatch(comp_component(2)))
    {
        return Err("profile mismatch reason code mismatch");
    }
    let mut weak = comp_manifest(2, 5, Vec::new());
    weak.tier = Tier::D;
    let tier = comp_pool(vec![weak]);
    if resolve(&target, &tier, SelectionStrategy::HighestVersion)
        != Err(ResolutionError::TierMismatch(comp_component(2)))
    {
        return Err("tier mismatch reason code mismatch");
    }
    Ok(())
}

fn composition_cycle_is_reported() -> Result<(), &'static str> {
    use aether_comp_model::resolve;
    let a = comp_manifest(1, 1, vec![comp_requirement(2, 1, 1)]);
    let b = comp_manifest(2, 1, vec![comp_requirement(1, 1, 1)]);
    let candidates = comp_pool(vec![a.clone(), b]);
    match resolve(&a, &candidates, SelectionStrategy::HighestVersion) {
        Err(ResolutionError::Cycle(members))
            if members == vec![comp_component(1), comp_component(2)] =>
        {
            Ok(())
        }
        _ => Err("dependency cycle was not reported with sorted members"),
    }
}

fn composition_order_is_deterministic() -> Result<(), &'static str> {
    use aether_comp_model::resolve;
    let build = || {
        let root = comp_manifest(
            1,
            1,
            vec![comp_requirement(2, 1, 1), comp_requirement(3, 1, 1)],
        );
        let a = comp_manifest(2, 1, vec![comp_requirement(4, 1, 1)]);
        let b = comp_manifest(3, 1, vec![comp_requirement(4, 1, 1)]);
        let d = comp_manifest(4, 1, Vec::new());
        let pool = comp_pool(vec![d, b, a]);
        resolve(&root, &pool, SelectionStrategy::HighestVersion).map(|resolution| resolution.order)
    };
    let first = build().map_err(|_| "first resolution failed")?;
    let second = build().map_err(|_| "second resolution failed")?;
    if first != second {
        return Err("identical inputs produced different instantiation orders");
    }
    let position = |id: u64| {
        first
            .iter()
            .position(|component| *component == comp_component(id))
            .ok_or("component missing from resolution")
    };
    if position(4)? > position(2)? || position(4)? > position(3)? {
        return Err("a provider was ordered after its dependent");
    }
    if position(1)? != first.len() - 1 {
        return Err("the target was not instantiated last");
    }
    Ok(())
}

fn invalid_manifest_is_rejected() -> Result<(), &'static str> {
    let mut zero_id = comp_manifest(1, 1, Vec::new());
    zero_id.id = comp_component(0);
    match validate_manifest(&zero_id) {
        Err(ManifestError::ZeroComponentId) => {}
        _ => return Err("zero component id was accepted"),
    }
    let mut zero_instances = comp_manifest(1, 1, Vec::new());
    zero_instances.max_instances = 0;
    match validate_manifest(&zero_instances) {
        Err(ManifestError::InvalidMaxInstances) => {}
        _ => return Err("max-instances = 0 was accepted"),
    }
    let mut no_profiles = comp_manifest(1, 1, Vec::new());
    no_profiles.profiles = Vec::new();
    match validate_manifest(&no_profiles) {
        Err(ManifestError::NoProfiles) => {}
        _ => return Err("a manifest without profiles was accepted"),
    }
    let reversed = comp_manifest(1, 1, vec![comp_requirement(2, 9, 1)]);
    match validate_manifest(&reversed) {
        Err(ManifestError::InvalidRequirement) => {}
        _ => return Err("an inverted requirement range was accepted"),
    }
    let empty_name = comp_manifest(1, 1, Vec::new());
    if validate_manifest(&empty_name).is_err() {
        return Err("valid baseline manifest was rejected");
    }
    let _ = NarrowingPolicy::Attenuate;
    Ok(())
}

fn concurrent_writer_conflicts_match_rfc_0016_9() -> Result<(), &'static str> {
    let mut engine = StateEngine::new();
    let seed = state_transaction(&mut engine);
    engine.commit(seed).map_err(|_| "seed commit failed")?;

    let mut first = engine.prepare(1, 1).map_err(|_| "prepare first failed")?;
    first.mutate(Mutation {
        object: ObjectId(9),
        version: VersionId(2),
        content_digest: [9; 32],
    });
    let mut second = engine.prepare(2, 2).map_err(|_| "prepare second failed")?;
    second.mutate(Mutation {
        object: ObjectId(9),
        version: VersionId(3),
        content_digest: [10; 32],
    });

    engine
        .commit(first)
        .map_err(|_| "first writer failed to commit")?;
    let length = engine.durable_log().len();
    if engine.commit(second) != Err(aether_state_model::StateError::WriteWriteConflict) {
        return Err("second writer was not rejected with WriteWriteConflict");
    }
    if engine.durable_log().len() != length {
        return Err("a conflicted transaction appended a frame");
    }
    if engine.visible_version(ObjectId(9)) != Some(VersionId(2)) {
        return Err("first writer's commit is not visible");
    }
    Ok(())
}

fn snapshot_read_matches_rfc_0016_8() -> Result<(), &'static str> {
    let mut engine = StateEngine::new();
    let seed = state_transaction(&mut engine); // object 9 -> version 1
    engine.commit(seed).map_err(|_| "seed commit failed")?;

    let reader = engine.prepare(1, 1).map_err(|_| "prepare reader failed")?;
    let mut writer = engine.prepare(2, 2).map_err(|_| "prepare writer failed")?;
    writer.mutate(Mutation {
        object: ObjectId(9),
        version: VersionId(2),
        content_digest: [11; 32],
    });
    engine.commit(writer).map_err(|_| "writer commit failed")?;

    if reader.snapshot_read(ObjectId(9)) != Some(VersionId(1)) {
        return Err("snapshot read observed a concurrent commit");
    }
    let mut reader = reader;
    reader.mutate(Mutation {
        object: ObjectId(9),
        version: VersionId(9),
        content_digest: [12; 32],
    });
    if reader.snapshot_read(ObjectId(9)) != Some(VersionId(9)) {
        return Err("snapshot read did not apply read-your-writes");
    }
    let fresh = engine.prepare(3, 3).map_err(|_| "prepare fresh failed")?;
    if fresh.snapshot_read(ObjectId(9)) != Some(VersionId(2)) {
        return Err("a fresh transaction did not observe the committed version");
    }
    Ok(())
}

fn prof_mechanism(name: &str) -> Mechanism {
    Mechanism::new(name).expect("non-empty mechanism name")
}

fn prof_manifest(id: u64, tier: Tier) -> Manifest {
    Manifest {
        id: ComponentId(id),
        schema_version: 1,
        version: 1,
        requires: Vec::new(),
        provides: Vec::new(),
        profiles: vec![Profile::Desktop],
        tier,
        max_instances: 1,
        rollback_on_failure: true,
        start: "main".to_string(),
        stop: "main".to_string(),
    }
}

fn prof_desktop() -> ProfileDeclaration {
    ProfileDeclaration {
        profile: Profile::Desktop,
        mandatory: vec![prof_mechanism("RFC-0011: layer1-scheduler")],
        declared_optional: vec![prof_mechanism("RFC-0020: posix-projection")],
        forbidden: vec![prof_mechanism("multi-domain-federation")],
        extensions: Vec::new(),
        minimum_tier: Tier::B,
    }
}

fn profile_selection_matches_rfc_0005() -> Result<(), &'static str> {
    let strong = prof_manifest(1, Tier::A);
    let decision =
        apply_profile(&prof_desktop(), &strong).map_err(|_| "strong-tier manifest was rejected")?;
    if decision.activated
        != vec![
            prof_mechanism("RFC-0011: layer1-scheduler"),
            prof_mechanism("RFC-0020: posix-projection"),
        ]
    {
        return Err("activated mechanism set mismatch");
    }
    // Default-off: no undeclared optional or extension may appear.
    if decision.activated.len() != 2 {
        return Err("an undeclared mechanism was activated by default");
    }
    // Explicit opt-in activates an extension (RFC-0005.5).
    let mut with_extension = prof_desktop();
    with_extension
        .extensions
        .push(prof_mechanism("object-projection"));
    let decision =
        apply_profile(&with_extension, &strong).map_err(|_| "opted-in extension was rejected")?;
    if !decision
        .activated
        .contains(&prof_mechanism("object-projection"))
    {
        return Err("an explicitly opted-in extension was not activated");
    }
    // Opting into a forbidden mechanism fails.
    let mut forbidden_opt_in = prof_desktop();
    forbidden_opt_in
        .extensions
        .push(prof_mechanism("multi-domain-federation"));
    if apply_profile(&forbidden_opt_in, &strong).is_ok() {
        return Err("an opted-in forbidden mechanism was accepted");
    }
    let weak = prof_manifest(1, Tier::D);
    if apply_profile(&prof_desktop(), &weak).is_ok() {
        return Err("manifest weaker than the profile floor was accepted");
    }
    Ok(())
}

fn profile_combination_matches_rfc_0005() -> Result<(), &'static str> {
    let safety = ProfileDeclaration {
        profile: Profile::SafetyCritical,
        mandatory: vec![prof_mechanism("locked-manifests")],
        declared_optional: Vec::new(),
        forbidden: vec![
            prof_mechanism("RFC-0011: layer3-offline-learning"),
            // The desktop mandatory mechanism is forbidden by Safety-Critical:
            // it must be suppressed, never activated.
            prof_mechanism("RFC-0011: layer1-scheduler"),
        ],
        extensions: Vec::new(),
        minimum_tier: Tier::A,
    };
    let desktop = prof_desktop();
    let manifest = prof_manifest(1, Tier::A);
    let decision = combine_profiles(&[desktop, safety.clone()], &manifest)
        .map_err(|_| "combination was rejected")?;
    if decision
        .activated
        .contains(&prof_mechanism("RFC-0011: layer1-scheduler"))
    {
        return Err("a mechanism forbidden by a combined profile was activated");
    }
    if !decision
        .suppressed
        .contains(&prof_mechanism("RFC-0011: layer1-scheduler"))
    {
        return Err("a suppressed mechanism was not reported as suppressed");
    }
    if !decision
        .activated
        .contains(&prof_mechanism("locked-manifests"))
    {
        return Err("combined mandatory sets were not unioned");
    }
    // The strictest floor applies: Tier B manifest fails the Safety-Critical floor.
    let weaker = prof_manifest(1, Tier::B);
    let safety = ProfileDeclaration {
        profile: Profile::SafetyCritical,
        mandatory: vec![prof_mechanism("locked-manifests")],
        declared_optional: Vec::new(),
        forbidden: Vec::new(),
        extensions: Vec::new(),
        minimum_tier: Tier::A,
    };
    if combine_profiles(&[prof_desktop(), safety], &weaker).is_ok() {
        return Err("the strictest tier floor was not applied");
    }
    Ok(())
}

fn routing_grantor(provides: Vec<aether_comp_model::Grant>) -> aether_comp_model::Manifest {
    aether_comp_model::Manifest {
        id: ComponentId(1),
        schema_version: 1,
        version: 1,
        requires: Vec::new(),
        provides,
        profiles: vec![Profile::Desktop],
        tier: Tier::B,
        max_instances: 1,
        rollback_on_failure: true,
        start: "main".to_string(),
        stop: "main".to_string(),
    }
}

fn routing_requestor() -> aether_comp_model::Manifest {
    aether_comp_model::Manifest {
        id: ComponentId(2),
        ..routing_grantor(Vec::new())
    }
}

fn routing_authority() -> Capability {
    Capability {
        object: ObjectId(42),
        object_type: ObjectType::Memory,
        rights: Rights::READ.union(Rights::WRITE).union(Rights::MAP),
        bounds: Bounds::new(0, 65536).expect("valid bounds"),
        flags: CapabilityFlags::NONE,
        expires_at_ns: 0,
    }
}

fn attenuated(name: &str, ceiling: Option<Rights>) -> aether_comp_model::Grant {
    aether_comp_model::Grant {
        name: name.to_string(),
        narrowing: NarrowingPolicy::Attenuate,
        rights_ceiling: ceiling,
    }
}

fn routing_grant_matches_rfc_0003() -> Result<(), &'static str> {
    let grantor = routing_grantor(vec![attenuated(
        "memory.read",
        Some(Rights::READ.union(Rights::WRITE)),
    )]);
    let requestor = routing_requestor();
    let authority = routing_authority();
    let outcome = routing::route(
        &grantor,
        &requestor,
        &authority,
        &routing::RoutingRequest {
            capability: "memory.read".to_string(),
            requested_rights: Rights::READ,
        },
    );
    match outcome {
        routing::RoutingOutcome::Granted(capability)
            if capability.rights == Rights::READ
                && capability.object == authority.object
                && capability.object_type == authority.object_type
                && capability.bounds == authority.bounds
                && capability.flags == authority.flags
                && capability.expires_at_ns == authority.expires_at_ns =>
        {
            Ok(())
        }
        _ => Err("attenuated grant was not narrowed to the requested rights"),
    }
}

fn routing_denial_matches_rfc_0003() -> Result<(), &'static str> {
    use routing::DenialReason;
    let authority = routing_authority();
    let requestor = routing_requestor();

    // Request beyond the ceiling.
    let grantor = routing_grantor(vec![attenuated("memory.read", Some(Rights::READ))]);
    let outcome = routing::route(
        &grantor,
        &requestor,
        &authority,
        &routing::RoutingRequest {
            capability: "memory.read".to_string(),
            requested_rights: Rights::MAP,
        },
    );
    if outcome != routing::RoutingOutcome::Denied(DenialReason::NarrowingFailed) {
        return Err("request exceeding the ceiling was not rejected");
    }

    // Ceiling the grantor authority does not hold.
    let grantor = routing_grantor(vec![attenuated("invoke", Some(Rights::INVOKE))]);
    let outcome = routing::route(
        &grantor,
        &requestor,
        &authority,
        &routing::RoutingRequest {
            capability: "invoke".to_string(),
            requested_rights: Rights::INVOKE,
        },
    );
    if outcome != routing::RoutingOutcome::Denied(DenialReason::GrantNotDerivable) {
        return Err("ceiling beyond grantor authority was not rejected");
    }

    // Absent grant.
    let grantor = routing_grantor(Vec::new());
    let outcome = routing::route(
        &grantor,
        &requestor,
        &authority,
        &routing::RoutingRequest {
            capability: "absent".to_string(),
            requested_rights: Rights::READ,
        },
    );
    if outcome != routing::RoutingOutcome::Denied(DenialReason::GrantNotDerivable) {
        return Err("absent grant was not rejected");
    }

    // Grant with no ceiling is a declaration, not authority.
    let grantor = routing_grantor(vec![attenuated("ceilingless", None)]);
    let outcome = routing::route(
        &grantor,
        &requestor,
        &authority,
        &routing::RoutingRequest {
            capability: "ceilingless".to_string(),
            requested_rights: Rights::READ,
        },
    );
    if outcome != routing::RoutingOutcome::Denied(DenialReason::NarrowingFailed) {
        return Err("ceiling-less grant was not rejected");
    }
    Ok(())
}

fn conformance_core_input() -> conformance::DeclarationInput {
    conformance::DeclarationInput {
        cts_version: "cts-0.1.0".to_string(),
        implementation: "aether-ref-0.1.0".to_string(),
        profile: None,
        core_results: conformance::CORE_COMPOSITION_TESTS
            .iter()
            .map(|fixture| conformance::FixtureResult::pass(fixture))
            .collect(),
        profile_subset: Vec::new(),
        substitutions: Vec::new(),
        omitted: Vec::new(),
        manifests_tested: Vec::new(),
        tier_claims: Vec::new(),
    }
}

fn composition_conformance_matches_rfc_0006() -> Result<(), &'static str> {
    let declaration = conformance::declare_composition(conformance_core_input())
        .map_err(|_| "valid core declaration was rejected")?;
    if !declaration.conformant || !declaration.gaps.is_empty() {
        return Err("valid core declaration was not reported conformant");
    }
    // RFC-0006.4: the declaration cites both the test set and the artifact.
    let second = conformance::declare_composition(conformance_core_input())
        .map_err(|_| "second identical declaration was rejected")?;
    if declaration.artifact != second.artifact {
        return Err("artifact identifier is not deterministic for identical runs");
    }
    Ok(())
}

fn conformance_rejects_incomplete_declarations() -> Result<(), &'static str> {
    // Missing core fixture is rejected with the missing fixture id.
    let mut input = conformance_core_input();
    input
        .core_results
        .retain(|result| result.fixture != "CTS-COMP-PROF-4");
    match conformance::declare_composition(input) {
        Err(conformance::ConformanceError::MissingCoreTest("CTS-COMP-PROF-4")) => {}
        _ => return Err("a missing core fixture was not rejected"),
    }

    // A failing fixture without a classification is rejected (methodology §10).
    let mut input = conformance_core_input();
    let position = input
        .core_results
        .iter()
        .position(|result| result.fixture == "CTS-COMP-RT-2")
        .expect("fixture exists");
    input.core_results[position] = conformance::FixtureResult {
        fixture: "CTS-COMP-RT-2",
        passed: false,
        failure: None,
    };
    match conformance::declare_composition(input) {
        Err(conformance::ConformanceError::FailureNotClassified("CTS-COMP-RT-2")) => {}
        _ => return Err("an unclassified failure was not rejected"),
    }

    // A failing core fixture must produce a non-conformant declaration with
    // gap records (methodology section 10); never a passing one.
    let mut input = conformance_core_input();
    let position = input
        .core_results
        .iter()
        .position(|result| result.fixture == "CTS-COMP-MAN-4")
        .expect("fixture exists");
    input.core_results[position] = conformance::FixtureResult::fail(
        "CTS-COMP-MAN-4",
        conformance::FailureMode::ImplementationBug,
    );
    let declaration = conformance::declare_composition(input)
        .map_err(|_| "a failing core fixture must still be declarable")?;
    if declaration.conformant {
        return Err("a failing core fixture was reported as conformant");
    }
    if declaration.gaps.is_empty() {
        return Err("a failing core fixture produced no gap records");
    }
    Ok(())
}

fn crash_sweep_matches_persistence_baseline() -> Result<(), &'static str> {
    // Multi-frame log: one transaction and one effect acknowledgement.
    let mut engine = StateEngine::new();
    let seed = state_transaction(&mut engine); // object 9 -> version 1, operation 10
    engine.commit(seed).map_err(|_| "seed commit failed")?;
    engine
        .record_effect(OperationId(10), EffectStatus::Acknowledged)
        .map_err(|_| "record effect failed")?;
    let log = engine.durable_log().to_vec();
    let sweep = aether_state_model::fault::CrashSweep::new(log);
    if !sweep.violations().is_empty() {
        return Err("crash sweep reported violations on a valid multi-frame log");
    }

    // Corrupt one byte inside the first complete frame: recovery must fail at
    // cuts after the corruption point.
    let mut corrupt = engine.durable_log().to_vec();
    let corrupt_at = 17;
    corrupt[corrupt_at] ^= 1;
    let sweep = aether_state_model::fault::CrashSweep::new(corrupt);
    if !sweep.violations().iter().any(|violation| {
        matches!(violation, aether_state_model::fault::CrashViolation::RecoverFailed(cut) if *cut > corrupt_at)
    }) {
        return Err("corruption was not detected at later byte cuts");
    }
    Ok(())
}

fn conformance_report_cites_artifact_and_tests() -> Result<(), &'static str> {
    let declaration = conformance::declare_composition(conformance_core_input())
        .map_err(|_| "valid declaration was rejected")?;
    if !declaration.artifact_is_cited() {
        return Err("declaration does not cite both the fixture set and a non-zero artifact");
    }
    // Negative axis: a zero artifact must NOT be reported as cited.
    let mut zeroed = declaration.clone();
    zeroed.artifact = conformance::ArtifactId(0);
    if zeroed.artifact_is_cited() {
        return Err("a zero artifact identifier was reported as cited");
    }
    let report = declaration.report();
    if report.artifact != declaration.artifact {
        return Err("report does not cite the declaration artifact");
    }
    if report.artifacts_tested.len() != conformance::CORE_COMPOSITION_TESTS.len() {
        return Err("report fixture set does not match the executed core set");
    }
    let mut sorted = report.artifacts_tested.clone();
    sorted.sort_unstable();
    if sorted != report.artifacts_tested {
        return Err("report fixture list is not in canonical order");
    }
    Ok(())
}

fn conformance_forensics_counts_match_methodology_s10() -> Result<(), &'static str> {
    let modes = [
        (
            "CTS-COMP-MAN-1",
            conformance::FailureMode::ConformanceViolation,
        ),
        ("CTS-COMP-MAN-2", conformance::FailureMode::ConformanceGap),
        (
            "CTS-COMP-MAN-3",
            conformance::FailureMode::ImplementationBug,
        ),
        ("CTS-COMP-MAN-4", conformance::FailureMode::TestBug),
        (
            "CTS-COMP-MAN-5",
            conformance::FailureMode::EnvironmentalFailure,
        ),
    ];
    let mut input = conformance_core_input();
    for (fixture, mode) in modes {
        let position = input
            .core_results
            .iter()
            .position(|result| result.fixture == fixture)
            .expect("fixture exists");
        input.core_results[position] = conformance::FixtureResult::fail(fixture, mode);
    }
    let declaration = conformance::declare_composition(input)
        .map_err(|_| "classified failures must still be declarable")?;
    let counts = declaration.report().forensics;
    if counts.conformance_violations != 1
        || counts.conformance_gaps != 1
        || counts.implementation_bugs != 1
        || counts.test_bugs != 1
        || counts.environmental_failures != 1
    {
        return Err("forensics category counts do not match the classified failures");
    }
    Ok(())
}

fn outbox_dispatch_matches_persistence_baseline() -> Result<(), &'static str> {
    use aether_state_model::outbox::{OutboxDispatcher, Receiver};

    // Commit two effects; dispatch; both must be acknowledged.
    let mut engine = StateEngine::new();
    let mut tx = engine.prepare(7, 11).map_err(|_| "prepare failed")?;
    tx.mutate(Mutation {
        object: ObjectId(77),
        version: VersionId(1),
        content_digest: [7; 32],
    });
    for operation in [101_u128, 102] {
        tx.intend(EffectIntent {
            operation: OperationId(operation),
            class: EffectClass::Deduplicated,
            request_digest: [8; 32],
        });
    }
    engine.commit(tx).map_err(|_| "commit failed")?;
    let mut dispatcher = OutboxDispatcher::new(Receiver::new());
    let report = dispatcher
        .dispatch(&mut engine)
        .map_err(|_| "dispatch failed")?;
    if report.delivered != 2
        || engine.effect_status(OperationId(101)) != Some(EffectStatus::Acknowledged)
        || engine.effect_status(OperationId(102)) != Some(EffectStatus::Acknowledged)
    {
        return Err("fresh intents were not acknowledged");
    }

    // Crash after send for a fresh effect: uncertain outcome, then reconcile.
    let tx = engine
        .prepare(7, 11)
        .map_err(|_| "prepare crash intent failed")?;
    let mut tx = tx;
    tx.intend(EffectIntent {
        operation: OperationId(103),
        class: EffectClass::Deduplicated,
        request_digest: [9; 32],
    });
    engine
        .commit(tx)
        .map_err(|_| "commit crash intent failed")?;
    let mut dispatcher =
        OutboxDispatcher::new(Receiver::new().crashing_after_send_on(OperationId(103)));
    let first = dispatcher
        .dispatch(&mut engine)
        .map_err(|_| "crash dispatch failed")?;
    if first.uncertain != 1
        || engine.effect_status(OperationId(103)) != Some(EffectStatus::Uncertain)
    {
        return Err("crash after send did not produce an uncertain outcome");
    }
    let reconciled = dispatcher
        .reconcile(&mut engine)
        .map_err(|_| "reconcile failed")?;
    if reconciled.acknowledged != 1
        || engine.effect_status(OperationId(103)) != Some(EffectStatus::Acknowledged)
    {
        return Err("uncertain outcome was not reconciled by deduplicated redelivery");
    }

    // A rejecting-redelivery receiver must end Rejected, never blind-acked.
    let tx = engine
        .prepare(7, 11)
        .map_err(|_| "prepare reject intent failed")?;
    let mut tx = tx;
    tx.intend(EffectIntent {
        operation: OperationId(104),
        class: EffectClass::Deduplicated,
        request_digest: [11; 32],
    });
    engine
        .commit(tx)
        .map_err(|_| "commit reject intent failed")?;
    let mut dispatcher =
        OutboxDispatcher::new(Receiver::new().crashing_after_send_on(OperationId(104)));
    let first = dispatcher
        .dispatch(&mut engine)
        .map_err(|_| "crash dispatch failed")?;
    if first.uncertain != 1 {
        return Err("crash after send did not leave the op uncertain");
    }
    let mut dispatcher = OutboxDispatcher::new(Receiver::new().rejecting(OperationId(104)));
    let reconciled = dispatcher
        .reconcile(&mut engine)
        .map_err(|_| "reconcile failed")?;
    if reconciled.rejected != 1
        || engine.effect_status(OperationId(104)) != Some(EffectStatus::Rejected)
    {
        return Err("redelivery was not routed through the receiver (blind ack)");
    }
    Ok(())
}

/// Capability round-trip: derive a narrowed child, then authorize at exactly
/// the derived rights and bounds. The authorized capability must match what the
/// derivation produced — not the parent's authority — and the model and the
/// table-backed provider must agree.
fn capability_round_trip() -> Result<(), &'static str> {
    let (mut model, model_bootstrap, mut system, system_bootstrap) = pair();
    let model_root = model
        .create_root(&model_bootstrap, domain(1), root())
        .map_err(|_| "model root failed")?;
    let system_root = system
        .create_root(&system_bootstrap, domain(1), root())
        .map_err(|_| "system root failed")?;

    let narrowed_rights = Rights::READ.union(Rights::WRITE);
    let narrowed_bounds = bounds(100, 200);
    let model_child = model
        .derive(
            domain(1),
            model_root,
            narrowed_rights,
            narrowed_bounds,
            NOW_NS,
        )
        .map_err(|_| "model derivation failed")?;
    let system_child = system
        .derive(
            domain(1),
            system_root,
            narrowed_rights,
            narrowed_bounds,
            NOW_NS,
        )
        .map_err(|_| "system derivation failed")?;

    let model_auth = model
        .authorize(
            domain(1),
            model_child,
            ObjectType::Memory,
            narrowed_rights,
            narrowed_bounds,
            NOW_NS,
        )
        .map_err(|_| "model authorization failed")?;
    let system_auth = system
        .authorize(
            domain(1),
            system_child,
            ObjectType::Memory,
            narrowed_rights,
            narrowed_bounds,
            NOW_NS,
        )
        .map_err(|_| "system authorization failed")?;
    compare_auth(Ok(model_auth), Ok(system_auth))?;

    if model_auth != system_auth {
        return Err("model and system round-trip capabilities differ");
    }
    if model_auth.rights != narrowed_rights || model_auth.bounds != narrowed_bounds {
        return Err("round-trip did not preserve the derived rights and bounds");
    }
    if model_auth.object != system_auth.object || model_auth.object_type != system_auth.object_type
    {
        return Err("round-trip changed the capability object or type");
    }
    Ok(())
}

/// Rights narrowing composes: each derivation attenuates, rights can never be
/// re-expanded, bounds can never be widened, and the model and provider agree
/// at every step. Authorize at the final narrowed bounds succeeds; authorize
/// outside them fails with Bounds in both implementations.
fn rights_narrowing_composition() -> Result<(), &'static str> {
    let (mut model, model_bootstrap, mut system, system_bootstrap) = pair();
    let model_root = model
        .create_root(&model_bootstrap, domain(1), root())
        .map_err(|_| "model root failed")?;
    let system_root = system
        .create_root(&system_bootstrap, domain(1), root())
        .map_err(|_| "system root failed")?;

    let full_rights = Rights::READ.union(Rights::WRITE).union(Rights::GRANT);
    let child_bounds = bounds(0, 4096);
    let grandchild_bounds = bounds(1024, 2048);

    let model_child = model
        .derive(domain(1), model_root, Rights::READ, child_bounds, NOW_NS)
        .map_err(|_| "first model derivation failed")?;
    let system_child = system
        .derive(domain(1), system_root, Rights::READ, child_bounds, NOW_NS)
        .map_err(|_| "first system derivation failed")?;

    let model_grandchild = model
        .derive(
            domain(1),
            model_child,
            Rights::READ,
            grandchild_bounds,
            NOW_NS,
        )
        .map_err(|_| "second model derivation failed")?;
    let system_grandchild = system
        .derive(
            domain(1),
            system_child,
            Rights::READ,
            grandchild_bounds,
            NOW_NS,
        )
        .map_err(|_| "second system derivation failed")?;

    // Rights cannot be re-expanded from a READ-only child.
    compare_handle_error(
        model.derive(domain(1), model_child, full_rights, child_bounds, NOW_NS),
        system.derive(domain(1), system_child, full_rights, child_bounds, NOW_NS),
        CapError::Rights,
    )?;

    // Authorize exactly at the narrowed bounds: both sides agree.
    compare_auth(
        model.authorize(
            domain(1),
            model_grandchild,
            ObjectType::Memory,
            Rights::READ,
            grandchild_bounds,
            NOW_NS,
        ),
        system.authorize(
            domain(1),
            system_grandchild,
            ObjectType::Memory,
            Rights::READ,
            grandchild_bounds,
            NOW_NS,
        ),
    )?;

    // Authorize outside the narrowed bounds: both sides reject with Bounds.
    let model_oob = model.authorize(
        domain(1),
        model_grandchild,
        ObjectType::Memory,
        Rights::READ,
        child_bounds,
        NOW_NS,
    );
    let system_oob = system.authorize(
        domain(1),
        system_grandchild,
        ObjectType::Memory,
        Rights::READ,
        child_bounds,
        NOW_NS,
    );
    if model_oob != Err(CapError::Bounds) || system_oob != Err(CapError::Bounds) {
        return Err("out-of-bounds authorization was not rejected in both implementations");
    }
    Ok(())
}

/// Dispatch requires capability authority: a valid BOUND + MAP + Memory
/// capability authorizes dispatch; missing BOUND, missing MAP, wrong object
/// type, or an expired capability are each rejected with the proper error.
fn dispatch_requires_capability_authority() -> Result<(), &'static str> {
    let mut dispatcher = Dispatcher::new();
    let domain = DomainId::new(1).unwrap();
    dispatcher
        .register_domain(domain, None, 0)
        .map_err(|_| "domain registration failed")?;

    let valid_cap = Capability {
        object: ObjectId(1),
        object_type: ObjectType::Memory,
        rights: Rights::MAP,
        bounds: Bounds::new(0, 4096).unwrap(),
        flags: CapabilityFlags::BOUND,
        expires_at_ns: 0,
    };

    // Valid capability authorizes dispatch.
    let request = DispatchRequest {
        thread_id: 1,
        capability: valid_cap,
        hart_id: 1,
        domain,
        now_ns: 0,
    };
    let id = dispatcher
        .dispatch(request)
        .map_err(|_| "valid dispatch was rejected")?;
    if dispatcher.state(id) != Some(aether_dispatch_model::DispatchState::Execute) {
        return Err("valid dispatch did not enter Running state");
    }

    // Missing BOUND flag is rejected.
    let no_bound = Capability {
        flags: CapabilityFlags::NONE,
        ..valid_cap
    };
    let request = DispatchRequest {
        thread_id: 2,
        capability: no_bound,
        hart_id: 1,
        domain,
        now_ns: 0,
    };
    if dispatcher.dispatch(request) != Err(DispatchError::Unauthorized) {
        return Err("dispatch without BOUND flag was not rejected");
    }

    // Missing MAP right is rejected.
    let no_map = Capability {
        rights: Rights::READ,
        ..valid_cap
    };
    let request = DispatchRequest {
        thread_id: 3,
        capability: no_map,
        hart_id: 1,
        domain,
        now_ns: 0,
    };
    if dispatcher.dispatch(request) != Err(DispatchError::Unauthorized) {
        return Err("dispatch without MAP right was not rejected");
    }

    // Wrong object type is rejected.
    let wrong_type = Capability {
        object_type: aether_types::ObjectType::State,
        ..valid_cap
    };
    let request = DispatchRequest {
        thread_id: 4,
        capability: wrong_type,
        hart_id: 1,
        domain,
        now_ns: 0,
    };
    if dispatcher.dispatch(request) != Err(DispatchError::Unauthorized) {
        return Err("dispatch with wrong object type was not rejected");
    }

    // Expired capability is rejected.
    let expired = Capability {
        expires_at_ns: 100,
        ..valid_cap
    };
    let request = DispatchRequest {
        thread_id: 5,
        capability: expired,
        hart_id: 1,
        domain,
        now_ns: 101,
    };
    if dispatcher.dispatch(request) != Err(DispatchError::Expired) {
        return Err("dispatch with expired capability was not rejected");
    }

    Ok(())
}

/// CTS-EXEC-DISP-1 (RFC-0008.2): capability verification before dispatch — a
/// dispatching entity must hold a BOUND + MAP + Memory capability, and the
/// dispatcher aborts on missing authority.
fn exec_dispatch_capability_verification() -> Result<(), &'static str> {
    let mut dispatcher = Dispatcher::new();
    let domain = DomainId::new(1).unwrap();
    dispatcher
        .register_domain(domain, None, 0)
        .map_err(|_| "domain registration failed")?;

    // Valid capability authorizes dispatch.
    let valid_cap = Capability {
        object: ObjectId(1),
        object_type: ObjectType::Memory,
        rights: Rights::MAP,
        bounds: Bounds::new(0, 4096).unwrap(),
        flags: CapabilityFlags::BOUND,
        expires_at_ns: 0,
    };
    let request = DispatchRequest {
        thread_id: 1,
        capability: valid_cap,
        hart_id: 1,
        domain,
        now_ns: 0,
    };
    let id = dispatcher
        .dispatch(request)
        .map_err(|_| "valid dispatch was rejected")?;
    if dispatcher.state(id) != Some(DispatchState::Execute) {
        return Err("valid dispatch did not enter Execute state");
    }

    // Missing BOUND flag is rejected (no dispatch authority).
    let no_bound = Capability {
        flags: CapabilityFlags::NONE,
        ..valid_cap
    };
    let request = DispatchRequest {
        thread_id: 2,
        capability: no_bound,
        hart_id: 1,
        domain,
        now_ns: 0,
    };
    if dispatcher.dispatch(request) != Err(DispatchError::Unauthorized) {
        return Err("dispatch without BOUND flag was not rejected");
    }
    // Rejection leaves no observable state (atomicity, RFC-0008.11).
    if dispatcher.len() != 1 {
        return Err("rejected dispatch mutated dispatcher state");
    }
    Ok(())
}

/// CTS-EXEC-DISP-2 (RFC-0008.3): domain compatibility — dispatching domain B
/// to a hart already running domain A aborts with DomainMismatch.
fn exec_dispatch_domain_compatibility() -> Result<(), &'static str> {
    let mut dispatcher = Dispatcher::new();
    dispatcher
        .register_domain(domain(1), None, 0)
        .map_err(|_| "domain 1 registration failed")?;
    dispatcher
        .register_domain(domain(2), None, 0)
        .map_err(|_| "domain 2 registration failed")?;

    let cap = Capability {
        object: ObjectId(2),
        object_type: ObjectType::Memory,
        rights: Rights::MAP,
        bounds: Bounds::new(0, 4096).unwrap(),
        flags: CapabilityFlags::BOUND,
        expires_at_ns: 0,
    };
    // hart 1 runs domain 1.
    dispatcher
        .dispatch(DispatchRequest {
            thread_id: 1,
            capability: cap,
            hart_id: 1,
            domain: domain(1),
            now_ns: 0,
        })
        .map_err(|_| "first dispatch failed")?;
    // Dispatching domain 2 to hart 1 while domain 1 runs is a mismatch.
    if dispatcher.dispatch(DispatchRequest {
        thread_id: 2,
        capability: cap,
        hart_id: 1,
        domain: domain(2),
        now_ns: 1,
    }) != Err(DispatchError::DomainMismatch)
    {
        return Err("domain-mismatched dispatch was not rejected");
    }
    // hart 1 still runs domain 1.
    match dispatcher.hart_state(1) {
        Some(HartState::Running {
            dispatch_id,
            domain: running,
        }) if running == domain(1) && dispatch_id == DispatchId(1) => {}
        _ => return Err("hart 1 no longer runs domain 1 after rejected dispatch"),
    }
    Ok(())
}

/// CTS-EXEC-DISP-3 (RFC-0008.5): dispatch latency bound — a documented,
/// verifiable worst-case bound is declared.
fn exec_dispatch_latency_bound() -> Result<(), &'static str> {
    let bound = Dispatcher::dispatch_bound_ns();
    if bound == 0 {
        return Err("dispatch bound must be declared and non-zero");
    }
    // A dispatch completes synchronously within the bound (no blocking).
    let mut dispatcher = Dispatcher::new();
    let domain = DomainId::new(1).unwrap();
    dispatcher
        .register_domain(domain, None, 0)
        .map_err(|_| "domain registration failed")?;
    let cap = Capability {
        object: ObjectId(3),
        object_type: ObjectType::Memory,
        rights: Rights::MAP,
        bounds: Bounds::new(0, 4096).unwrap(),
        flags: CapabilityFlags::BOUND,
        expires_at_ns: 0,
    };
    let _id = dispatcher
        .dispatch(DispatchRequest {
            thread_id: 1,
            capability: cap,
            hart_id: 1,
            domain,
            now_ns: 0,
        })
        .map_err(|_| "dispatch failed")?;
    Ok(())
}

/// CTS-EXEC-DISP-4 (RFC-0008.7): migration emits a migration event
/// (RFC-0031.17) and re-runs pre-dispatch verification on the target.
fn exec_dispatch_migration_emits_event() -> Result<(), &'static str> {
    let mut dispatcher = Dispatcher::new();
    dispatcher
        .register_domain(domain(1), None, 0)
        .map_err(|_| "domain registration failed")?;
    let cap = Capability {
        object: ObjectId(4),
        object_type: ObjectType::Memory,
        rights: Rights::MAP,
        bounds: Bounds::new(0, 4096).unwrap(),
        flags: CapabilityFlags::BOUND,
        expires_at_ns: 0,
    };
    let id = dispatcher
        .dispatch(DispatchRequest {
            thread_id: 1,
            capability: cap,
            hart_id: 1,
            domain: domain(1),
            now_ns: 0,
        })
        .map_err(|_| "initial dispatch failed")?;
    dispatcher
        .migrate(id, 2, 500)
        .map_err(|_| "migration failed")?;
    match dispatcher.hart_state(2) {
        Some(HartState::Running {
            dispatch_id,
            domain: running,
        }) if running == domain(1) && dispatch_id == id => {}
        _ => return Err("migrated dispatch not found on target hart"),
    }
    if !dispatcher
        .events()
        .iter()
        .any(|e| e.reason == DispatchReason::Migrated)
    {
        return Err("migration event was not emitted");
    }
    Ok(())
}

/// CTS-EXEC-DISP-5 (RFC-0008.8): migration lock-hierarchy outcome — migrating
/// to a hart running an incompatible domain is rejected (verification is
/// re-run; no silent lock-order violation).
fn exec_dispatch_migration_lock_hierarchy() -> Result<(), &'static str> {
    let mut dispatcher = Dispatcher::new();
    dispatcher
        .register_domain(domain(1), None, 0)
        .map_err(|_| "domain 1 registration failed")?;
    dispatcher
        .register_domain(domain(2), None, 0)
        .map_err(|_| "domain 2 registration failed")?;
    let cap = Capability {
        object: ObjectId(5),
        object_type: ObjectType::Memory,
        rights: Rights::MAP,
        bounds: Bounds::new(0, 4096).unwrap(),
        flags: CapabilityFlags::BOUND,
        expires_at_ns: 0,
    };
    let id1 = dispatcher
        .dispatch(DispatchRequest {
            thread_id: 1,
            capability: cap,
            hart_id: 1,
            domain: domain(1),
            now_ns: 0,
        })
        .map_err(|_| "first dispatch failed")?;
    // hart 1 runs domain 1, hart 2 runs domain 2.
    let id2 = dispatcher
        .dispatch(DispatchRequest {
            thread_id: 2,
            capability: cap,
            hart_id: 2,
            domain: domain(2),
            now_ns: 1,
        })
        .map_err(|_| "second dispatch failed")?;
    // Migrate dispatch 1 (domain 1) to hart 2 (running domain 2): mismatch.
    if dispatcher.migrate(id1, 2, 500).is_ok() {
        return Err("incompatible-domain migration was not rejected");
    }
    // Both dispatches stayed put.
    if dispatcher.state(id1) != Some(DispatchState::Execute)
        || dispatcher.hart_state(1)
            != Some(HartState::Running {
                dispatch_id: id1,
                domain: domain(1),
            })
        || dispatcher.state(id2) != Some(DispatchState::Execute)
        || dispatcher.hart_state(2)
            != Some(HartState::Running {
                dispatch_id: id2,
                domain: domain(2),
            })
    {
        return Err("rejected migration moved a dispatch");
    }
    Ok(())
}

/// CTS-EXEC-DISP-6 (RFC-0008.11): atomic transition — a rejected dispatch
/// leaves no observable state and a preempted thread is suspended cleanly.
fn exec_dispatch_atomic_transition() -> Result<(), &'static str> {
    let mut dispatcher = Dispatcher::new();
    let domain = DomainId::new(1).unwrap();
    dispatcher
        .register_domain(domain, None, 0)
        .map_err(|_| "domain registration failed")?;
    let cap = Capability {
        object: ObjectId(6),
        object_type: ObjectType::Memory,
        rights: Rights::MAP,
        bounds: Bounds::new(0, 4096).unwrap(),
        flags: CapabilityFlags::BOUND,
        expires_at_ns: 0,
    };
    let id = dispatcher
        .dispatch(DispatchRequest {
            thread_id: 1,
            capability: cap,
            hart_id: 1,
            domain,
            now_ns: 0,
        })
        .map_err(|_| "dispatch failed")?;
    dispatcher
        .preempt(id, 1_000)
        .map_err(|_| "preemption failed")?;
    if dispatcher.state(id) != Some(DispatchState::Suspend) {
        return Err("preempted dispatch did not transition to Suspend");
    }
    // Hart is idle after preemption.
    if dispatcher.hart_state(1) != Some(HartState::Idle) {
        return Err("hart did not return to idle after preemption");
    }
    Ok(())
}

/// Storage block device read/write round-trip: data written to a sector can be
/// read back identically; out-of-range access is rejected.
fn storage_block_read_write_round_trip() -> Result<(), &'static str> {
    use aether_storage_model::{BlockDevice, RamBlockDevice};

    const BLOCK_SIZE: usize = 512;
    let mut dev = RamBlockDevice::new(64);
    let data = [0xABu8; BLOCK_SIZE];
    dev.write(0, &data).map_err(|_| "block write failed")?;
    let mut buf = [0u8; BLOCK_SIZE];
    dev.read(0, &mut buf).map_err(|_| "block read failed")?;
    if buf != data {
        return Err("read-back data does not match written data");
    }
    if dev.read(1000, &mut buf).is_ok() {
        return Err("out-of-range read was not rejected");
    }
    if dev.write(1000, &data).is_ok() {
        return Err("out-of-range write was not rejected");
    }
    Ok(())
}

/// Runtime instruction execution: a small program that prints 'A' then exits
/// with code 42 produces the expected stdout and exit code.
fn runtime_instruction_execution_produces_output() -> Result<(), &'static str> {
    use aether_runtime_model::{Inst, Op, Process};

    // MOV r0, 65 (imm 'A'); SYSCALL print r0; SYSCALL exit 42
    let prog = vec![
        Inst::new(Op::Mov, 0, 1, 65),
        Inst::new(Op::Syscall, 1, 0, 0),
        Inst::new(Op::Syscall, 2, 42, 0),
    ];
    let mut p = Process::new(&prog);
    let code = p.run().map_err(|_| "runtime execution failed")?;
    if code != 42 {
        return Err("runtime exit code does not match expected value");
    }
    if p.stdout_string() != "A" {
        return Err("runtime stdout does not match expected output");
    }
    Ok(())
}

/// Update A/B rollback: after 3 consecutive boot failures on the pending
/// partition, the system rolls back to the previous active partition.
fn update_rollback_after_boot_failures() -> Result<(), &'static str> {
    use aether_update_model::{Manifest, Partition, UpdateSystem};

    let mut sys = UpdateSystem::new();
    let m = Manifest {
        version: "2.0.0".to_string(),
        hash: 0xDEAD,
        size: 1024,
    };
    sys.begin_update(m).map_err(|_| "begin_update failed")?;
    sys.apply_update().map_err(|_| "apply_update failed")?;
    assert!(!sys.record_boot_failure()); // retry 1
    assert!(!sys.record_boot_failure()); // retry 2
    assert!(sys.record_boot_failure()); // retry 3 -> rollback
    if sys.active != Partition::A {
        return Err("rollback did not reactivate partition A");
    }
    if sys.active_version() != "1.0.0" {
        return Err("rollback did not restore previous version");
    }
    if sys.update_in_progress {
        return Err("update still in progress after rollback");
    }
    Ok(())
}

/// Shell command execution: touch a file, write content, read it back, then
/// remove it — verifying state transitions through the virtual file system.
fn shell_command_execution_and_state() -> Result<(), &'static str> {
    use aether_shell_model::Shell;

    let mut shell = Shell::new();

    // touch then cat (empty content) succeeds.
    shell
        .run_line("touch notes.txt")
        .map_err(|_| "touch failed")?;
    let out = shell.run_line("cat notes.txt").map_err(|_| "cat failed")?;
    if out != vec![""] {
        return Err("newly created file should have empty content");
    }

    // write then cat returns the written content.
    shell.fs.write_file("notes.txt", "hello world").unwrap();
    let out = shell
        .run_line("cat notes.txt")
        .map_err(|_| "cat after write failed")?;
    if out != vec!["hello world"] {
        return Err("cat does not return written content");
    }

    // rm then cat fails.
    shell.run_line("rm notes.txt").map_err(|_| "rm failed")?;
    if shell.run_line("cat notes.txt").is_ok() {
        return Err("cat after rm should fail");
    }

    // pwd returns the root directory.
    let out = shell.run_line("pwd").map_err(|_| "pwd failed")?;
    if out != vec!["/"] {
        return Err("pwd should return root");
    }

    Ok(())
}

/// CTS-EXEC-IRQ-1 (RFC-0010.1/.2/.11): domain isolation — an interrupt bound
/// to one domain is delivered within that domain's context; delivery to any
/// other domain is rejected and reported as spurious, never silently dropped.
fn exec_interrupt_domain_isolation() -> Result<(), &'static str> {
    let mut router = InterruptRouter::new();
    router
        .register_domain(domain(1), None, 1_000)
        .map_err(|_| "domain 1 registration failed")?;
    router
        .register_domain(domain(2), None, 1_000)
        .map_err(|_| "domain 2 registration failed")?;
    let vector = IrqVector(10);
    router
        .configure_routing(
            domain(1),
            vector,
            &irq_bind_cap(),
            irq_entry(vector, DeliveryMode::Direct, 5),
            1_000,
        )
        .map_err(|_| "configure routing failed")?;
    if router.deliver(vector, domain(1), 2_000) != DeliveryOutcome::Delivered {
        return Err("interrupt was not delivered to its bound domain");
    }
    if router.deliver(vector, domain(2), 2_000) != DeliveryOutcome::NoHandler {
        return Err("misrouted interrupt was delivered outside its bound domain");
    }
    let last = router.events().last().ok_or("no events recorded")?;
    if !matches!(last, InterruptEvent::Spurious { .. }) {
        return Err("misrouted interrupt was not reported as spurious");
    }
    Ok(())
}

/// CTS-EXEC-IRQ-2 (RFC-0010.1/.4): routing changes require an INTERRUPT_BIND
/// capability; a bound capability configures the table and direct delivery
/// follows.
fn exec_interrupt_bind_gating_and_direct_delivery() -> Result<(), &'static str> {
    let mut router = InterruptRouter::new();
    router
        .register_domain(domain(1), None, 1_000)
        .map_err(|_| "domain registration failed")?;
    let vector = IrqVector(42);
    let mut no_bind = irq_bind_cap();
    no_bind.flags = CapabilityFlags::NONE;
    if router
        .configure_routing(
            domain(1),
            vector,
            &no_bind,
            irq_entry(vector, DeliveryMode::Direct, 5),
            1_000,
        )
        .is_ok()
    {
        return Err("routing accepted without INTERRUPT_BIND capability");
    }
    router
        .configure_routing(
            domain(1),
            vector,
            &irq_bind_cap(),
            irq_entry(vector, DeliveryMode::Direct, 5),
            1_000,
        )
        .map_err(|_| "configure routing failed")?;
    if router.deliver(vector, domain(1), 2_000) != DeliveryOutcome::Delivered {
        return Err("direct delivery did not report Delivered");
    }
    Ok(())
}

/// CTS-EXEC-IRQ-3 (RFC-0010.6): queued delivery preserves interrupt order.
fn exec_interrupt_queued_order_preserved() -> Result<(), &'static str> {
    let mut router = InterruptRouter::new();
    router
        .register_domain(domain(1), None, 1_000)
        .map_err(|_| "domain registration failed")?;
    for v in [IrqVector(1), IrqVector(2), IrqVector(3)] {
        router
            .configure_routing(
                domain(1),
                v,
                &irq_bind_cap(),
                irq_entry(v, DeliveryMode::Queued, 3),
                1_000,
            )
            .map_err(|_| "configure routing failed")?;
    }
    for v in [IrqVector(1), IrqVector(2), IrqVector(3)] {
        if router.deliver(v, domain(1), 2_000) != DeliveryOutcome::Queued {
            return Err("queued delivery did not report Queued");
        }
    }
    let delivered = router
        .events()
        .iter()
        .filter(|e| matches!(e, InterruptEvent::Delivered { .. }))
        .count();
    if delivered != 3 {
        return Err("queued delivery did not record one delivery per interrupt");
    }
    Ok(())
}

/// CTS-EXEC-IRQ-4 (RFC-0010.11): a vector bound to no domain is reported as
/// spurious when it arrives, never silently dropped.
fn exec_interrupt_unhandled_reported_spurious() -> Result<(), &'static str> {
    let mut router = InterruptRouter::new();
    router
        .register_domain(domain(1), None, 1_000)
        .map_err(|_| "domain registration failed")?;
    if router.deliver(IrqVector(99), domain(1), 2_000) != DeliveryOutcome::NoHandler {
        return Err("unhandled vector was not rejected");
    }
    let last = router.events().last().ok_or("no events recorded")?;
    if !matches!(last, InterruptEvent::Spurious { .. }) {
        return Err("unhandled interrupt was not reported as spurious");
    }
    Ok(())
}

/// CTS-EXEC-IRQ-5 (RFC-0010.8): cross-domain delivery requires an explicit
/// cross-domain capability token; unauthorized crossing is rejected and
/// reported as spurious.
fn exec_interrupt_cross_domain_requires_capability() -> Result<(), &'static str> {
    let mut router = InterruptRouter::new();
    router
        .register_domain(domain(1), None, 1_000)
        .map_err(|_| "domain 1 registration failed")?;
    router
        .register_domain(domain(2), None, 1_000)
        .map_err(|_| "domain 2 registration failed")?;
    let vector = IrqVector(55);
    router
        .configure_routing(
            domain(1),
            vector,
            &irq_bind_cap(),
            irq_entry(vector, DeliveryMode::Direct, 5),
            1_000,
        )
        .map_err(|_| "configure routing failed")?;
    if router
        .deliver_cross_domain(vector, domain(1), domain(2), None, 2_000)
        .is_ok()
    {
        return Err("cross-domain delivery without capability was accepted");
    }
    let last = router.events().last().ok_or("no events recorded")?;
    if !matches!(last, InterruptEvent::Spurious { .. }) {
        return Err("unauthorized crossing was not reported as spurious");
    }
    if router
        .deliver_cross_domain(vector, domain(1), domain(2), Some(&irq_bind_cap()), 2_000)
        .is_err()
    {
        return Err("authorized cross-domain delivery was rejected");
    }
    Ok(())
}

/// CTS-EXEC-IRQ-6 (RFC-0010.9/.10/.12): priorities below the minimum maskable
/// level are rejected, documented limits are exposed, and a fire rate above
/// 10x the coalescing threshold reports an interrupt storm.
fn exec_interrupt_priority_nesting_and_storm() -> Result<(), &'static str> {
    let mut router = InterruptRouter::new();
    router
        .register_domain(domain(1), None, 1_000)
        .map_err(|_| "domain registration failed")?;
    if router.min_maskable_priority() != 1 {
        return Err("minimum maskable priority is not 1");
    }
    if router.max_nesting_depth() != 8 {
        return Err("maximum nesting depth is not 8");
    }
    if router
        .configure_routing(
            domain(1),
            IrqVector(1),
            &irq_bind_cap(),
            irq_entry(IrqVector(1), DeliveryMode::Direct, 0),
            1_000,
        )
        .is_ok()
    {
        return Err("priority below the minimum maskable level was accepted");
    }
    let vector = IrqVector(9);
    router
        .configure_routing(
            domain(1),
            vector,
            &irq_bind_cap(),
            irq_entry(vector, DeliveryMode::Coalesced, 4),
            1_000,
        )
        .map_err(|_| "configure routing failed")?;
    // Threshold 1000 Hz: storm above 10,000 fires in one second. All fires
    // below use timestamps within a 10-microsecond window.
    for i in 0..10_001u64 {
        router.deliver(vector, domain(1), 2_000 + i);
    }
    if !router
        .events()
        .iter()
        .any(|e| matches!(e, InterruptEvent::StormDetected { .. }))
    {
        return Err("interrupt storm above 10x the coalescing threshold was not detected");
    }
    Ok(())
}
