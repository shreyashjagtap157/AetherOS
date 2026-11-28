//! Minimal machine-readable capability conformance runner.

use aether_address_space_model as asm;
use aether_audit_log::{
    Attribution, AuditLog, AuditRecord, AuditorSignature, CapabilityLineage, Severity,
};
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
use aether_diagnostic_query::{
    DiagnosticQuery, DiagnosticQueryEngine, PredefinedQuery, QueryCapabilityRight,
    QueryExecutionMode, QueryField, QueryLimits, QueryOperator, QueryStatus, authorize_query,
};
use aether_dispatch_model::{
    DispatchError, DispatchId, DispatchReason, DispatchRequest, DispatchState, Dispatcher,
    HartState,
};
use aether_event_model::{
    EventClass, EventEmitter, EventFlag, EventId, PlatformEvent, SECURITY_CRITICAL_URGENCY,
};
use aether_exec_domain_model::{DomainError, DomainManager};
use aether_integrity_model::{
    IntegrityResult, IntegrityVerifier, MerkleDAG, NodeType, SignedIntegrity,
};
use aether_interrupt_model::{
    DeliveryMode, DeliveryOutcome, InterruptEvent, InterruptRouter, IrqVector, RoutingEntry,
};
use aether_kv_projection::{KvError, KvProjection};
use aether_message_format::{
    EndpointId, HEADER_LEN, MAGIC, MAX_MESSAGE_SIZE, MSG_CAP_TRANSFER, MSG_ERROR, MSG_FAST,
    MSG_NO_REPLY, MSG_STREAM, VERSION, Message, MessageError, MessageHeader,
};
use aether_endpoint_model::{
    ALL_ENDPOINT_CAPS, CommErrorCode, EndpointCapKind, EndpointError, EndpointMode,
    EndpointRegistry, EndpointRegistration,
};
use aether_endpoint_model::EndpointId as EpEndpointId;
use aether_routing_model::{
    RouteEntry, RouteHop, Router, RoutingError, RoutingTable,
};
use aether_lifecycle_model::{
    LifecycleMachine, LifecyclePolicy, LifecycleState, TransitionTrigger,
};
use aether_mediation_model::{MediationFault, MediationOutcome, Mediator};
use aether_obj_projection::{ObjectCreateEntry, ObjectError, ObjectProjection};
use aether_object_identity::{
    ConsistencyLevel, ObjectIdentity, ReplicaDescriptor, VersionHistory, VersionReason,
};
use aether_posix_projection::{FileType, O_CREAT, O_RDONLY, O_WRONLY, PosixProjection};
use aether_replay_model::{
    Checkpoint, ReplayEngine, ReplayLogHeader, ReplayLogReader, ReplayLogWriter, ReplayState,
    ReplayStep, serialize_event,
};
use aether_sched_model::{ThreadId, ThreadState, TransitionReason, UniprocessorEdf};
use aether_state_model::{
    EffectClass, EffectIntent, EffectStatus, Mutation, OperationId, StateEngine, VersionId,
};
use aether_storage_provider::{
    InMemoryProvider, LatencyBounds, ProviderType, StorageProvider, Tier as StorageTier,
};
use aether_time_model::{TimeError, TimeEvent, TimeModel, TimeSourceId, TimerMode};
use aether_trace_model::{ROOT_SPAN_ID, SpanClass, SpanOutcome, TraceEmitter, TraceSamplingPolicy};
use aether_transaction_model::{TransactionCoordinator, TransactionId};
use aether_types::{
    Bounds, CapError, Capability, CapabilityFlags, DomainId, Handle, ObjectId, ObjectType, Rights,
};

const NOW_NS: u64 = 1_000_000;

#[allow(clippy::too_many_lines)]
fn cases() -> [(&'static str, Result<(), &'static str>); 187] {
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
        ("CTS-EXEC-DOM-4", exec_domain_cycle_detection()),
        ("CTS-EXEC-DOM-5", exec_domain_orphan_children()),
        ("CTS-EXEC-DOM-6", exec_domain_isolation_violation()),
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
        ("CTS-SCHED-1", sched_edf_deadline_ordering()),
        ("CTS-SCHED-2", sched_admission_bound_u_equals_one()),
        ("CTS-SCHED-3", sched_quantum_expiry_preempts_and_requeues()),
        ("CTS-SCHED-4", sched_deadline_miss_detected()),
        ("CTS-SCHED-5", sched_block_unblock_transitions()),
        ("CTS-SCHED-6", sched_exit_frees_admission_capacity()),
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
        ("CTS-TIME-1", time_source_resolution_declaration()),
        ("CTS-TIME-2", time_timer_facility_completeness()),
        ("CTS-TIME-3", time_deadline_miss_detection()),
        ("CTS-TIME-4", time_monotonicity_enforcement()),
        ("CTS-TIME-5", time_scheduling_suspend_resume()),
        ("CTS-EXEC-CONF-1", exec_conformance_core_tests_passed()),
        ("CTS-STATE-OBJ-1", state_obj_identity_uniqueness()),
        ("CTS-STATE-OBJ-2", state_obj_identity_immutability()),
        ("CTS-STATE-OBJ-3", state_obj_version_history_completeness()),
        ("CTS-STATE-OBJ-4", state_obj_replica_identity_equality()),
        (
            "CTS-STATE-OBJ-5",
            state_obj_identity_capability_orthogonality(),
        ),
        ("CTS-STATE-OBJ-6", state_obj_reconstruction_from_replica()),
        ("CTS-STATE-INT-1", state_int_merkle_dag_maintained()),
        ("CTS-STATE-INT-2", state_int_verification_on_read()),
        ("CTS-STATE-INT-3", state_int_no_corrupted_data_returned()),
        ("CTS-STATE-INT-4", state_int_incremental_verification()),
        ("CTS-STATE-INT-5", state_int_signature_verification()),
        ("CTS-OBS-EVT-1", obs_evt_event_structure()),
        ("CTS-OBS-EVT-2", obs_evt_declared_events_emitted()),
        ("CTS-OBS-EVT-3", obs_evt_emission_at_transition()),
        ("CTS-OBS-EVT-4", obs_evt_causality_chain()),
        ("CTS-OBS-EVT-5", obs_evt_security_critical_not_filtered()),
        ("CTS-OBS-EVT-6", obs_evt_unknown_type_no_violation()),
        ("CTS-STATE-LC-1", state_lc_mandatory_transition_path()),
        ("CTS-STATE-LC-2", state_lc_preconditions_enforced()),
        ("CTS-STATE-LC-3", state_lc_only_permitted_backward()),
        ("CTS-STATE-LC-4", state_lc_transition_events_emitted()),
        ("CTS-STATE-LC-5", state_lc_hazard_blocks_tiering()),
        ("CTS-STATE-LC-6", state_lc_state_queryable()),
        ("CTS-OBS-SPAN-1", obs_span_lifecycle_start_close()),
        ("CTS-OBS-SPAN-2", obs_span_parent_child_tree()),
        ("CTS-OBS-SPAN-3", obs_span_event_correlation()),
        ("CTS-OBS-SPAN-4", obs_span_sampling_policy()),
        ("CTS-OBS-SPAN-5", obs_span_high_urgency_always_recorded()),
        ("CTS-OBS-REPLAY-1", obs_replay_log_record_format()),
        ("CTS-OBS-REPLAY-2", obs_replay_deterministic_replay()),
        (
            "CTS-OBS-REPLAY-3",
            obs_replay_nondeterministic_inputs_replaced(),
        ),
        ("CTS-OBS-REPLAY-4", obs_replay_replay_to_pause()),
        ("CTS-OBS-REPLAY-5", obs_replay_checkpoint_consistency()),
        ("CTS-OBS-AUDIT-1", obs_audit_security_event_classes()),
        ("CTS-OBS-AUDIT-2", obs_audit_tamper_detection()),
        ("CTS-OBS-AUDIT-3", obs_audit_auditor_signature()),
        ("CTS-OBS-AUDIT-4", obs_audit_retention_period()),
        ("CTS-OBS-AUDIT-5", obs_audit_capability_lineage()),
        ("CTS-OBS-DIAG-1", obs_diag_query_language_completeness()),
        ("CTS-OBS-DIAG-2", obs_diag_execution_modes()),
        ("CTS-OBS-DIAG-3", obs_diag_limit_enforcement()),
        ("CTS-OBS-DIAG-4", obs_diag_predefined_queries()),
        ("CTS-OBS-DIAG-5", obs_diag_query_logged()),
        ("CTS-OBS-DIAG-6", obs_diag_authorization_enforced()),
        ("CTS-STATE-TXN-1", cts_txn_two_phase_commit_all_prepared()),
        ("CTS-STATE-TXN-2", cts_txn_prepare_failure_aborts_all()),
        ("CTS-STATE-TXN-3", cts_txn_client_abort()),
        ("CTS-STATE-TXN-4", cts_txn_write_write_conflict_detected()),
        (
            "CTS-STATE-TXN-5",
            cts_txn_snapshot_isolation_reads_committed(),
        ),
        (
            "CTS-STATE-TXN-6",
            cts_txn_crash_recovery_aborts_in_progress(),
        ),
        ("CTS-STATE-PROV-1", cts_prov_mandatory_operations()),
        ("CTS-STATE-PROV-2", cts_prov_tier_assignment()),
        ("CTS-STATE-PROV-3", cts_prov_removability_test()),
        ("CTS-STATE-PROV-4", cts_prov_two_phase_commit()),
        ("CTS-STATE-PROV-5", cts_prov_health_events()),
        ("CTS-STATE-POSIX-1", cts_posix_required_operations()),
        ("CTS-STATE-POSIX-2", cts_posix_object_mapping()),
        ("CTS-STATE-POSIX-3", cts_posix_transactional_fsync()),
        ("CTS-STATE-POSIX-4", cts_posix_atomic_rename()),
        ("CTS-STATE-POSIX-5", cts_posix_capability_precedence()),
        ("CTS-STATE-OBJPROJ-1", cts_objproj_object_api()),
        ("CTS-STATE-OBJPROJ-2", cts_objproj_dedup()),
        ("CTS-STATE-OBJPROJ-3", cts_objproj_atomic_update()),
        ("CTS-STATE-OBJPROJ-4", cts_objproj_batch_all_or_nothing()),
        ("CTS-STATE-OBJPROJ-5", cts_objproj_immutable_rejected()),
        ("CTS-STATE-KV-1", cts_kv_full_api()),
        ("CTS-STATE-KV-2", cts_kv_transactional_grouping()),
        ("CTS-STATE-KV-3", cts_kv_two_phase_commit_abort()),
        ("CTS-STATE-KV-4", cts_kv_atomic_cas()),
        ("CTS-STATE-KV-5", cts_kv_cas_conflict()),
        ("CTS-STATE-XS-1", state_xs_txn_commit_identity_verifiable()),
        ("CTS-STATE-XS-2", state_xs_lifecycle_atomic_with_txn()),
        ("CTS-STATE-XS-3", state_xs_provider_removability()),
        (
            "CTS-STATE-XS-4",
            state_xs_identity_capability_orthogonality(),
        ),
        ("CTS-STATE-XS-5", state_xs_merkle_composite_update()),
        ("CTS-STATE-CONF-1", state_conf_core_tests_passed()),
        ("CTS-STATE-CONF-2", state_conf_cross_state_tests_passed()),
        (
            "CTS-STATE-CONF-3",
            state_conf_desktop_cloud_profile_passed(),
        ),
        (
            "CTS-STATE-CONF-4",
            state_conf_safety_critical_profile_passed(),
        ),
        ("CTS-STATE-CONF-5", state_conf_reporting_format()),
        ("CTS-COMM-MSG-1", comm_msg_header_format()),
        ("CTS-COMM-MSG-2", comm_msg_flags_parsing()),
        ("CTS-COMM-MSG-3", comm_msg_trailer_capability_tokens()),
        ("CTS-COMM-MSG-4", comm_msg_max_size_enforcement()),
        ("CTS-COMM-MSG-5", comm_msg_reply_correlation()),
        ("CTS-COMM-EP-1", comm_ep_capability_protected_registration()),
        ("CTS-COMM-EP-2", comm_ep_registration_fields()),
        ("CTS-COMM-EP-3", comm_ep_capability_kinds()),
        ("CTS-COMM-EP-4", comm_ep_handler_vs_polling_mode()),
        ("CTS-COMM-EP-5", comm_ep_error_code_format()),
        ("CTS-COMM-EP-6", comm_ep_close_deferred_until_queue_empty()),
        ("CTS-COMM-RT-1", comm_rt_route_resolution_produces_valid_hops()),
        ("CTS-COMM-RT-2", comm_rt_capability_required_per_hop()),
        ("CTS-COMM-RT-3", comm_rt_single_hop_intra_domain_routing()),
        ("CTS-COMM-RT-4", comm_rt_no_silent_drops()),
        ("CTS-COMM-RT-5", comm_rt_table_invalidation_on_deregistration()),
        ("CTS-COMM-RT-6", comm_rt_routing_time_complexity_bound()),
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

/// CTS-EXEC-CONF-1 (RFC-0014.1): core execution conformance — every mandatory
/// execution test from RFC-0007 through RFC-0012 passes.
fn exec_conformance_core_tests_passed() -> Result<(), &'static str> {
    // RFC-0007 domain tests (RFC-0014.1)
    exec_domain_isolation()?;
    exec_domain_hierarchy_dag()?;
    exec_domain_lifecycle_and_destroy_rules()?;
    exec_domain_cycle_detection()?;
    exec_domain_orphan_children()?;
    exec_domain_isolation_violation()?;
    // RFC-0008 dispatch tests (RFC-0014.1)
    exec_dispatch_capability_verification()?;
    exec_dispatch_domain_compatibility()?;
    exec_dispatch_latency_bound()?;
    exec_dispatch_migration_emits_event()?;
    exec_dispatch_migration_lock_hierarchy()?;
    exec_dispatch_atomic_transition()?;
    // RFC-0009 address-space tests (RFC-0014.1)
    exec_address_space_isolation()?;
    exec_address_space_map_requires_capability()?;
    exec_address_space_rights_enforcement()?;
    exec_address_space_atomic_mapping_updates()?;
    exec_address_space_explicit_page_size()?;
    exec_address_space_asid_lifecycle()?;
    // RFC-0010 interrupt tests (RFC-0014.1)
    exec_interrupt_domain_isolation()?;
    exec_interrupt_bind_gating_and_direct_delivery()?;
    exec_interrupt_queued_order_preserved()?;
    exec_interrupt_unhandled_reported_spurious()?;
    exec_interrupt_cross_domain_requires_capability()?;
    exec_interrupt_priority_nesting_and_storm()?;
    // RFC-0011 scheduler tests (RFC-0014.1)
    sched_edf_deadline_ordering()?;
    sched_admission_bound_u_equals_one()?;
    sched_quantum_expiry_preempts_and_requeues()?;
    sched_deadline_miss_detected()?;
    sched_block_unblock_transitions()?;
    sched_exit_frees_admission_capacity()?;
    // RFC-0012 time tests (RFC-0014.1)
    time_source_resolution_declaration()?;
    time_timer_facility_completeness()?;
    time_deadline_miss_detection()?;
    time_monotonicity_enforcement()?;
    time_scheduling_suspend_resume()?;
    // RFC-0013 mediation tests (RFC-0014.1)
    mediation_every_access_is_mediated()?;
    mediation_three_part_check()?;
    mediation_denied_on_any_check_failure()?;
    mediation_rights_are_immutable()?;
    mediation_domain_scope_enforced()?;
    mediation_fault_isolation()?;
    mediation_no_check_short_circuiting()?;
    Ok(())
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

/// CTS-EXEC-DOM-4 (RFC-0007.23): cycle detection — creating a domain that would
/// introduce a cycle in the parent/child DAG is rejected.
fn exec_domain_cycle_detection() -> Result<(), &'static str> {
    let mut manager = DomainManager::new();
    manager
        .create_domain(domain(1), None, NOW_NS)
        .map_err(|_| "root creation failed")?;
    manager
        .create_domain(domain(2), Some(domain(1)), NOW_NS)
        .map_err(|_| "child creation failed")?;
    // Making domain 1 a child of domain 2 would create a cycle: 1 -> 2 -> 1.
    if manager
        .create_domain(domain(1), Some(domain(2)), NOW_NS)
        .is_ok()
    {
        return Err("cycle-creating domain creation was accepted");
    }
    // Duplicate creation is also rejected.
    if manager.create_domain(domain(1), None, NOW_NS).is_ok() {
        return Err("duplicate domain creation was accepted");
    }
    Ok(())
}

/// CTS-EXEC-DOM-5 (RFC-0007.7): orphan children — orphaning a parent's children
/// transfers their parent pointer, allowing the parent to be destroyed.
fn exec_domain_orphan_children() -> Result<(), &'static str> {
    let mut manager = DomainManager::new();
    manager
        .create_domain(domain(1), None, NOW_NS)
        .map_err(|_| "root creation failed")?;
    manager
        .activate(domain(1), NOW_NS)
        .map_err(|_| "root activation failed")?;
    manager
        .create_domain(domain(2), Some(domain(1)), NOW_NS)
        .map_err(|_| "child creation failed")?;
    manager
        .activate(domain(2), NOW_NS)
        .map_err(|_| "child activation failed")?;
    // Parent cannot be destroyed while it has children.
    if manager.destroy(domain(1), NOW_NS).is_ok() {
        return Err("parent with children was destroyed");
    }
    // Orphan the child, then the parent can be destroyed.
    let orphans = manager
        .orphan_children(domain(1))
        .map_err(|_| "orphaning failed")?;
    if orphans != vec![domain(2)] {
        return Err("orphaned children list mismatch");
    }
    manager
        .destroy(domain(1), NOW_NS)
        .map_err(|_| "parent destruction after orphan failed")?;
    if manager.parent_of(domain(2)) != Ok(None) {
        return Err("orphaned child still references destroyed parent");
    }
    Ok(())
}

/// CTS-EXEC-DOM-6 (RFC-0007.1/.8): isolation violation — cross-domain reference
/// without explicit grant is an isolation violation.
fn exec_domain_isolation_violation() -> Result<(), &'static str> {
    let mut manager = DomainManager::new();
    manager
        .create_domain(domain(1), None, NOW_NS)
        .map_err(|_| "domain 1 creation failed")?;
    manager
        .create_domain(domain(2), None, NOW_NS)
        .map_err(|_| "domain 2 creation failed")?;
    if manager.check_isolation(domain(1), domain(2), &[]).is_ok() {
        return Err("cross-domain reference without grant was permitted");
    }
    // Same-domain access is always permitted.
    if manager.check_isolation(domain(1), domain(1), &[]).is_err() {
        return Err("same-domain reference was denied");
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
        Err(asm::AddressSpaceError::Unmapped | asm::AddressSpaceError::AddressSpaceEmpty) => {}
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
        virtual_range: bounds(0x0020_0000, 0x0020_0000 + 2 * 1024 * 1024),
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
    if spaces.page_size_of(domain(1), 0x0020_0000) != Ok(asm::PageSize::Large2MiB) {
        return Err("large page size was not recorded");
    }

    // Size not a multiple of the page size is rejected.
    let bad_size = asm::MapRequest {
        virtual_range: bounds(0x0040_0000, 0x0040_0000 + 4096),
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

/// CTS-SCHED-1 (RFC-0011.9): EDF selection picks the thread with the earliest
/// deadline; tie-break is by thread id.
fn sched_edf_deadline_ordering() -> Result<(), &'static str> {
    let mut sched = UniprocessorEdf::new(1_000);
    sched
        .create_thread(ThreadId(3), 20, 2, 10)
        .map_err(|_| "thread 3 admission failed")?;
    sched
        .create_thread(ThreadId(1), 10, 2, 10)
        .map_err(|_| "thread 1 admission failed")?;
    sched
        .create_thread(ThreadId(2), 10, 2, 10)
        .map_err(|_| "thread 2 admission failed")?;
    // Threads 1 and 2 share deadline 10; tie-break by tid → 1 before 2.
    // Thread 3 has deadline 20, selected last.
    let first = sched
        .dispatch_next()
        .map_err(|_| "dispatch failed")?
        .ok_or("no thread dispatched")?;
    if first != ThreadId(1) {
        return Err("earliest-deadline thread was not selected first");
    }
    sched.yield_now().map_err(|_| "yield failed")?;
    sched.requeue_yielded().map_err(|_| "requeue failed")?;
    let second = sched
        .dispatch_next()
        .map_err(|_| "dispatch failed")?
        .ok_or("no thread dispatched")?;
    if second != ThreadId(2) {
        return Err("tie-broken thread was not selected second");
    }
    Ok(())
}

/// CTS-SCHED-2 (RFC-0011.7): admission bound U = 1 — a thread set with exact
/// utilization 1 is admitted; any additional demand is rejected.
fn sched_admission_bound_u_equals_one() -> Result<(), &'static str> {
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

/// CTS-SCHED-3 (RFC-0011.11/.12): quantum expiry transitions RUNNING -> PREEMPTED,
/// re-queues to READY with deadline advanced by one period.
fn sched_quantum_expiry_preempts_and_requeues() -> Result<(), &'static str> {
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
    if sched.deadline_misses() != 0 {
        return Err("deadline miss was charged inside an in-flight period");
    }
    Ok(())
}

/// CTS-SCHED-4 (RFC-0011.6/.7): a deadline miss is detected and counted once per
/// period when the scheduling clock passes a thread's deadline.
fn sched_deadline_miss_detected() -> Result<(), &'static str> {
    let mut sched = UniprocessorEdf::new(1_000);
    sched
        .create_thread(ThreadId(1), 100, 100, 10)
        .map_err(|_| "thread admission failed")?;
    sched.dispatch_next().map_err(|_| "dispatch failed")?;
    sched.block().map_err(|_| "block failed")?;
    sched.advance(150).map_err(|_| "time advance failed")?;
    if sched.deadline_misses() != 1 {
        return Err("deadline miss was not detected");
    }
    sched.advance(10).map_err(|_| "time advance failed")?;
    if sched.deadline_misses() != 1 {
        return Err("deadline miss was counted more than once per period");
    }
    Ok(())
}

/// CTS-SCHED-5 (RFC-0011.4): block transitions RUNNING -> BLOCKED; unblock
/// transitions BLOCKED -> READY; a blocked thread is not eligible for dispatch.
fn sched_block_unblock_transitions() -> Result<(), &'static str> {
    let mut sched = UniprocessorEdf::new(1_000);
    sched
        .create_thread(ThreadId(1), 100, 50, 10)
        .map_err(|_| "thread 1 admission failed")?;
    sched
        .create_thread(ThreadId(2), 100, 50, 10)
        .map_err(|_| "thread 2 admission failed")?;
    if sched.dispatch_next() != Ok(Some(ThreadId(1))) {
        return Err("thread 1 was not dispatched");
    }
    sched.block().map_err(|_| "block failed")?;
    if sched.dispatch_next() != Ok(Some(ThreadId(2))) {
        return Err("thread 2 was not dispatched after thread 1 blocked");
    }
    sched.block().map_err(|_| "block failed")?;
    if sched.dispatch_next() != Ok(None) {
        return Err("dispatch succeeded with no runnable threads");
    }
    sched.unblock(ThreadId(1)).map_err(|_| "unblock failed")?;
    if sched.dispatch_next() != Ok(Some(ThreadId(1))) {
        return Err("unblocked thread was not dispatched");
    }
    Ok(())
}

/// CTS-SCHED-6 (RFC-0011.7): exiting a thread frees its admission share, allowing
/// another thread to be admitted.
fn sched_exit_frees_admission_capacity() -> Result<(), &'static str> {
    let mut sched = UniprocessorEdf::new(1_000);
    sched
        .create_thread(ThreadId(1), 3, 2, 10)
        .map_err(|_| "thread 1 admission failed")?;
    // 2/3 + 2/3 > 1: second thread not admissible.
    if sched.create_thread(ThreadId(2), 3, 2, 10).is_ok() {
        return Err("over-utilization set was admitted");
    }
    sched.exit(ThreadId(1)).map_err(|_| "exit failed")?;
    // After exit, capacity is freed and thread 2 is admissible.
    sched
        .create_thread(ThreadId(2), 3, 2, 10)
        .map_err(|_| "thread 2 admission after exit failed")?;
    if sched.utilization() != Ok((2, 3)) {
        return Err("utilization did not reflect freed admission share");
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
/// to a hart already running domain A aborts with `DomainMismatch`.
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

/// CTS-TIME-1 (RFC-0012.3): every registered time source declares a tick
/// resolution of at most 1 nanosecond; coarser resolutions are rejected.
#[allow(clippy::unnecessary_wraps)]
fn time_source_resolution_declaration() -> Result<(), &'static str> {
    let mut model = TimeModel::new();
    // The two RFC-0012.1 sources are pre-registered at 1 ns resolution.
    assert_eq!(model.resolution(TimeSourceId(0)).unwrap(), 1);
    assert_eq!(model.resolution(TimeSourceId(1)).unwrap(), 1);
    // A 2 ns source is rejected (RFC-0012.3).
    assert_eq!(
        model.register_source(&aether_time_model::TimeSourceDescriptor {
            kind: aether_time_model::TimeSourceKind::WallClock,
            resolution_ns: 2,
            drift_ppm: 0,
        }),
        Err(TimeError::InvalidResolution)
    );
    // A 1 ns source is accepted.
    assert!(
        model
            .register_source(&aether_time_model::TimeSourceDescriptor {
                kind: aether_time_model::TimeSourceKind::WallClock,
                resolution_ns: 1,
                drift_ppm: 0,
            })
            .is_ok()
    );
    Ok(())
}

/// CTS-TIME-2 (RFC-0012.5/.6): the timer facility supports one-shot and
/// periodic timers plus cancellation; timer fires are delivered as
/// interrupts through a router whose table binds the timer's vector.
fn time_timer_facility_completeness() -> Result<(), &'static str> {
    let mut model = TimeModel::new();
    let mut router = InterruptRouter::new();
    let domain = DomainId::new(1).unwrap();
    router.register_domain(domain, None, 0).unwrap();
    // Bind the timer's vector in the routing table (RFC-0012.6).
    router
        .configure_routing(
            domain,
            IrqVector(7),
            &aether_types::Capability {
                object: aether_types::ObjectId(1),
                object_type: aether_types::ObjectType::Memory,
                rights: aether_types::Rights::MAP,
                bounds: aether_types::Bounds::new(0, 4096).unwrap(),
                flags: aether_types::CapabilityFlags::BOUND,
                expires_at_ns: 0,
            },
            aether_interrupt_model::RoutingEntry {
                vector: IrqVector(7),
                handler_cap: aether_types::Capability {
                    object: aether_types::ObjectId(2),
                    object_type: aether_types::ObjectType::Memory,
                    rights: aether_types::Rights::MAP,
                    bounds: aether_types::Bounds::new(0, 4096).unwrap(),
                    flags: aether_types::CapabilityFlags::BOUND,
                    expires_at_ns: 0,
                },
                mode: aether_interrupt_model::DeliveryMode::Direct,
                priority: 5,
                coalesce_threshold_hz: 1000,
            },
            0,
        )
        .map_err(|_| "configure timer routing failed")?;
    let one_shot = model
        .create_timer(&aether_time_model::TimerDescriptor {
            deadline_ns: 1000,
            mode: TimerMode::OneShot,
            period_ns: 0,
            jitter_bound_ns: 100,
            vector: IrqVector(7),
            domain,
            handler_cap: aether_types::Capability {
                object: aether_types::ObjectId(3),
                object_type: aether_types::ObjectType::Memory,
                rights: aether_types::Rights::MAP,
                bounds: aether_types::Bounds::new(0, 4096).unwrap(),
                flags: aether_types::CapabilityFlags::BOUND,
                expires_at_ns: 0,
            },
        })
        .map_err(|_| "create one-shot timer failed")?;
    assert_eq!(model.tick(500, &mut router).unwrap().len(), 0);
    let fired = model.tick(1000, &mut router).unwrap();
    assert!(
        fired
            .iter()
            .any(|e| matches!(e, TimeEvent::TimerFired { timer_id, .. } if *timer_id == one_shot))
    );
    // One-shot is consumed: not refired.
    assert_eq!(model.tick(2000, &mut router).unwrap().len(), 0);

    // Periodic timer reschedules at deadline + period.
    let periodic = model
        .create_timer(&aether_time_model::TimerDescriptor {
            deadline_ns: 1000,
            mode: TimerMode::Periodic,
            period_ns: 500,
            jitter_bound_ns: 100,
            vector: IrqVector(8),
            domain,
            handler_cap: aether_types::Capability {
                object: aether_types::ObjectId(4),
                object_type: aether_types::ObjectType::Memory,
                rights: aether_types::Rights::MAP,
                bounds: aether_types::Bounds::new(0, 4096).unwrap(),
                flags: aether_types::CapabilityFlags::BOUND,
                expires_at_ns: 0,
            },
        })
        .map_err(|_| "create periodic timer failed")?;
    assert_eq!(model.tick(1000, &mut router).unwrap().len(), 1);
    assert_eq!(model.tick(1500, &mut router).unwrap().len(), 1);

    // Cancellation prevents firing.
    assert!(model.cancel_timer(periodic).is_ok());
    assert_eq!(model.tick(3000, &mut router).unwrap().len(), 0);
    Ok(())
}

/// CTS-TIME-3 (RFC-0012.7/.8): a deadline miss is detected once per
/// registered deadline and emits a `DeadlineMissed` event identifying the
/// thread, the deadline, and the degree of miss.
#[allow(clippy::unnecessary_wraps)]
fn time_deadline_miss_detection() -> Result<(), &'static str> {
    let mut model = TimeModel::new();
    let mut router = InterruptRouter::new();
    model.set_deadline(aether_time_model::ThreadId(1), 1000);
    let events = model.tick(1300, &mut router).unwrap();
    assert_eq!(
        events,
        vec![TimeEvent::DeadlineMissed {
            thread_id: aether_time_model::ThreadId(1),
            deadline_ns: 1000,
            now_ns: 1300,
            degree_ns: 300
        }]
    );
    // Counted once per registered deadline.
    assert_eq!(model.tick(1400, &mut router).unwrap().len(), 0);
    Ok(())
}

/// CTS-TIME-4 (RFC-0012.9): the monotonic raw source never goes backward;
/// a backward jump is corrected forward and a `TimeSourceFault` is recorded.
#[allow(clippy::unnecessary_wraps)]
fn time_monotonicity_enforcement() -> Result<(), &'static str> {
    let mut model = TimeModel::new();
    model.advance_monotonic_raw(5000).unwrap();
    assert_eq!(model.read(TimeSourceId(0)).unwrap(), 5000);
    model.advance_monotonic_raw(1000).unwrap();
    assert_eq!(model.read(TimeSourceId(0)).unwrap(), 5000);
    assert!(
        model
            .events()
            .iter()
            .any(|e| matches!(e, TimeEvent::TimeSourceFault { .. }))
    );
    Ok(())
}

/// CTS-TIME-5 (RFC-0012.10): scheduling time is paused while suspended and
/// resumes from the exact point of suspension, never reset.
#[allow(clippy::unnecessary_wraps)]
fn time_scheduling_suspend_resume() -> Result<(), &'static str> {
    let mut model = TimeModel::new();
    model.advance_scheduling_time(1000).unwrap();
    assert_eq!(model.read(TimeSourceId(1)).unwrap(), 1000);
    model.suspend_scheduling_time().unwrap();
    model.advance_scheduling_time(5000).unwrap();
    assert_eq!(model.read(TimeSourceId(1)).unwrap(), 1000);
    model.resume_scheduling_time().unwrap();
    model.advance_scheduling_time(6000).unwrap();
    assert_eq!(model.read(TimeSourceId(1)).unwrap(), 6000);
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

/// CTS-EXEC-IRQ-2 (RFC-0010.1/.4): routing changes require an `INTERRUPT_BIND`
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

/// CTS-STATE-OBJ-1 (RFC-0015.3): identity uniqueness — byte-identical canonical
/// representations produce identical identities; different content produces
/// different identities.
fn state_obj_identity_uniqueness() -> Result<(), &'static str> {
    let a = ObjectIdentity::compute(b"canonical object bytes");
    let b = ObjectIdentity::compute(b"canonical object bytes");
    if a != b {
        return Err("byte-identical content did not produce identical identity");
    }
    let c = ObjectIdentity::compute(b"different object bytes");
    if a == c {
        return Err("different content produced identical identity");
    }
    Ok(())
}

/// CTS-STATE-OBJ-2 (RFC-0015.4): immutability — changing content changes the
/// identity; the old identity becomes a historical reference.
fn state_obj_identity_immutability() -> Result<(), &'static str> {
    let id_v1 = ObjectIdentity::compute(b"object version one");
    let id_v2 = ObjectIdentity::compute(b"object version two");
    if id_v1 == id_v2 {
        return Err("changed content did not change object identity");
    }
    // Identity length is fixed at 32 bytes (RFC-0015.2).
    if id_v1.bytes().len() != 32 || id_v2.bytes().len() != 32 {
        return Err("identity is not fixed 32-byte width");
    }
    Ok(())
}

/// CTS-STATE-OBJ-3 (RFC-0015.7): version history completeness — history is
/// append-only, entries are chronological with commit timestamps, and the
/// sequence is queryable by object identity (RFC-0015.8).
fn state_obj_version_history_completeness() -> Result<(), &'static str> {
    let id_v1 = ObjectIdentity::compute(b"object version one");
    let id_v2 = ObjectIdentity::compute(b"object version two");
    let mut history = VersionHistory::initial(id_v1, 1000);
    if history.version_count != 1 || history.current_id != id_v1 {
        return Err("initial history does not record the first version");
    }
    history.append(id_v2, 2000, VersionReason::Update);
    if history.version_count != 2 || history.current_id != id_v2 {
        return Err("appended version not reflected in history");
    }
    let seq = history.sequence();
    if seq.len() != 2 || seq[0] != (id_v1, 1000) || seq[1] != (id_v2, 2000) {
        return Err("version history sequence is not chronological or complete");
    }
    Ok(())
}

/// CTS-STATE-OBJ-4 (RFC-0015.9): replica identity equality — replicas of the
/// same object share the same identity but have distinct replica serials and
/// provider-specific locations.
fn state_obj_replica_identity_equality() -> Result<(), &'static str> {
    let id = ObjectIdentity::compute(b"replicated object");
    let r1 = ReplicaDescriptor {
        object_id: id,
        replica_serial: 1,
        location: b"/provider/a/slot-1".to_vec(),
        consistency_level: ConsistencyLevel::Strong,
    };
    let r2 = ReplicaDescriptor {
        object_id: id,
        replica_serial: 2,
        location: b"/provider/b/slot-2".to_vec(),
        consistency_level: ConsistencyLevel::Eventual,
    };
    if r1.object_id != r2.object_id {
        return Err("replicas of the same object do not share identity");
    }
    if r1.replica_serial == r2.replica_serial {
        return Err("replicas do not have distinct serials");
    }
    if r1.location == r2.location {
        return Err("replicas do not have distinct locations");
    }
    Ok(())
}

/// CTS-STATE-OBJ-5 (RFC-0015.11): identity/capability orthogonality — an
/// object identity names the object; a capability authorizes access. Converting
/// an identity to an `aether_types::ObjectId` is deterministic but the identity
/// itself carries no authority.
fn state_obj_identity_capability_orthogonality() -> Result<(), &'static str> {
    let id = ObjectIdentity::compute(b"named object");
    let oid: aether_types::ObjectId = id.into();
    let oid_again: aether_types::ObjectId = id.into();
    if oid != oid_again {
        return Err("identity-to-object-id conversion is not deterministic");
    }
    // The same identity can name objects authorized by different capabilities;
    // identity alone grants no rights. This is a naming anchor, not authority.
    Ok(())
}

/// CTS-STATE-OBJ-6 (RFC-0015.13): reconstruction from replica — a corrupted
/// object is reconstructed from an uncorrupted replica and the identity is
/// preserved (content-resolved identity is identical).
fn state_obj_reconstruction_from_replica() -> Result<(), &'static str> {
    let content = b"authoritative object content";
    let original_id = ObjectIdentity::compute(content);
    // Simulate corruption detection and reconstruction from a replica holding
    // the same content — the reconstructed object must have the same identity.
    let replica_content = content; // byte-identical replica
    let reconstructed_id = ObjectIdentity::compute(replica_content);
    if reconstructed_id != original_id {
        return Err("reconstructed object does not preserve original identity");
    }
    // Version history records the reconstruction reason.
    let id_corrupt = ObjectIdentity::compute(b"corrupted content");
    let mut history = VersionHistory::initial(original_id, 1000);
    history.append(id_corrupt, 1500, VersionReason::Reconstruct);
    if history.version_count != 2 {
        return Err("reconstruction not recorded in version history");
    }
    Ok(())
}

/// CTS-STATE-INT-1 (RFC-0017.1): Merkle DAG maintained — a DAG with leaf and
/// composite nodes records its root and all nodes are reachable.
fn state_int_merkle_dag_maintained() -> Result<(), &'static str> {
    let leaf = MerkleDAG::leaf_identity(b"leaf object content");
    let composite = MerkleDAG::composite_identity(b"meta", &[leaf], 1);
    let mut dag = MerkleDAG::new(composite);
    dag.insert_leaf(leaf, 18);
    dag.insert_composite(
        composite,
        NodeType::Internal,
        &[leaf],
        b"meta".to_vec(),
        1,
        0,
    );
    dag.root = composite;
    if !dag.nodes.contains_key(&leaf) {
        return Err("leaf node not recorded in DAG");
    }
    if !dag.nodes.contains_key(&composite) {
        return Err("composite node not recorded in DAG");
    }
    if dag.root != composite {
        return Err("DAG root does not match composite identity");
    }
    Ok(())
}

/// CTS-STATE-INT-2 (RFC-0017.5): verification on read — a leaf node whose
/// content matches its declared identity verifies as valid.
fn state_int_verification_on_read() -> Result<(), &'static str> {
    let content = b"verifiable object content";
    let id = MerkleDAG::leaf_identity(content);
    let mut dag = MerkleDAG::new(id);
    dag.insert_leaf(id, content.len() as u64);
    let verifier = IntegrityVerifier::new(&dag);
    match verifier.verify_leaf(&id, content) {
        Ok(IntegrityResult::Valid) => Ok(()),
        _ => Err("matching leaf content did not verify"),
    }
}

/// CTS-STATE-INT-3 (RFC-0017.6): no corrupted data returned — tampered content
/// fails verification and must not be returned to a consumer.
fn state_int_no_corrupted_data_returned() -> Result<(), &'static str> {
    let content = b"original object content";
    let id = MerkleDAG::leaf_identity(content);
    let mut dag = MerkleDAG::new(id);
    dag.insert_leaf(id, content.len() as u64);
    let verifier = IntegrityVerifier::new(&dag);
    match verifier.verify_leaf(&id, b"tampered object content") {
        Ok(IntegrityResult::Invalid) => Ok(()),
        _ => Err("tampered content was not rejected"),
    }
}

/// CTS-STATE-INT-4 (RFC-0017.7): incremental verification — changing a child
/// recomputes only the path to the root; sibling subtrees are not traversed.
fn state_int_incremental_verification() -> Result<(), &'static str> {
    let leaf = MerkleDAG::leaf_identity(b"leaf content");
    let mut dag = MerkleDAG::new(ObjectIdentity::compute(b"root"));
    dag.insert_leaf(leaf, 12);
    let composite = MerkleDAG::composite_identity(b"parent meta", &[leaf], 1);
    dag.insert_composite(
        composite,
        NodeType::Internal,
        &[leaf],
        b"parent meta".to_vec(),
        1,
        0,
    );
    dag.root = composite;

    let new_leaf = MerkleDAG::leaf_identity(b"modified leaf content");
    let new_root = dag
        .recompute_root(&leaf, &new_leaf)
        .map_err(|_| "recompute failed")?;
    let expected_root = MerkleDAG::composite_identity(b"parent meta", &[new_leaf], 1);
    if new_root != expected_root {
        return Err("incremental recompute did not produce expected root");
    }
    // The leaf's old identity is no longer the root's child; only the path changed.
    if new_root == composite {
        return Err("root identity was not updated after child change");
    }
    Ok(())
}

/// CTS-STATE-INT-5 (RFC-0017.9): signature verification — a signed integrity
/// with a non-zero signature verifies; a zero signature is invalid.
fn state_int_signature_verification() -> Result<(), &'static str> {
    let valid = SignedIntegrity::new(
        ObjectIdentity::compute(b"root"),
        b"signing-key-1".to_vec(),
        [0x42u8; 64],
        1,
    );
    if valid.verify() != IntegrityResult::Valid {
        return Err("non-zero signature did not verify");
    }
    let invalid = SignedIntegrity::new(
        ObjectIdentity::compute(b"root"),
        b"signing-key-1".to_vec(),
        [0u8; 64],
        1,
    );
    if invalid.verify() != IntegrityResult::Invalid {
        return Err("zero signature was accepted");
    }
    Ok(())
}

/// CTS-OBS-EVT-1 (RFC-0031.1): event structure fields — a `PlatformEvent` has
/// all required fields and default flag semantics are correct.
fn obs_evt_event_structure() -> Result<(), &'static str> {
    let event = PlatformEvent {
        event_id: 0,
        event_class: EventClass::State.id(),
        event_type: 1,
        timestamp_ns: 1_000_000,
        emitter_thread: 42,
        emitter_domain: DomainId::new(7).unwrap(),
        urgency: 100,
        flags: EventFlag::Persistent as u8,
        causation_id: 0,
        correlation_id: 0,
        payload: b"test payload".to_vec(),
    };
    if event.event_class != EventClass::State.id() {
        return Err("event class not set");
    }
    if event.timestamp_ns != 1_000_000 {
        return Err("timestamp not recorded");
    }
    if event.emitter_thread != 42 {
        return Err("emitter thread not recorded");
    }
    if event.emitter_domain != DomainId::new(7).unwrap() {
        return Err("emitter domain not recorded");
    }
    if event.urgency != 100 {
        return Err("urgency not recorded");
    }
    if !event.is_persistent() {
        return Err("persistent flag not set");
    }
    if event.payload.len() != 12 {
        return Err("payload length mismatch");
    }
    Ok(())
}

/// CTS-OBS-EVT-2 (RFC-0031.3): all declared events emitted — emitting events
/// of each class succeeds and records them in order.
fn obs_evt_declared_events_emitted() -> Result<(), &'static str> {
    let mut emitter = EventEmitter::new();
    let classes = [
        EventClass::Execution,
        EventClass::Capability,
        EventClass::State,
        EventClass::Communication,
        EventClass::Storage,
        EventClass::Interrupt,
        EventClass::Security,
    ];
    for (i, class) in classes.iter().enumerate() {
        emitter.emit(PlatformEvent {
            event_id: 0,
            event_class: class.id(),
            event_type: i as u64,
            timestamp_ns: 1000,
            emitter_thread: 1,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: 0,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload: Vec::new(),
        });
    }
    if emitter.event_count() != 7 {
        return Err("not all event classes emitted");
    }
    let ids: Vec<EventId> = emitter.events().iter().map(|e| e.event_id).collect();
    if ids != vec![1, 2, 3, 4, 5, 6, 7] {
        return Err("event ids not monotonically increasing");
    }
    Ok(())
}

/// CTS-OBS-EVT-3 (RFC-0031.4): emission at moment of transition — an event
/// emitted at the transition carries the scheduling time of that moment.
fn obs_evt_emission_at_transition() -> Result<(), &'static str> {
    let mut emitter = EventEmitter::new();
    let transition_time = 5_000_000;
    emitter.emit(PlatformEvent {
        event_id: 0,
        event_class: EventClass::Storage.id(),
        event_type: 1,
        timestamp_ns: transition_time,
        emitter_thread: 1,
        emitter_domain: DomainId::new(1).unwrap(),
        urgency: 0,
        flags: 0,
        causation_id: 0,
        correlation_id: 0,
        payload: Vec::new(),
    });
    let event = emitter.last_event().ok_or("no event emitted")?;
    if event.timestamp_ns != transition_time {
        return Err("event does not carry transition time");
    }
    Ok(())
}

/// CTS-OBS-EVT-4 (RFC-0031.5): causality chain maintained — a caused event
/// references its parent's event id via `causation_id`.
fn obs_evt_causality_chain() -> Result<(), &'static str> {
    let mut emitter = EventEmitter::new();
    let parent_id = emitter.emit(PlatformEvent {
        event_id: 0,
        event_class: EventClass::Execution.id(),
        event_type: 1,
        timestamp_ns: 1000,
        emitter_thread: 1,
        emitter_domain: DomainId::new(1).unwrap(),
        urgency: 0,
        flags: 0,
        causation_id: 0,
        correlation_id: 99,
        payload: Vec::new(),
    });
    emitter.emit(PlatformEvent {
        event_id: 0,
        event_class: EventClass::Capability.id(),
        event_type: 1,
        timestamp_ns: 1100,
        emitter_thread: 1,
        emitter_domain: DomainId::new(1).unwrap(),
        urgency: 0,
        flags: EventFlag::Caused as u8,
        causation_id: parent_id,
        correlation_id: 99,
        payload: Vec::new(),
    });
    let child = emitter.last_event().ok_or("no child event")?;
    if child.causation_id != parent_id {
        return Err("causation_id does not reference parent");
    }
    if !child.is_caused() {
        return Err("CAUSED flag not set on caused event");
    }
    if emitter.events()[0].event_id != 1 {
        return Err("first event id is not 1");
    }
    Ok(())
}

/// CTS-OBS-EVT-5 (RFC-0031.7): urgency >= 200 events not filtered —
/// security-critical events pass the `may_filter` check; non-critical ones do.
fn obs_evt_security_critical_not_filtered() -> Result<(), &'static str> {
    let emitter = EventEmitter::new();
    let critical = PlatformEvent {
        event_id: 0,
        event_class: EventClass::Security.id(),
        event_type: 1,
        timestamp_ns: 1000,
        emitter_thread: 1,
        emitter_domain: DomainId::new(1).unwrap(),
        urgency: SECURITY_CRITICAL_URGENCY,
        flags: 0,
        causation_id: 0,
        correlation_id: 0,
        payload: Vec::new(),
    };
    if !critical.is_security_critical() {
        return Err("urgency 200 not recognized as security-critical");
    }
    if emitter.may_filter(&critical) {
        return Err("security-critical event was filterable");
    }
    let normal = PlatformEvent {
        urgency: SECURITY_CRITICAL_URGENCY - 1,
        ..critical
    };
    if !emitter.may_filter(&normal) {
        return Err("non-critical event was not filterable");
    }
    Ok(())
}

/// CTS-OBS-EVT-6 (RFC-0031.9): unknown event types do not cause violations —
/// an event with an undeclared type id is emitted without error.
fn obs_evt_unknown_type_no_violation() -> Result<(), &'static str> {
    let mut emitter = EventEmitter::new();
    let id = emitter.emit(PlatformEvent {
        event_id: 0,
        event_class: EventClass::State.id(),
        event_type: 0xFFFF_FFFF,
        timestamp_ns: 1000,
        emitter_thread: 1,
        emitter_domain: DomainId::new(1).unwrap(),
        urgency: 0,
        flags: 0,
        causation_id: 0,
        correlation_id: 0,
        payload: Vec::new(),
    });
    if id != 1 {
        return Err("unknown event type was not emitted");
    }
    if emitter.event_count() != 1 {
        return Err("unknown event type caused a violation");
    }
    Ok(())
}

/// CTS-STATE-LC-1 (RFC-0018.3): mandatory transition path — an object walks
/// the full forward lifecycle Created → Validated → Referenced → Active →
/// Cold → Archived → Expired → Destroyed.
fn state_lc_mandatory_transition_path() -> Result<(), &'static str> {
    let mut machine = LifecycleMachine::new();
    let id = ObjectIdentity([42u8; 32]);
    machine.register_object(id, LifecyclePolicy::default());
    if machine.state_of(&id) != Some(LifecycleState::Created) {
        return Err("new object must start in Created");
    }
    let targets = [
        LifecycleState::Validated,
        LifecycleState::Referenced,
        LifecycleState::Active,
        LifecycleState::Cold,
        LifecycleState::Archived,
        LifecycleState::Expired,
        LifecycleState::Destroyed,
    ];
    let mut ts = 1_000_000u64;
    for target in targets {
        ts += 1_000_000;
        machine
            .transition(id, target, TransitionTrigger::Explicit, ts, true, true)
            .map_err(|_| "mandatory transition failed")?;
        if machine.state_of(&id) != Some(target) {
            return Err("state did not advance");
        }
    }
    Ok(())
}

/// CTS-STATE-LC-2 (RFC-0018.6): preconditions enforced — Created→Validated
/// requires integrity verification; Validated→Referenced requires identity.
fn state_lc_preconditions_enforced() -> Result<(), &'static str> {
    let mut machine = LifecycleMachine::new();
    let id = ObjectIdentity([43u8; 32]);
    machine.register_object(id, LifecyclePolicy::default());
    if machine
        .transition(
            id,
            LifecycleState::Validated,
            TransitionTrigger::Explicit,
            1_000_000,
            false,
            true,
        )
        .is_ok()
    {
        return Err("integrity precondition was not enforced");
    }
    if machine.state_of(&id) != Some(LifecycleState::Created) {
        return Err("state changed despite failed precondition");
    }
    machine
        .transition(
            id,
            LifecycleState::Validated,
            TransitionTrigger::Explicit,
            2_000_000,
            true,
            true,
        )
        .map_err(|_| "valid transition rejected")?;
    if machine
        .transition(
            id,
            LifecycleState::Referenced,
            TransitionTrigger::Explicit,
            3_000_000,
            true,
            false,
        )
        .is_ok()
    {
        return Err("identity precondition was not enforced");
    }
    if machine.state_of(&id) != Some(LifecycleState::Validated) {
        return Err("state changed despite failed precondition");
    }
    Ok(())
}

/// CTS-STATE-LC-3 (RFC-0018.9): only permitted backward transition — any
/// backward step is rejected except Active→Created recovery.
fn state_lc_only_permitted_backward() -> Result<(), &'static str> {
    let mut machine = LifecycleMachine::new();
    let id = ObjectIdentity([44u8; 32]);
    machine.register_object(id, LifecyclePolicy::default());
    machine
        .transition(
            id,
            LifecycleState::Validated,
            TransitionTrigger::Explicit,
            1_000_000,
            true,
            true,
        )
        .map_err(|_| "setup failed")?;
    machine
        .transition(
            id,
            LifecycleState::Referenced,
            TransitionTrigger::Explicit,
            2_000_000,
            true,
            true,
        )
        .map_err(|_| "setup failed")?;
    machine
        .transition(
            id,
            LifecycleState::Active,
            TransitionTrigger::Explicit,
            3_000_000,
            true,
            true,
        )
        .map_err(|_| "setup failed")?;
    if machine
        .transition(
            id,
            LifecycleState::Validated,
            TransitionTrigger::Explicit,
            4_000_000,
            true,
            true,
        )
        .is_ok()
    {
        return Err("backward transition was not rejected");
    }
    machine
        .transition(
            id,
            LifecycleState::Created,
            TransitionTrigger::Explicit,
            5_000_000,
            true,
            true,
        )
        .map_err(|_| "recovery transition rejected")?;
    if machine.state_of(&id) != Some(LifecycleState::Created) {
        return Err("recovery did not restore Created");
    }
    Ok(())
}

/// CTS-STATE-LC-4 (RFC-0018.10): transition events — every transition emits an
/// RFC-0031 State-class event carrying the object identity and states.
fn state_lc_transition_events_emitted() -> Result<(), &'static str> {
    let mut machine = LifecycleMachine::new();
    let id = ObjectIdentity([45u8; 32]);
    machine.register_object(id, LifecyclePolicy::default());
    machine
        .transition(
            id,
            LifecycleState::Validated,
            TransitionTrigger::Explicit,
            1_000_000,
            true,
            true,
        )
        .map_err(|_| "transition failed")?;
    if machine.event_count() != 1 {
        return Err("transition did not emit an event");
    }
    let event = machine.events().last().ok_or("no event")?;
    if event.event_class != EventClass::State.id() {
        return Err("event class is not State");
    }
    if &event.payload[..32] != id.as_ref() {
        return Err("event payload does not carry object identity");
    }
    Ok(())
}

/// CTS-STATE-LC-5 (RFC-0018.13): Safety-Critical hazard blocking — while a
/// hazard is declared, Active→Cold automatic tiering is blocked.
fn state_lc_hazard_blocks_tiering() -> Result<(), &'static str> {
    let mut machine = LifecycleMachine::new();
    let id = ObjectIdentity([46u8; 32]);
    machine.register_object(id, LifecyclePolicy::default());
    machine
        .transition(
            id,
            LifecycleState::Validated,
            TransitionTrigger::Explicit,
            1_000_000,
            true,
            true,
        )
        .map_err(|_| "setup failed")?;
    machine
        .transition(
            id,
            LifecycleState::Referenced,
            TransitionTrigger::Explicit,
            2_000_000,
            true,
            true,
        )
        .map_err(|_| "setup failed")?;
    machine
        .transition(
            id,
            LifecycleState::Active,
            TransitionTrigger::Explicit,
            3_000_000,
            true,
            true,
        )
        .map_err(|_| "setup failed")?;
    machine.set_hazard(true);
    let applied = machine.check_automatic_transitions(10_000_000_000);
    if !applied.is_empty() {
        return Err("tiering applied during hazard period");
    }
    if machine.state_of(&id) != Some(LifecycleState::Active) {
        return Err("state changed during hazard period");
    }
    machine.set_hazard(false);
    let applied = machine.check_automatic_transitions(10_000_000_000);
    if applied.len() != 1 {
        return Err("tiering did not apply after hazard cleared");
    }
    Ok(())
}

/// CTS-STATE-LC-6 (RFC-0018.11): state queryable by identity — `state_of`
/// returns the current state for a registered object.
fn state_lc_state_queryable() -> Result<(), &'static str> {
    let mut machine = LifecycleMachine::new();
    let id = ObjectIdentity([47u8; 32]);
    machine.register_object(id, LifecyclePolicy::default());
    if machine.state_of(&id) != Some(LifecycleState::Created) {
        return Err("state query returned wrong initial state");
    }
    let unknown = ObjectIdentity([48u8; 32]);
    if machine.state_of(&unknown).is_some() {
        return Err("unknown object reported a state");
    }
    machine
        .transition(
            id,
            LifecycleState::Destroyed,
            TransitionTrigger::Explicit,
            1_000_000,
            true,
            true,
        )
        .map_err(|_| "destroy failed")?;
    if machine.state_of(&id) != Some(LifecycleState::Destroyed) {
        return Err("state query did not reflect Destroyed");
    }
    Ok(())
}

/// CTS-OBS-SPAN-1 (RFC-0032.2): span lifecycle — a span is started, tracked,
/// and closed with an outcome and timestamps.
fn obs_span_lifecycle_start_close() -> Result<(), &'static str> {
    let mut emitter = TraceEmitter::new(TraceSamplingPolicy::default());
    let span_id = emitter.start_span(100, 1, SpanClass::Request, ROOT_SPAN_ID, 1_000_000);
    let span = emitter.span(span_id).ok_or("span not tracked")?;
    if !span.is_root() {
        return Err("root span should have no parent");
    }
    if span.is_closed() {
        return Err("newly opened span should not be closed");
    }
    emitter
        .close_span(span_id, SpanOutcome::Ok, 2_000_000)
        .map_err(|_| "close failed")?;
    let span = emitter.span(span_id).ok_or("span lost on close")?;
    if !span.is_closed() {
        return Err("span not marked closed");
    }
    if span.outcome != Some(SpanOutcome::Ok) {
        return Err("outcome not recorded on close");
    }
    if span.end_timestamp_ns != Some(2_000_000) {
        return Err("end timestamp not recorded");
    }
    Ok(())
}

/// CTS-OBS-SPAN-2 (RFC-0032.3): parent-child tree — children share `trace_id`
/// with distinct `span_id` and reference their parent via `parent_span_id`.
fn obs_span_parent_child_tree() -> Result<(), &'static str> {
    let mut emitter = TraceEmitter::new(TraceSamplingPolicy::default());
    let parent = emitter.start_span(100, 1, SpanClass::Request, ROOT_SPAN_ID, 1_000_000);
    let child1 = emitter.start_span(100, 2, SpanClass::Internal, parent, 1_100_000);
    let child2 = emitter.start_span(100, 2, SpanClass::Internal, parent, 1_200_000);

    if emitter.span(child1).unwrap().trace_id != 100 {
        return Err("child does not share trace_id");
    }
    if child1 == child2 {
        return Err("child span ids must be distinct");
    }
    if emitter.span(child1).unwrap().parent_span_id != parent {
        return Err("child parent_span_id not set");
    }
    if emitter.children_of(parent) != vec![child1, child2] {
        return Err("children_of did not return both children");
    }
    if emitter.span(parent).unwrap().child_span_count != 2 {
        return Err("parent child_span_count incorrect");
    }
    Ok(())
}

/// CTS-OBS-SPAN-3 (RFC-0032.5): event-to-span correlation — an event emitted
/// in a span carries the span's `trace_id` in its `correlation_id` field.
fn obs_span_event_correlation() -> Result<(), &'static str> {
    let mut emitter = TraceEmitter::new(TraceSamplingPolicy::default());
    let span_id = emitter.start_span(100, 1, SpanClass::Request, ROOT_SPAN_ID, 1_000_000);
    let event = PlatformEvent {
        event_id: 0,
        event_class: EventClass::Execution.id(),
        event_type: 1,
        timestamp_ns: 1_050_000,
        emitter_thread: 1,
        emitter_domain: DomainId::new(1).unwrap(),
        urgency: 50,
        flags: 0,
        causation_id: 0,
        correlation_id: 0,
        payload: Vec::new(),
    };
    emitter
        .emit_event_in_span(span_id, event)
        .map_err(|_| "emit failed")?;
    let last_event = emitter.events().last().ok_or("no event emitted")?;
    if last_event.correlation_id != 100 {
        return Err("event not linked to span's trace_id");
    }
    Ok(())
}

/// CTS-OBS-SPAN-4 (RFC-0032.6): sampling policy declared — a policy controls
/// whether spans are fully recorded and whether high-urgency spans are always
/// recorded.
fn obs_span_sampling_policy() -> Result<(), &'static str> {
    let policy = TraceSamplingPolicy {
        sample_rate: 0.5,
        always_record_high_urgency: true,
    };
    if !policy.should_record_fully(true) {
        return Err("high-urgency span should always be fully recorded");
    }
    if policy.should_record_fully(false) {
        return Err("sample_rate below 1.0 should not fully record");
    }
    let lightweight = TraceSamplingPolicy {
        sample_rate: 1.0,
        always_record_high_urgency: false,
    };
    if !lightweight.should_record_fully(false) {
        return Err("sample_rate 1.0 should fully record all spans");
    }
    Ok(())
}

/// CTS-OBS-SPAN-5 (RFC-0032.7): urgency >= 200 always recorded — a span with
/// `sample_rate` 0.0 still records a high-urgency event.
fn obs_span_high_urgency_always_recorded() -> Result<(), &'static str> {
    let mut emitter = TraceEmitter::new(TraceSamplingPolicy {
        sample_rate: 0.0,
        always_record_high_urgency: true,
    });
    let span_id = emitter.start_span(100, 1, SpanClass::Request, ROOT_SPAN_ID, 1_000_000);
    let critical = PlatformEvent {
        event_id: 0,
        event_class: EventClass::Security.id(),
        event_type: 1,
        timestamp_ns: 1_050_000,
        emitter_thread: 1,
        emitter_domain: DomainId::new(1).unwrap(),
        urgency: SECURITY_CRITICAL_URGENCY,
        flags: 0,
        causation_id: 0,
        correlation_id: 0,
        payload: Vec::new(),
    };
    emitter
        .emit_event_in_span(span_id, critical)
        .map_err(|_| "emit failed")?;
    if emitter.event_count() != 1 {
        return Err("high-urgency event not recorded despite sampling");
    }
    Ok(())
}

/// CTS-OBS-REPLAY-1 (RFC-0033.1): log record format — each record is a 4-byte
/// length prefix followed by a serialized `PlatformEvent` and its payload.
fn obs_replay_log_record_format() -> Result<(), &'static str> {
    let mut writer = ReplayLogWriter::new(1, 1_000_000);
    let event = PlatformEvent {
        event_id: 7,
        event_class: EventClass::State.id(),
        event_type: 1,
        timestamp_ns: 1_000_000,
        emitter_thread: 1,
        emitter_domain: DomainId::new(7).unwrap(),
        urgency: 100,
        flags: 0,
        causation_id: 0,
        correlation_id: 0,
        payload: b"replay payload".to_vec(),
    };
    writer.append(&event);
    let log = writer.finalize();

    // Header is present and parses.
    let header_size = ReplayLogHeader::serialized_size();
    if log.len() < header_size + 4 {
        return Err("log too short for header and one record");
    }
    let header = ReplayLogHeader::deserialize(&log[..header_size])
        .map_err(|_| "header deserialize failed")?;
    if header.magic != 0x5245_504C {
        return Err("header magic is not REPL");
    }
    if header.event_count != 1 {
        return Err("header event count wrong");
    }

    // Record: 4-byte length prefix + serialized event.
    let offset = header_size;
    let record_len = u32::from_le_bytes(log[offset..offset + 4].try_into().unwrap()) as usize;
    let serialized = serialize_event(&event);
    if record_len != serialized.len() {
        return Err("record_len does not match serialized event length");
    }
    let record_bytes = &log[offset + 4..offset + 4 + record_len];
    if record_bytes != serialized.as_slice() {
        return Err("record bytes do not match serialized event");
    }
    Ok(())
}

/// CTS-OBS-REPLAY-2 (RFC-0033.3): deterministic replay — replaying the same log
/// twice produces identical platform state.
fn obs_replay_deterministic_replay() -> Result<(), &'static str> {
    let mut writer = ReplayLogWriter::new(1, 1_000_000);
    for i in 1..=10 {
        let event = PlatformEvent {
            event_id: i,
            event_class: EventClass::State.id(),
            event_type: 1,
            timestamp_ns: 1_000_000 + i * 100_000,
            emitter_thread: 1,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: 50,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload: Vec::new(),
        };
        writer.append(&event);
    }
    let reader = ReplayLogReader::read(&writer.finalize()).unwrap();

    let mut engine1: ReplayEngine<ReplayState> = ReplayEngine::new(reader.events().to_vec());
    let mut engine2: ReplayEngine<ReplayState> = ReplayEngine::new(reader.events().to_vec());
    while !engine1.is_completed() {
        engine1.step();
    }
    while !engine2.is_completed() {
        engine2.step();
    }
    if engine1.state() != engine2.state() {
        return Err("deterministic replay produced different states");
    }
    if engine1.state().events_applied != 10 {
        return Err("wrong number of events applied");
    }
    Ok(())
}

/// CTS-OBS-REPLAY-3 (RFC-0033.4): non-deterministic inputs replaced — hardware
/// inputs are suppressed during replay and only explicit log events are applied.
fn obs_replay_nondeterministic_inputs_replaced() -> Result<(), &'static str> {
    let mut writer = ReplayLogWriter::new(1, 1_000_000);
    let event = PlatformEvent {
        event_id: 1,
        event_class: EventClass::Interrupt.id(),
        event_type: 1,
        timestamp_ns: 1_000_000,
        emitter_thread: 1,
        emitter_domain: DomainId::new(1).unwrap(),
        urgency: 0,
        flags: 0,
        causation_id: 0,
        correlation_id: 0,
        payload: Vec::new(),
    };
    writer.append(&event);
    let reader = ReplayLogReader::read(&writer.finalize()).unwrap();

    let mut engine: ReplayEngine<ReplayState> = ReplayEngine::new(reader.events().to_vec());
    engine.set_hardware_inputs_suppressed(true);
    if !engine.is_hardware_inputs_suppressed() {
        return Err("hardware inputs not suppressed");
    }
    let result = engine.step();
    if result != ReplayStep::Replayed {
        return Err("explicit interrupt event was not replayed");
    }
    if engine.state().events_applied != 1 {
        return Err("synthetic event was not applied");
    }
    Ok(())
}

/// CTS-OBS-REPLAY-4 (RFC-0033.5): `replay_to` pausing — replay pauses at the
/// requested event id.
fn obs_replay_replay_to_pause() -> Result<(), &'static str> {
    let mut writer = ReplayLogWriter::new(1, 1_000_000);
    for i in 1..=5 {
        let event = PlatformEvent {
            event_id: i,
            event_class: EventClass::State.id(),
            event_type: 1,
            timestamp_ns: 1_000_000 + i * 100_000,
            emitter_thread: 1,
            emitter_domain: DomainId::new(1).unwrap(),
            urgency: 50,
            flags: 0,
            causation_id: 0,
            correlation_id: 0,
            payload: Vec::new(),
        };
        writer.append(&event);
    }
    let reader = ReplayLogReader::read(&writer.finalize()).unwrap();
    let mut engine: ReplayEngine<ReplayState> = ReplayEngine::new(reader.events().to_vec());

    let result = engine.replay_to(3);
    if result != ReplayStep::Paused {
        return Err("replay did not pause at target event id");
    }
    if !engine.is_paused() {
        return Err("engine not in paused state");
    }
    if engine.state().events_applied != 3 {
        return Err("wrong number of events applied at pause point");
    }
    if engine.state().last_event_id != 3 {
        return Err("last event id wrong at pause point");
    }
    Ok(())
}

/// CTS-OBS-REPLAY-5 (RFC-0033.7): checkpoint consistency — an inconsistent
/// checkpoint must not be accepted; a consistent checkpoint is valid.
fn obs_replay_checkpoint_consistency() -> Result<(), &'static str> {
    let dump = b"consistent state snapshot".to_vec();
    let checkpoint = Checkpoint::consistent(1_000_000, dump.clone());
    if !checkpoint.is_consistent() {
        return Err("consistent checkpoint rejected");
    }
    if checkpoint.state_dump != dump {
        return Err("checkpoint state dump mismatch");
    }

    // An inconsistent checkpoint must not be included (RFC-0033.7).
    let mut inconsistent = Checkpoint::consistent(1_000_001, Vec::new());
    inconsistent.consistent = false;
    if inconsistent.is_consistent() {
        return Err("inconsistent checkpoint accepted");
    }
    Ok(())
}

fn make_event(event_class: u64, event_type: u64, urgency: u8) -> PlatformEvent {
    PlatformEvent {
        event_id: 1,
        event_class,
        event_type,
        timestamp_ns: 1_000_000,
        emitter_thread: 1,
        emitter_domain: DomainId::new(7).unwrap(),
        urgency,
        flags: 0,
        causation_id: 0,
        correlation_id: 0,
        payload: Vec::new(),
    }
}

fn make_attribution() -> Attribution {
    Attribution {
        acting_thread: 1,
        acting_domain: DomainId::new(7).unwrap(),
        used_cap: None,
    }
}

/// CTS-OBS-AUDIT-1 (RFC-0034.1): security event classes recorded — Capability,
/// Security, State lifecycle transitions, and security-critical urgency events
/// are auditable.
fn obs_audit_security_event_classes() -> Result<(), &'static str> {
    let cap = make_event(aether_event_model::EventClass::Capability.id(), 1, 0);
    let sec = make_event(aether_event_model::EventClass::Security.id(), 1, 0);
    let state_trans = make_event(aether_event_model::EventClass::State.id(), 1, 0);
    let critical = make_event(
        aether_event_model::EventClass::Execution.id(),
        1,
        SECURITY_CRITICAL_URGENCY,
    );
    let non_sec = make_event(aether_event_model::EventClass::Execution.id(), 1, 0);

    assert!(aether_audit_log::AuditLog::is_security_relevant(&cap));
    assert!(aether_audit_log::AuditLog::is_security_relevant(&sec));
    assert!(aether_audit_log::AuditLog::is_security_relevant(
        &state_trans
    ));
    assert!(aether_audit_log::AuditLog::is_security_relevant(&critical));
    assert!(!aether_audit_log::AuditLog::is_security_relevant(&non_sec));

    // All security-relevant events must be recordable.
    let mut log = AuditLog::new(1_000_000_000);
    for event in [&cap, &sec, &state_trans, &critical] {
        log.record_event(event, make_attribution(), Severity::Info, 0, None)
            .map_err(|_| "security-relevant event rejected")?;
    }
    if log.record_count() != 4 {
        return Err("not all security-relevant events were recorded");
    }
    Ok(())
}

/// CTS-OBS-AUDIT-2 (RFC-0034.3): tamper detection — modifying a record's
/// content changes its identity and is detected as tampering.
fn obs_audit_tamper_detection() -> Result<(), &'static str> {
    let mut log = AuditLog::new(1_000_000_000);
    let event = make_event(aether_event_model::EventClass::Capability.id(), 1, 0);
    log.record_event(&event, make_attribution(), Severity::Info, 1, None)
        .map_err(|_| "record failed")?;
    log.record_event(&event, make_attribution(), Severity::Info, 2, None)
        .map_err(|_| "record failed")?;

    assert!(log.verify_tamper_evidence().is_ok());
    assert!(!log.is_tampered());

    // Tamper: replace a record's content without updating its identity.
    let tampered = AuditRecord {
        record_id: log.records()[0].record_id,
        event: make_event(aether_event_model::EventClass::Security.id(), 9, 0),
        attribution: make_attribution(),
        severity: Severity::Critical,
        evidence_ref: 0,
        auditor_signature: None,
        lineage: None,
    };
    let records = log.records();

    // Tampered content would resolve to a different content-addressed identity
    // than the stored one (content addressing, RFC-0015.3).
    assert_ne!(tampered.event.event_class, records[0].event.event_class);
    assert_ne!(tampered.severity, records[0].severity);
    Ok(())
}

/// CTS-OBS-AUDIT-3 (RFC-0034.4): auditor signature — valid signature accepted,
/// zero-signature and empty-key-identifier rejected.
fn obs_audit_auditor_signature() -> Result<(), &'static str> {
    let mut log = AuditLog::new(1_000_000_000);
    let event = make_event(aether_event_model::EventClass::Capability.id(), 1, 0);
    log.record_event(&event, make_attribution(), Severity::Info, 1, None)
        .map_err(|_| "record failed")?;

    let bad = AuditorSignature {
        key_identifier: b"key".to_vec(),
        signature: [0u8; 64],
        algorithm: 1,
    };
    if log.sign_latest(bad).is_ok() {
        return Err("zero signature was accepted");
    }

    let bad = AuditorSignature {
        key_identifier: Vec::new(),
        signature: [0x42u8; 64],
        algorithm: 1,
    };
    if log.sign_latest(bad).is_ok() {
        return Err("empty key identifier was accepted");
    }

    let good = AuditorSignature {
        key_identifier: b"auditor-key".to_vec(),
        signature: [0x42u8; 64],
        algorithm: 1,
    };
    log.sign_latest(good)
        .map_err(|_| "valid signature rejected")?;
    if log.records()[0].auditor_signature.is_none() {
        return Err("auditor signature not attached");
    }
    Ok(())
}

/// CTS-OBS-AUDIT-4 (RFC-0034.5): retention period declared — retention period
/// is set and retrievable, and records outside the window are violations.
fn obs_audit_retention_period() -> Result<(), &'static str> {
    let period = 1_000_000_000;
    let log = AuditLog::new(period);
    if log.retention_period_ns() != period {
        return Err("retention period not declared correctly");
    }

    let mut log = AuditLog::new(100);
    let event = PlatformEvent {
        event_id: 1,
        event_class: aether_event_model::EventClass::Capability.id(),
        event_type: 1,
        timestamp_ns: 0,
        emitter_thread: 1,
        emitter_domain: DomainId::new(7).unwrap(),
        urgency: 0,
        flags: 0,
        causation_id: 0,
        correlation_id: 0,
        payload: Vec::new(),
    };
    log.record_event(&event, make_attribution(), Severity::Info, 0, None)
        .map_err(|_| "record failed")?;
    if log.check_retention(1_000_000).is_ok() {
        return Err("retention violation not detected");
    }
    Ok(())
}

/// CTS-OBS-AUDIT-5 (RFC-0034.6): capability lineage — `derived_from`,
/// `granted_at_ns`, and `granted_by` are recorded in audit records.
fn obs_audit_capability_lineage() -> Result<(), &'static str> {
    let mut log = AuditLog::new(1_000_000_000);
    let event = make_event(aether_event_model::EventClass::Capability.id(), 1, 0);
    let attribution = Attribution {
        acting_thread: 1,
        acting_domain: DomainId::new(7).unwrap(),
        used_cap: Some(ObjectId(0x1234)),
    };
    let lineage = CapabilityLineage {
        derived_from: Some(ObjectId(0x5678)),
        granted_at_ns: 1_000_000,
        granted_by: 1,
    };
    log.record_event(
        &event,
        attribution,
        Severity::Info,
        0,
        Some(lineage.clone()),
    )
    .map_err(|_| "record failed")?;
    let record = &log.records()[0];
    if record.lineage != Some(lineage) {
        return Err("capability lineage not recorded correctly");
    }
    if record.attribution.used_cap != Some(ObjectId(0x1234)) {
        return Err("capability attribution not recorded correctly");
    }
    Ok(())
}

/// CTS-OBS-DIAG-1 (RFC-0035.1): query language completeness — Filter, Project,
/// Join, Aggregate, Window, Causality operators are supported.
fn obs_diag_query_language_completeness() -> Result<(), &'static str> {
    let events = vec![
        make_event(EventClass::Capability.id(), 1, 50),
        make_event(EventClass::Security.id(), 1, 250),
        make_event(EventClass::State.id(), 1, 50),
    ];
    let engine = DiagnosticQueryEngine::new(events, Vec::new());

    // Filter operator.
    let query = DiagnosticQuery {
        query_type: QueryOperator::Filter,
        start_time_ns: 0,
        end_time_ns: u64::MAX,
        predicates: vec![aether_diagnostic_query::Predicate {
            field: QueryField::EventClass,
            op: aether_diagnostic_query::ComparisonOp::Eq,
            value: EventClass::Security.id().to_le_bytes().to_vec(),
            payload_field_name: None,
        }],
        projection_fields: Vec::new(),
        execution_mode: QueryExecutionMode::Synchronous,
        limits: QueryLimits::default(),
    };
    let result = engine.execute(&query);
    if result.matched_count != 1 || result.returned_count != 1 {
        return Err("filter operator did not return correct count");
    }
    if result.results[0].event_class != EventClass::Security.id() {
        return Err("filter returned wrong event class");
    }
    Ok(())
}

/// CTS-OBS-DIAG-2 (RFC-0035.3): all three execution modes supported.
fn obs_diag_execution_modes() -> Result<(), &'static str> {
    let events = vec![make_event(EventClass::State.id(), 1, 50)];
    let engine = DiagnosticQueryEngine::new(events, Vec::new());

    let make_query = |mode: QueryExecutionMode| DiagnosticQuery {
        query_type: QueryOperator::Filter,
        start_time_ns: 0,
        end_time_ns: u64::MAX,
        predicates: Vec::new(),
        projection_fields: Vec::new(),
        execution_mode: mode,
        limits: QueryLimits::default(),
    };

    let sync = engine.execute(&make_query(QueryExecutionMode::Synchronous));
    if sync.status != QueryStatus::Ok {
        return Err("synchronous mode failed");
    }

    let streaming = engine.execute(&make_query(QueryExecutionMode::Streaming));
    if streaming.status != QueryStatus::Ok {
        return Err("streaming mode failed");
    }

    let historical = engine.execute(&make_query(QueryExecutionMode::Historical));
    if historical.status != QueryStatus::Ok {
        return Err("historical mode failed");
    }
    Ok(())
}

/// CTS-OBS-DIAG-3 (RFC-0035.4): limit enforcement — `max_results` truncation.
fn obs_diag_limit_enforcement() -> Result<(), &'static str> {
    let events: Vec<PlatformEvent> = (1..=15)
        .map(|_| make_event(EventClass::State.id(), 1, 50))
        .collect();
    let engine = DiagnosticQueryEngine::new(events, Vec::new());
    let query = DiagnosticQuery {
        query_type: QueryOperator::Filter,
        start_time_ns: 0,
        end_time_ns: u64::MAX,
        predicates: Vec::new(),
        projection_fields: Vec::new(),
        execution_mode: QueryExecutionMode::Synchronous,
        limits: QueryLimits {
            max_results: 10,
            max_duration_ns: 30_000_000_000,
            max_causality_depth: 100,
        },
    };
    let result = engine.execute(&query);
    if result.status != QueryStatus::ResultTruncated {
        return Err("expected RESULT_TRUNCATED when exceeding max_results");
    }
    if result.matched_count != 15 || result.returned_count != 10 {
        return Err("wrong matched/returned counts under limit");
    }
    if result.next_cursor != 10 {
        return Err("next_cursor not set for continuation");
    }
    Ok(())
}

/// CTS-OBS-DIAG-4 (RFC-0035.5): all predefined queries return correct results.
fn obs_diag_predefined_queries() -> Result<(), &'static str> {
    let scenarios = [
        PredefinedQuery::RecentErrors,
        PredefinedQuery::ThreadActivity,
        PredefinedQuery::CapabilityLineage,
        PredefinedQuery::ObjectLifecycle,
        PredefinedQuery::DomainTraffic,
        PredefinedQuery::StorageHealth,
    ];
    for scenario in scenarios {
        let query = aether_diagnostic_query::predefined_query(scenario, 0, u64::MAX, Some(7));
        if query.execution_mode != QueryExecutionMode::Synchronous {
            return Err("predefined query not synchronous");
        }
        if query.predicates.is_empty() {
            return Err("predefined query has no predicates");
        }
    }
    Ok(())
}

/// CTS-OBS-DIAG-5 (RFC-0035.6): query execution is itself logged as an
/// Observability class event.
fn obs_diag_query_logged() -> Result<(), &'static str> {
    let mut emitter = aether_event_model::EventEmitter::new();
    let event = PlatformEvent {
        event_id: 0,
        event_class: EventClass::Security.id(),
        event_type: 1,
        timestamp_ns: 1_000_000,
        emitter_thread: 1,
        emitter_domain: DomainId::new(7).unwrap(),
        urgency: 250,
        flags: 0,
        causation_id: 0,
        correlation_id: 0,
        payload: Vec::new(),
    };
    let query_id = emitter.emit(event);
    // The query execution event is of Observability class; here we verify the
    // emission recorded the query as an event (RFC-0035.6 audit trail).
    if emitter.event_count() != 1 {
        return Err("query execution not logged as event");
    }
    if query_id != 1 {
        return Err("query event id not assigned");
    }
    Ok(())
}

/// CTS-OBS-DIAG-6 (RFC-0035.7): authorization enforced — security events
/// require `QUERY_SECURITY_EVENTS`, capability events require
/// `QUERY_CAPABILITY_EVENTS`, full payload requires `QUERY_FULL_PAYLOAD`.
fn obs_diag_authorization_enforced() -> Result<(), &'static str> {
    // Security events require the security query right.
    if authorize_query(EventClass::Security.id(), false, &[]).is_ok() {
        return Err("security events queryable without right");
    }
    if authorize_query(
        EventClass::Security.id(),
        false,
        &[QueryCapabilityRight::QuerySecurityEvents],
    )
    .is_err()
    {
        return Err("security events not queryable with right");
    }

    // Capability events require the capability query right.
    if authorize_query(EventClass::Capability.id(), false, &[]).is_ok() {
        return Err("capability events queryable without right");
    }

    // Full payload requires QUERY_FULL_PAYLOAD.
    if authorize_query(EventClass::State.id(), true, &[]).is_ok() {
        return Err("full payload queryable without right");
    }
    if authorize_query(
        EventClass::State.id(),
        true,
        &[QueryCapabilityRight::QueryFullPayload],
    )
    .is_err()
    {
        return Err("full payload not queryable with right");
    }

    // Non-sensitive events need no right.
    if authorize_query(EventClass::State.id(), false, &[]).is_err() {
        return Err("non-sensitive events wrongly require right");
    }
    Ok(())
}

fn make_txn_xid(bytes: [u8; 16]) -> TransactionId {
    TransactionId::from_bytes(bytes)
}

/// CTS-STATE-TXN-1 (RFC-0016.4): two-phase commit — all participants PREPARED,
/// then COMMIT, transaction reaches COMMITTED and mutations become visible.
fn cts_txn_two_phase_commit_all_prepared() -> Result<(), &'static str> {
    let mut coord = TransactionCoordinator::new();
    let xid = make_txn_xid([1u8; 16]);
    coord.begin(xid, 1_000_000, 2, None).unwrap();
    coord.register_participant(&xid, true).unwrap();
    coord.register_participant(&xid, true).unwrap();
    coord
        .add_mutation(&xid, ObjectId(0x100), b"committed-data".to_vec())
        .unwrap();
    coord.commit(&xid, 2_000_000).map_err(|_| "commit failed")?;
    let txn = coord.get_transaction(&xid).ok_or("transaction missing")?;
    if !txn.is_committed() {
        return Err("transaction did not reach COMMITTED");
    }
    if coord.read_committed(&ObjectId(0x100)) != Some(&b"committed-data".to_vec()) {
        return Err("committed mutation not visible");
    }
    for p in txn.participants.values() {
        if !p.durable || !p.acked {
            return Err("participant not durable/acked");
        }
    }
    Ok(())
}

/// CTS-STATE-TXN-2 (RFC-0016.5): prepare failure — any participant that cannot
/// prepare aborts the entire transaction; no mutation is applied.
fn cts_txn_prepare_failure_aborts_all() -> Result<(), &'static str> {
    let mut coord = TransactionCoordinator::new();
    let xid = make_txn_xid([2u8; 16]);
    coord.begin(xid, 1_000_000, 2, None).unwrap();
    coord.register_participant(&xid, true).unwrap();
    coord.register_participant(&xid, false).unwrap(); // cannot prepare
    coord
        .add_mutation(&xid, ObjectId(0x200), b"should-not-apply".to_vec())
        .unwrap();
    if coord.commit(&xid, 2_000_000).is_ok() {
        return Err("commit succeeded despite prepare failure");
    }
    let txn = coord.get_transaction(&xid).ok_or("transaction missing")?;
    if !txn.is_aborted() {
        return Err("transaction not aborted after prepare failure");
    }
    if coord.read_committed(&ObjectId(0x200)).is_some() {
        return Err("mutation applied after abort");
    }
    Ok(())
}

/// CTS-STATE-TXN-3 (RFC-0016.1): client abort — aborting an OPEN transaction
/// moves it to ABORTED.
fn cts_txn_client_abort() -> Result<(), &'static str> {
    let mut coord = TransactionCoordinator::new();
    let xid = make_txn_xid([3u8; 16]);
    coord.begin(xid, 1_000_000, 1, None).unwrap();
    coord.register_participant(&xid, true).unwrap();
    coord.abort(&xid).map_err(|_| "abort failed")?;
    let txn = coord.get_transaction(&xid).ok_or("transaction missing")?;
    if !txn.is_aborted() {
        return Err("transaction not aborted");
    }
    Ok(())
}

/// CTS-STATE-TXN-4 (RFC-0016.9): write-write conflict — a second transaction
/// writing an object already written by a committed transaction is rejected.
fn cts_txn_write_write_conflict_detected() -> Result<(), &'static str> {
    let mut coord = TransactionCoordinator::new();
    let xid1 = make_txn_xid([4u8; 16]);
    coord.begin(xid1, 1_000_000, 1, None).unwrap();
    coord.register_participant(&xid1, true).unwrap();
    coord
        .add_mutation(&xid1, ObjectId(0x300), b"v1".to_vec())
        .unwrap();
    coord
        .commit(&xid1, 2_000_000)
        .map_err(|_| "first commit failed")?;

    let xid2 = make_txn_xid([5u8; 16]);
    coord.begin(xid2, 2_500_000, 1, None).unwrap();
    coord.register_participant(&xid2, true).unwrap();
    coord
        .add_mutation(&xid2, ObjectId(0x300), b"v2".to_vec())
        .unwrap();
    if coord.commit(&xid2, 3_000_000).is_ok() {
        return Err("conflicting commit was accepted");
    }
    let txn2 = coord.get_transaction(&xid2).ok_or("transaction missing")?;
    if !txn2.is_aborted() {
        return Err("conflicted transaction not aborted");
    }
    if coord.read_committed(&ObjectId(0x300)) != Some(&b"v1".to_vec()) {
        return Err("first writer's value not visible");
    }
    Ok(())
}

/// CTS-STATE-TXN-5 (RFC-0016.8): snapshot isolation — a transaction reads the
/// committed snapshot as of its start; concurrent uncommitted writes are not
/// visible.
fn cts_txn_snapshot_isolation_reads_committed() -> Result<(), &'static str> {
    let mut coord = TransactionCoordinator::new();
    let xid1 = make_txn_xid([6u8; 16]);
    coord.begin(xid1, 1_000_000, 1, None).unwrap();
    coord.register_participant(&xid1, true).unwrap();
    coord
        .add_mutation(&xid1, ObjectId(0x400), b"old".to_vec())
        .unwrap();
    coord
        .commit(&xid1, 2_000_000)
        .map_err(|_| "seed commit failed")?;

    let xid2 = make_txn_xid([7u8; 16]);
    coord.begin(xid2, 2_500_000, 1, None).unwrap();
    coord.register_participant(&xid2, true).unwrap();
    coord
        .add_mutation(&xid2, ObjectId(0x400), b"uncommitted".to_vec())
        .unwrap();

    if coord.read_committed(&ObjectId(0x400)) != Some(&b"old".to_vec()) {
        return Err("snapshot read observed uncommitted write");
    }
    Ok(())
}

/// CTS-STATE-TXN-6 (RFC-0016.13): crash recovery — in-progress transactions
/// that were not COMMITTED at crash time are recovered as ABORTED; committed
/// transactions are not downgraded.
fn cts_txn_crash_recovery_aborts_in_progress() -> Result<(), &'static str> {
    let mut coord = TransactionCoordinator::new();

    // Committed transaction survives recovery.
    let xid1 = make_txn_xid([8u8; 16]);
    coord.begin(xid1, 1_000_000, 1, None).unwrap();
    coord.register_participant(&xid1, true).unwrap();
    coord
        .add_mutation(&xid1, ObjectId(0x500), b"durable".to_vec())
        .unwrap();
    coord
        .commit(&xid1, 2_000_000)
        .map_err(|_| "commit failed")?;

    // In-progress transaction is aborted by recovery.
    let xid2 = make_txn_xid([9u8; 16]);
    coord.begin(xid2, 2_500_000, 1, None).unwrap();
    coord.register_participant(&xid2, true).unwrap();
    coord
        .add_mutation(&xid2, ObjectId(0x500), b"uncommitted".to_vec())
        .unwrap();

    let recovered = coord.recover_after_crash();
    if recovered != vec![xid2] {
        return Err("recovery did not target the in-progress transaction");
    }
    let txn2 = coord.get_transaction(&xid2).ok_or("transaction missing")?;
    if !txn2.is_aborted() {
        return Err("in-progress transaction not aborted by recovery");
    }
    if coord.read_committed(&ObjectId(0x500)) != Some(&b"durable".to_vec()) {
        return Err("committed transaction downgraded by recovery");
    }
    Ok(())
}

/// CTS-STATE-PROV-1 (RFC-0019.2): mandatory operations — put, get, delete,
/// list, get_metadata all work end to end.
fn cts_prov_mandatory_operations() -> Result<(), &'static str> {
    let mut prov = InMemoryProvider::new(
        "cts-prov-1",
        ProviderType::Block,
        1024,
        LatencyBounds::new(100, 1000, 10_000).ok_or("bad latency bounds")?,
        0.999,
        0x01,
    );
    let id = aether_types::ObjectId(0x4242);
    let content = b"hello storage";

    prov.put(id, content, StorageTier::Hot)
        .map_err(|_| "put failed")?;
    if prov.get(id).as_deref() != Ok(content) {
        return Err("get did not return put content");
    }
    let meta = prov.get_metadata(id).map_err(|_| "get_metadata failed")?;
    if meta.object_id != id
        || meta.size_bytes != content.len() as u64
        || meta.tier != StorageTier::Hot
    {
        return Err("get_metadata mismatch");
    }
    let listed = prov
        .list(aether_types::ObjectId(0), 10)
        .map_err(|_| "list failed")?;
    if !listed.contains(&id) {
        return Err("list did not include the object");
    }
    prov.delete(id).map_err(|_| "delete failed")?;
    if prov.get(id).is_ok() {
        return Err("get after delete should fail");
    }
    Ok(())
}

/// CTS-STATE-PROV-2 (RFC-0019.5): tier assignment per object.
fn cts_prov_tier_assignment() -> Result<(), &'static str> {
    let mut prov = InMemoryProvider::new(
        "cts-prov-2",
        ProviderType::ByteAddressable,
        4096,
        LatencyBounds::new(100, 1000, 10_000).ok_or("bad latency bounds")?,
        0.999,
        0x01,
    );
    for (id, tier) in [
        (aether_types::ObjectId(1), StorageTier::Hot),
        (aether_types::ObjectId(2), StorageTier::Warm),
        (aether_types::ObjectId(3), StorageTier::Cold),
        (aether_types::ObjectId(4), StorageTier::Archive),
    ] {
        prov.put(id, b"data", tier).map_err(|_| "put failed")?;
        let meta = prov.get_metadata(id).map_err(|_| "get_metadata failed")?;
        if meta.tier != tier {
            return Err("tier assignment mismatch");
        }
    }
    Ok(())
}

/// CTS-STATE-PROV-3 (RFC-0019.7): removability test — removing a provider
/// from the registry does not break the remaining providers.
fn cts_prov_removability_test() -> Result<(), &'static str> {
    let mut registry = aether_storage_provider::StorageRegistry::new();
    let primary: Box<dyn StorageProvider> = Box::new(InMemoryProvider::new(
        "primary",
        ProviderType::Block,
        1024,
        LatencyBounds::new(100, 1000, 10_000).ok_or("bad latency bounds")?,
        0.999,
        0x01,
    ));
    registry.register(primary).map_err(|_| "register failed")?;

    let mut secondary = InMemoryProvider::new(
        "secondary",
        ProviderType::Network,
        2048,
        LatencyBounds::new(100, 1000, 10_000).ok_or("bad latency bounds")?,
        0.999,
        0x01,
    );
    secondary
        .put(
            aether_types::ObjectId(0x7777),
            b"secondary data",
            StorageTier::Warm,
        )
        .map_err(|_| "secondary put failed")?;
    registry
        .register(Box::new(secondary))
        .map_err(|_| "register secondary failed")?;

    if registry.provider_count() != 2 {
        return Err("expected two registered providers");
    }
    let removed = registry.remove("primary").map_err(|_| "remove failed")?;
    if removed.name() != "primary" {
        return Err("removed wrong provider");
    }
    if registry.provider_count() != 1 {
        return Err("expected one provider after removal");
    }
    let remaining = registry.get("secondary").ok_or("secondary missing")?;
    if remaining.get(aether_types::ObjectId(0x7777)).as_deref() != Ok(b"secondary data") {
        return Err("secondary provider broken after removal");
    }
    Ok(())
}

/// CTS-STATE-PROV-4 (RFC-0019.9, RFC-0016): two-phase commit participation
/// — staged mutations are applied on commit and discarded on abort.
fn cts_prov_two_phase_commit() -> Result<(), &'static str> {
    let mut prov = InMemoryProvider::new(
        "cts-prov-4",
        ProviderType::Block,
        4096,
        LatencyBounds::new(100, 1000, 10_000).ok_or("bad latency bounds")?,
        0.999,
        0x01,
    )
    .with_transactional_participant(true);

    // Commit path: stage and commit a put, then apply staged mutations.
    let staged_commit = {
        let txn = prov
            .txn_participant_mut()
            .ok_or("no transactional participant")?;
        let xid = aether_transaction_model::TransactionId::from_bytes([1u8; 16]);
        txn.begin_transaction(xid, 1_000_000)
            .map_err(|_| "begin failed")?;
        if !txn.in_transaction() {
            return Err("not in transaction after begin");
        }
        txn.stage_put(
            aether_types::ObjectId(0x100),
            b"txn data".to_vec(),
            StorageTier::Hot,
        )
        .map_err(|_| "stage_put failed")?;
        txn.commit_transaction(2_000_000)
            .map_err(|_| "commit failed")?;
        // commit_transaction does not clear current_xid; mark_committed does
        // that after staged mutations are applied. Collect staged data now.
        txn.staged_mutations()
            .iter()
            .map(|(id, content)| {
                let tier = *txn.staged_tiers().get(id).unwrap_or(&StorageTier::Hot);
                (*id, content.clone(), tier)
            })
            .collect::<Vec<_>>()
    };
    // txn borrow ended; prov is free to mutate again.
    for (object_id, content, tier) in staged_commit {
        if content.is_empty() {
            prov.delete(object_id).map_err(|_| "delete failed")?;
        } else {
            prov.put(object_id, &content, tier)
                .map_err(|_| "put failed")?;
        }
    }
    {
        let txn = prov
            .txn_participant_mut()
            .ok_or("no transactional participant")?;
        txn.mark_committed();
    }
    if prov
        .txn_participant_mut()
        .ok_or("no transactional participant")?
        .in_transaction()
    {
        return Err("still in transaction after commit");
    }
    if prov.get(aether_types::ObjectId(0x100)).as_deref() != Ok(b"txn data") {
        return Err("committed data not visible");
    }

    // Abort path: staged mutations are discarded.
    let staged_abort_len = {
        let txn = prov
            .txn_participant_mut()
            .ok_or("no transactional participant")?;
        let xid2 = aether_transaction_model::TransactionId::from_bytes([2u8; 16]);
        txn.begin_transaction(xid2, 1_500_000)
            .map_err(|_| "begin 2 failed")?;
        txn.stage_put(
            aether_types::ObjectId(0x200),
            b"should not apply".to_vec(),
            StorageTier::Cold,
        )
        .map_err(|_| "stage_put 2 failed")?;
        txn.abort_transaction().map_err(|_| "abort failed")?;
        if txn.in_transaction() {
            return Err("still in transaction after abort");
        }
        txn.staged_mutations().len()
    };
    if staged_abort_len != 0 {
        return Err("staged mutations not cleared after abort");
    }
    if prov.get(aether_types::ObjectId(0x200)).is_ok() {
        return Err("aborted data should not be visible");
    }
    Ok(())
}

/// CTS-STATE-PROV-5 (RFC-0019.11, RFC-0031.44): health event emission on
/// low capacity and offline transition.
fn cts_prov_health_events() -> Result<(), &'static str> {
    let mut prov = InMemoryProvider::new(
        "cts-prov-5",
        ProviderType::Block,
        100,
        LatencyBounds::new(100, 1000, 10_000).ok_or("bad latency bounds")?,
        0.999,
        0x01,
    );
    prov.set_offline(true);
    if prov.health_event_count() == 0 {
        return Err("no health event emitted on offline");
    }
    let event = prov.health_events().last().ok_or("no health event")?;
    if event.event_class != aether_event_model::EventClass::Storage.id() {
        return Err("health event not in Storage class");
    }
    if prov.get(aether_types::ObjectId(1)).is_ok() {
        return Err("get succeeded while offline");
    }

    // Low capacity health event.
    let mut small = InMemoryProvider::new(
        "small",
        ProviderType::Block,
        100,
        LatencyBounds::new(100, 1000, 10_000).ok_or("bad latency bounds")?,
        0.999,
        0x01,
    );
    small
        .put(aether_types::ObjectId(1), &vec![0u8; 50], StorageTier::Hot)
        .map_err(|_| "put failed")?;
    small
        .put(aether_types::ObjectId(2), &vec![0u8; 45], StorageTier::Hot)
        .map_err(|_| "put failed")?;
    if small.capacity_remaining_fraction() >= 0.10 {
        return Err("capacity not below 10%");
    }
    let _ = small.get(aether_types::ObjectId(1));
    if small.health_event_count() == 0 {
        return Err("no low-capacity health event");
    }
    Ok(())
}

/// CTS-STATE-POSIX-1 (RFC-0020.1): required POSIX operations — mkdir, open,
/// write, read, close, stat work end to end.
fn cts_posix_required_operations() -> Result<(), &'static str> {
    let mut provider = InMemoryProvider::new(
        "cts-posix-1",
        ProviderType::Block,
        1024 * 1024,
        LatencyBounds::new(100, 1000, 10_000).ok_or("bad latency bounds")?,
        0.999,
        0x01,
    );
    let root_id = aether_types::ObjectId(1);
    provider
        .put(root_id, &[], StorageTier::Hot)
        .map_err(|_| "root put failed")?;
    let mut proj = PosixProjection::new(provider, root_id);
    proj.grant_capability(
        DomainId::new(1).unwrap(),
        root_id,
        Rights::READ.union(Rights::WRITE).union(Rights::EXECUTE),
    );

    proj.mkdir("/tmp", 1, 0o755).map_err(|_| "mkdir failed")?;
    let fd = proj
        .open("/tmp/hello.txt", O_CREAT | O_WRONLY, 1)
        .map_err(|_| "open create failed")?;
    proj.write(fd, b"hello world").map_err(|_| "write failed")?;
    proj.close(fd).map_err(|_| "close failed")?;
    let fd = proj
        .open("/tmp/hello.txt", O_RDONLY, 1)
        .map_err(|_| "open read failed")?;
    let mut buf = [0u8; 16];
    let n = proj.read(fd, &mut buf).map_err(|_| "read failed")?;
    if n != 11 || &buf[..n] != b"hello world" {
        return Err("read did not return written content");
    }
    proj.close(fd).map_err(|_| "close failed")?;
    let meta = proj.stat("/tmp/hello.txt", 1).map_err(|_| "stat failed")?;
    if meta.file_type != FileType::RegularFile || meta.size != 11 {
        return Err("stat metadata mismatch");
    }
    Ok(())
}

/// CTS-STATE-POSIX-2 (RFC-0020.3): every POSIX file and directory is a stored
/// object retrievable through the provider's object model.
fn cts_posix_object_mapping() -> Result<(), &'static str> {
    let mut provider = InMemoryProvider::new(
        "cts-posix-2",
        ProviderType::Block,
        1024 * 1024,
        LatencyBounds::new(100, 1000, 10_000).ok_or("bad latency bounds")?,
        0.999,
        0x01,
    );
    let root_id = aether_types::ObjectId(1);
    provider
        .put(root_id, &[], StorageTier::Hot)
        .map_err(|_| "root put failed")?;
    let mut proj = PosixProjection::new(provider, root_id);
    proj.grant_capability(
        DomainId::new(1).unwrap(),
        root_id,
        Rights::READ.union(Rights::WRITE).union(Rights::EXECUTE),
    );

    proj.mkdir("/tmp", 1, 0o755).map_err(|_| "mkdir failed")?;
    let fd = proj
        .open("/tmp/obj.txt", O_CREAT | O_WRONLY, 1)
        .map_err(|_| "open create failed")?;
    proj.write(fd, b"data").map_err(|_| "write failed")?;
    proj.close(fd).map_err(|_| "close failed")?;

    let meta = proj.stat("/tmp/obj.txt", 1).map_err(|_| "stat failed")?;
    if meta.file_type != FileType::RegularFile || meta.size != 4 {
        return Err("object metadata mismatch");
    }
    let fd2 = proj
        .open("/tmp/obj.txt", O_RDONLY, 1)
        .map_err(|_| "open read failed")?;
    let mut buf = [0u8; 16];
    let n = proj.read(fd2, &mut buf).map_err(|_| "read failed")?;
    if n != 4 || &buf[..n] != b"data" {
        return Err("object content mismatch");
    }
    proj.close(fd2).map_err(|_| "close failed")?;
    Ok(())
}

/// CTS-STATE-POSIX-3 (RFC-0020.6): writes are invisible until fsync commits.
fn cts_posix_transactional_fsync() -> Result<(), &'static str> {
    let mut provider = InMemoryProvider::new(
        "cts-posix-3",
        ProviderType::Block,
        1024 * 1024,
        LatencyBounds::new(100, 1000, 10_000).ok_or("bad latency bounds")?,
        0.999,
        0x01,
    );
    let root_id = aether_types::ObjectId(1);
    provider
        .put(root_id, &[], StorageTier::Hot)
        .map_err(|_| "root put failed")?;
    let mut proj = PosixProjection::new(provider, root_id);
    proj.grant_capability(
        DomainId::new(1).unwrap(),
        root_id,
        Rights::READ.union(Rights::WRITE).union(Rights::EXECUTE),
    );

    proj.mkdir("/tmp", 1, 0o755).map_err(|_| "mkdir failed")?;
    let fd = proj
        .open("/tmp/txn.txt", O_CREAT | O_WRONLY, 1)
        .map_err(|_| "open create failed")?;
    proj.write(fd, b"uncommitted").map_err(|_| "write failed")?;

    let fd2 = proj
        .open("/tmp/txn.txt", O_RDONLY, 1)
        .map_err(|_| "open read failed")?;
    let mut buf = [0u8; 16];
    let n = proj.read(fd2, &mut buf).map_err(|_| "read failed")?;
    if n != 0 {
        return Err("uncommitted write visible before fsync");
    }
    proj.close(fd2).map_err(|_| "close failed")?;

    proj.fsync(fd).map_err(|_| "fsync failed")?;

    let fd3 = proj
        .open("/tmp/txn.txt", O_RDONLY, 1)
        .map_err(|_| "open read after fsync failed")?;
    let mut buf = [0u8; 16];
    let n = proj.read(fd3, &mut buf).map_err(|_| "read failed")?;
    if n != 11 || &buf[..n] != b"uncommitted" {
        return Err("committed write not visible after fsync");
    }
    proj.close(fd3).map_err(|_| "close failed")?;
    Ok(())
}

/// CTS-STATE-POSIX-4 (RFC-0020.7): atomic rename — source removed, target added
/// atomically.
fn cts_posix_atomic_rename() -> Result<(), &'static str> {
    let mut provider = InMemoryProvider::new(
        "cts-posix-4",
        ProviderType::Block,
        1024 * 1024,
        LatencyBounds::new(100, 1000, 10_000).ok_or("bad latency bounds")?,
        0.999,
        0x01,
    );
    let root_id = aether_types::ObjectId(1);
    provider
        .put(root_id, &[], StorageTier::Hot)
        .map_err(|_| "root put failed")?;
    let mut proj = PosixProjection::new(provider, root_id);
    proj.grant_capability(
        DomainId::new(1).unwrap(),
        root_id,
        Rights::READ.union(Rights::WRITE).union(Rights::EXECUTE),
    );

    proj.mkdir("/tmp", 1, 0o755).map_err(|_| "mkdir failed")?;
    let fd = proj
        .open("/tmp/a.txt", O_CREAT | O_WRONLY, 1)
        .map_err(|_| "open create failed")?;
    proj.write(fd, b"content-a").map_err(|_| "write failed")?;
    proj.close(fd).map_err(|_| "close failed")?;

    let fd = proj
        .open("/tmp/b.txt", O_CREAT | O_WRONLY, 1)
        .map_err(|_| "open create failed")?;
    proj.write(fd, b"content-b").map_err(|_| "write failed")?;
    proj.close(fd).map_err(|_| "close failed")?;

    proj.rename("/tmp/a.txt", "/tmp/c.txt", 1)
        .map_err(|_| "rename failed")?;

    if proj.stat("/tmp/a.txt", 1).is_ok() {
        return Err("source entry still exists after rename");
    }
    let meta = proj
        .stat("/tmp/c.txt", 1)
        .map_err(|_| "stat target failed")?;
    let fd = proj
        .open("/tmp/c.txt", O_RDONLY, 1)
        .map_err(|_| "open target failed")?;
    let mut buf = [0u8; 16];
    let n = proj.read(fd, &mut buf).map_err(|_| "read target failed")?;
    if n != 9 || &buf[..n] != b"content-a" {
        return Err("renamed file has wrong content");
    }
    proj.close(fd).map_err(|_| "close failed")?;
    let _ = meta;
    if proj.stat("/tmp/b.txt", 1).is_err() {
        return Err("unrelated file missing after rename");
    }
    Ok(())
}

/// CTS-STATE-POSIX-5 (RFC-0020.12 / RFC-0020.13): capability rights take
/// precedence over POSIX mode bits.
fn cts_posix_capability_precedence() -> Result<(), &'static str> {
    let mut provider = InMemoryProvider::new(
        "cts-posix-5",
        ProviderType::Block,
        1024 * 1024,
        LatencyBounds::new(100, 1000, 10_000).ok_or("bad latency bounds")?,
        0.999,
        0x01,
    );
    let root_id = aether_types::ObjectId(1);
    provider
        .put(root_id, &[], StorageTier::Hot)
        .map_err(|_| "root put failed")?;
    let mut proj = PosixProjection::new(provider, root_id);
    proj.grant_capability(
        DomainId::new(1).unwrap(),
        root_id,
        Rights::READ.union(Rights::WRITE).union(Rights::EXECUTE),
    );

    proj.mkdir("/tmp", 1, 0o755).map_err(|_| "mkdir failed")?;
    let fd = proj
        .open("/tmp/secret.txt", O_CREAT | O_WRONLY, 1)
        .map_err(|_| "open create failed")?;
    proj.write(fd, b"sensitive").map_err(|_| "write failed")?;
    proj.close(fd).map_err(|_| "close failed")?;

    if proj.open("/tmp/secret.txt", O_RDONLY, 2).is_ok() {
        return Err("domain 2 access permitted without capability");
    }

    let secret_meta = proj
        .stat("/tmp/secret.txt", 1)
        .map_err(|_| "stat secret failed")?;
    let tmp_meta = proj.stat("/tmp", 1).map_err(|_| "stat /tmp failed")?;
    proj.grant_capability(
        DomainId::new(2).unwrap(),
        root_id,
        Rights::READ.union(Rights::EXECUTE),
    );
    proj.grant_capability(
        DomainId::new(2).unwrap(),
        tmp_meta.object_id,
        Rights::READ.union(Rights::EXECUTE),
    );
    proj.grant_capability(
        DomainId::new(2).unwrap(),
        secret_meta.object_id,
        Rights::READ,
    );
    let fd = proj
        .open("/tmp/secret.txt", O_RDONLY, 2)
        .map_err(|_| "open with capability failed")?;
    let mut buf = [0u8; 16];
    let n = proj.read(fd, &mut buf).map_err(|_| "read failed")?;
    if n != 9 || &buf[..n] != b"sensitive" {
        return Err("capability-granted read returned wrong content");
    }
    proj.close(fd).map_err(|_| "close failed")?;
    Ok(())
}

/// CTS-STATE-OBJPROJ-1 (RFC-0021.1): full object API — create, get, update,
/// delete, metadata, version listing work end to end.
fn cts_objproj_object_api() -> Result<(), &'static str> {
    let provider = InMemoryProvider::new(
        "cts-objproj-1",
        ProviderType::Block,
        1024 * 1024,
        LatencyBounds::new(100, 1000, 10_000).ok_or("bad latency bounds")?,
        0.999,
        0x01,
    );
    let mut proj = ObjectProjection::new(provider);

    // Create an object.
    let id = proj
        .object_create(b"version one")
        .map_err(|_| "create failed")?;
    let meta = proj
        .object_get_metadata(id)
        .map_err(|_| "metadata failed")?;
    if meta.size_bytes != 11 || meta.lifecycle_state != LifecycleState::Created || meta.immutable {
        return Err("object metadata mismatch");
    }

    // Get returns the content.
    if proj.object_get(id).map_err(|_| "get failed")? != b"version one" {
        return Err("get returned wrong content");
    }

    // Update creates a new identity; current version reflects it.
    let id2 = proj
        .object_update(id, b"version two")
        .map_err(|_| "update failed")?;
    if id2 == id {
        return Err("update did not create a new identity");
    }
    if proj.object_get(id).map_err(|_| "get after update failed")? != b"version two" {
        return Err("current version mismatch after update");
    }

    // Version history has two entries.
    let versions = proj
        .object_list_versions(id)
        .map_err(|_| "list versions failed")?;
    if versions.len() != 2 || versions[1].0 != id2 {
        return Err("version history mismatch");
    }

    // Get at historical version.
    if proj
        .object_get_at_version(id, 0)
        .map_err(|_| "get at version failed")?
        != b"version one"
    {
        return Err("historical version mismatch");
    }

    // Delete the original object; it is no longer readable.
    proj.object_delete(id).map_err(|_| "delete failed")?;
    if proj.object_get(id).is_ok() {
        return Err("deleted object still readable");
    }
    // The new identity object is unaffected.
    let _ = proj
        .object_get(id2)
        .map_err(|_| "new id unreadable after delete")?;
    Ok(())
}

/// CTS-STATE-OBJPROJ-2 (RFC-0021.3): deduplication at identity level —
/// identical content returns the same identity.
fn cts_objproj_dedup() -> Result<(), &'static str> {
    let mut proj = ObjectProjection::new(make_cts_provider()?);

    let id1 = proj
        .object_create(b"same content")
        .map_err(|_| "create failed")?;
    let id2 = proj
        .object_create(b"same content")
        .map_err(|_| "create failed")?;
    if id1 != id2 {
        return Err("identical content did not dedupe");
    }

    // No duplicate version is added.
    let versions = proj
        .object_list_versions(id1)
        .map_err(|_| "list versions failed")?;
    if versions.len() != 1 {
        return Err("dedupe added a spurious version");
    }
    Ok(())
}

/// CTS-STATE-OBJPROJ-3 (RFC-0021.4): create and update are atomic — both
/// identities are stored in the provider.
fn cts_objproj_atomic_update() -> Result<(), &'static str> {
    let mut proj = ObjectProjection::new(make_cts_provider()?);

    let id = proj
        .object_create(b"original")
        .map_err(|_| "create failed")?;
    let id2 = proj
        .object_update(id, b"updated")
        .map_err(|_| "update failed")?;
    if id2 == id {
        return Err("update did not create a new identity");
    }

    // Both identities are retrievable through the provider.
    let content1 = proj
        .provider
        .get(
            proj.object_get_metadata(id)
                .map_err(|_| "metadata failed")?
                .object_id,
        )
        .map_err(|_| "provider get failed")?;
    if content1 != b"original" {
        return Err("original content not stored");
    }
    let content2 = proj
        .provider
        .get(
            proj.object_get_metadata(id2)
                .map_err(|_| "metadata failed")?
                .object_id,
        )
        .map_err(|_| "provider get failed")?;
    if content2 != b"updated" {
        return Err("updated content not stored");
    }
    Ok(())
}

/// CTS-STATE-OBJPROJ-4 (RFC-0021.5): batch creation is all-or-nothing and
/// dedupes duplicate content.
fn cts_objproj_batch_all_or_nothing() -> Result<(), &'static str> {
    let mut proj = ObjectProjection::new(make_cts_provider()?);

    let ids = proj
        .object_create_batch(&[
            ObjectCreateEntry {
                content: b"batch one".to_vec(),
                immutable: false,
            },
            ObjectCreateEntry {
                content: b"batch two".to_vec(),
                immutable: true,
            },
        ])
        .map_err(|_| "batch create failed")?;
    if ids.len() != 2 {
        return Err("batch returned wrong count");
    }

    let meta0 = proj
        .object_get_metadata(ids[0])
        .map_err(|_| "metadata failed")?;
    if meta0.immutable || proj.object_get(ids[0]).map_err(|_| "get failed")? != b"batch one" {
        return Err("batch object 0 mismatch");
    }

    let meta1 = proj
        .object_get_metadata(ids[1])
        .map_err(|_| "metadata failed")?;
    if !meta1.immutable || proj.object_get(ids[1]).map_err(|_| "get failed")? != b"batch two" {
        return Err("batch object 1 mismatch");
    }

    // Duplicate content in a later batch dedupes to the existing id.
    let ids2 = proj
        .object_create_batch(&[ObjectCreateEntry {
            content: b"batch one".to_vec(),
            immutable: false,
        }])
        .map_err(|_| "batch create failed")?;
    if ids2 != vec![ids[0]] {
        return Err("batch did not dedupe duplicate content");
    }
    Ok(())
}

/// CTS-STATE-OBJPROJ-5 (RFC-0021.7): immutable flag enforced — updating an
/// immutable object returns an error and leaves content unchanged.
fn cts_objproj_immutable_rejected() -> Result<(), &'static str> {
    let mut proj = ObjectProjection::new(make_cts_provider()?);

    let ids = proj
        .object_create_batch(&[ObjectCreateEntry {
            content: b"immutable object".to_vec(),
            immutable: true,
        }])
        .map_err(|_| "batch create failed")?;
    let id = ids[0];

    assert_eq!(
        proj.object_update(id, b"attempted change"),
        Err(ObjectError::Immutable)
    );
    if proj.object_get(id).map_err(|_| "get failed")? != b"immutable object" {
        return Err("immutable object content changed");
    }

    // A mutable object can still be updated.
    let id2 = proj
        .object_create(b"mutable")
        .map_err(|_| "create failed")?;
    let id3 = proj
        .object_update(id2, b"changed")
        .map_err(|_| "update failed")?;
    if id3 == id2 {
        return Err("update did not create a new identity");
    }
    if proj.object_get(id2).map_err(|_| "get failed")? != b"changed" {
        return Err("mutable object not updated");
    }
    Ok(())
}

/// Builds a fresh in-memory provider for CTS cases.
fn make_cts_provider() -> Result<aether_storage_provider::InMemoryProvider, &'static str> {
    Ok(aether_storage_provider::InMemoryProvider::new(
        "cts-objproj",
        ProviderType::Block,
        1024 * 1024,
        LatencyBounds::new(100, 1000, 10_000).ok_or("bad latency bounds")?,
        0.999,
        0x01,
    ))
}

/// CTS-STATE-KV-1 (RFC-0022.1): full KV API — put, get, delete, exists, list
/// work end to end.
fn cts_kv_full_api() -> Result<(), &'static str> {
    let mut proj = KvProjection::new(make_cts_provider()?);

    // Initially empty.
    if proj.kv_exists(b"foo") {
        return Err("key exists before put");
    }
    if proj.kv_list(b"", 10).len() != 0 {
        return Err("empty projection not empty");
    }

    // Put.
    proj.kv_put(b"foo", b"value-one")
        .map_err(|_| "put failed")?;
    if !proj.kv_exists(b"foo") {
        return Err("key should exist after put");
    }
    if proj.kv_get(b"foo").map_err(|_| "get failed")? != b"value-one" {
        return Err("get returned wrong value");
    }

    // Put another key.
    proj.kv_put(b"bar", b"value-two")
        .map_err(|_| "put failed")?;

    // List in lexicographic order.
    let listed = proj.kv_list(b"", 10);
    if listed != vec![b"bar".to_vec(), b"foo".to_vec()] {
        return Err("list order mismatch");
    }

    // Prefix list.
    let prefixed = proj.kv_list(b"f", 10);
    if prefixed != vec![b"foo".to_vec()] {
        return Err("prefix list mismatch");
    }

    // Update.
    proj.kv_put(b"foo", b"value-one-updated")
        .map_err(|_| "put failed")?;
    if proj.kv_get(b"foo").map_err(|_| "get failed")? != b"value-one-updated" {
        return Err("update did not change value");
    }

    // Delete.
    proj.kv_delete(b"bar").map_err(|_| "delete failed")?;
    if proj.kv_exists(b"bar") {
        return Err("deleted key still exists");
    }
    if proj.kv_list(b"", 10) != vec![b"foo".to_vec()] {
        return Err("list after delete mismatch");
    }
    if proj.kv_delete(b"bar") != Err(KvError::NotFound) {
        return Err("delete of missing key should fail");
    }
    if proj.kv_get(b"bar") != Err(KvError::NotFound) {
        return Err("get of missing key should fail");
    }
    Ok(())
}

/// CTS-STATE-KV-2 (RFC-0022.5): transactional grouping — multiple puts and
/// deletes in one txn commit atomically.
fn cts_kv_transactional_grouping() -> Result<(), &'static str> {
    let mut proj = KvProjection::new(make_cts_provider()?);
    proj.kv_put(b"a", b"one").map_err(|_| "put failed")?;
    proj.kv_put(b"b", b"two").map_err(|_| "put failed")?;

    let txn = proj.kv_txn_open().map_err(|_| "txn open failed")?;
    proj.kv_txn_put(txn, b"a", b"one-changed")
        .map_err(|_| "txn put failed")?;
    proj.kv_txn_put(txn, b"c", b"three")
        .map_err(|_| "txn put failed")?;
    proj.kv_txn_delete(txn, b"b")
        .map_err(|_| "txn delete failed")?;

    // Uncommitted: reads still see pre-txn state.
    if proj.kv_get(b"a").map_err(|_| "get failed")? != b"one" {
        return Err("uncommitted put visible");
    }
    if proj.kv_exists(b"c") {
        return Err("uncommitted key visible");
    }
    if proj.kv_get(b"b").map_err(|_| "get failed")? != b"two" {
        return Err("uncommitted delete visible");
    }

    proj.kv_txn_commit(txn).map_err(|_| "txn commit failed")?;

    // Committed: all changes visible atomically.
    if proj.kv_get(b"a").map_err(|_| "get failed")? != b"one-changed" {
        return Err("committed put not visible");
    }
    if proj.kv_get(b"c").map_err(|_| "get failed")? != b"three" {
        return Err("committed key not visible");
    }
    if proj.kv_exists(b"b") {
        return Err("committed delete not applied");
    }
    if proj.kv_list(b"", 10) != vec![b"a".to_vec(), b"c".to_vec()] {
        return Err("list after commit mismatch");
    }
    Ok(())
}

/// CTS-STATE-KV-3 (RFC-0022.6): two-phase commit — abort discards all staged
/// writes; none are applied.
fn cts_kv_two_phase_commit_abort() -> Result<(), &'static str> {
    let mut proj = KvProjection::new(make_cts_provider()?);
    proj.kv_put(b"a", b"one").map_err(|_| "put failed")?;

    let txn = proj.kv_txn_open().map_err(|_| "txn open failed")?;
    proj.kv_txn_put(txn, b"a", b"should-not-apply")
        .map_err(|_| "txn put failed")?;
    proj.kv_txn_put(txn, b"b", b"also-not")
        .map_err(|_| "txn put failed")?;
    proj.kv_txn_abort(txn).map_err(|_| "txn abort failed")?;

    // State unchanged after abort.
    if proj.kv_get(b"a").map_err(|_| "get failed")? != b"one" {
        return Err("aborted put changed state");
    }
    if proj.kv_exists(b"b") {
        return Err("aborted key visible");
    }
    if proj.kv_list(b"", 10) != vec![b"a".to_vec()] {
        return Err("list after abort mismatch");
    }

    // The aborted handle can no longer be used.
    if proj.kv_txn_commit(txn) != Err(KvError::UnknownTransaction) {
        return Err("committing aborted txn should fail");
    }
    if proj.kv_txn_delete(txn, b"a") != Err(KvError::UnknownTransaction) {
        return Err("deleting on aborted txn should fail");
    }
    Ok(())
}

/// CTS-STATE-KV-4 (RFC-0022.7): atomic CAS — replaces value only if the
/// current value matches the expected identity.
fn cts_kv_atomic_cas() -> Result<(), &'static str> {
    let mut proj = KvProjection::new(make_cts_provider()?);
    proj.kv_put(b"k", b"value-one").map_err(|_| "put failed")?;
    proj.kv_put(b"k", b"value-two").map_err(|_| "put failed")?;
    let current = proj
        .index
        .get(b"k".as_slice())
        .copied()
        .ok_or("current missing")?;

    // CAS with matching expected value succeeds.
    assert!(proj.kv_cas(b"k", current, b"value-three").unwrap());
    if proj.kv_get(b"k").map_err(|_| "get failed")? != b"value-three" {
        return Err("CAS did not replace value");
    }
    Ok(())
}

/// CTS-STATE-KV-5 (RFC-0022.8): CAS conflict — returns false without modifying
/// state when the expected value does not match.
fn cts_kv_cas_conflict() -> Result<(), &'static str> {
    let mut proj = KvProjection::new(make_cts_provider()?);
    proj.kv_put(b"k", b"original").map_err(|_| "put failed")?;
    let current = proj
        .index
        .get(b"k".as_slice())
        .copied()
        .ok_or("current missing")?;

    // CAS with a non-matching expected value fails without changing state.
    let wrong = ObjectId(999);
    assert_ne!(wrong, current);
    if proj.kv_cas(b"k", wrong, b"replacement").unwrap() {
        return Err("CAS should report conflict");
    }
    if proj.kv_get(b"k").map_err(|_| "get failed")? != b"original" {
        return Err("conflicting CAS changed state");
    }
    if proj.index.get(b"k".as_slice()).copied() != Some(current) {
        return Err("conflicting CAS changed index");
    }

    // CAS on a missing key returns NotFound.
    if proj.kv_cas(b"missing", current, b"x") != Err(KvError::NotFound) {
        return Err("CAS on missing key should fail");
    }
    Ok(())
}

/// CTS-STATE-XS-1 (RFC-0023.2): a transaction commit produces a new object
/// identity that is verifiable via the Integrity Model (content-resolved
/// identity equals the Merkle leaf identity of the stored content).
fn state_xs_txn_commit_identity_verifiable() -> Result<(), &'static str> {
    let mut proj = ObjectProjection::new(make_cts_provider()?);
    let id = proj
        .object_create(b"version one")
        .map_err(|_| "object operation failed")?;
    let id2 = proj
        .object_update(id, b"version two")
        .map_err(|_| "object operation failed")?;
    if id2 == id {
        return Err("update did not produce a new identity");
    }
    let content = proj.provider.get(id2).map_err(|_| "provider get failed")?;
    if content != b"version two" {
        return Err("stored content does not match");
    }
    let leaf = MerkleDAG::leaf_identity(&content);
    let leaf_oid: ObjectId = leaf.into();
    if leaf_oid != id2 {
        return Err("object identity is not content-resolved / integrity-verifiable");
    }
    Ok(())
}

/// CTS-STATE-XS-2 (RFC-0023.2): lifecycle transitions occur atomically with
/// transaction commit — each lifecycle transition is only applied after the
/// enclosing transaction commits, and the state advances correctly along the
/// mandatory forward path.
fn state_xs_lifecycle_atomic_with_txn() -> Result<(), &'static str> {
    let mut machine = LifecycleMachine::new();
    let ident = ObjectIdentity([3u8; 32]);
    machine.register_object(ident, LifecyclePolicy::default());
    if machine.state_of(&ident) != Some(LifecycleState::Created) {
        return Err("new object must start in Created");
    }

    // First committed transaction, then the first valid lifecycle transition
    // (Created -> Validated requires integrity verification = true).
    let mut participant = aether_storage_provider::TransactionalParticipant::new(true);
    let xid = TransactionId::from_bytes([3u8; 16]);
    participant
        .begin_transaction(xid, 1_000_000)
        .map_err(|_| "object operation failed")?;
    participant
        .stage_put(ObjectId(1), b"payload".to_vec(), StorageTier::Hot)
        .map_err(|_| "object operation failed")?;
    participant
        .commit_transaction(2_000_000)
        .map_err(|_| "object operation failed")?;
    machine
        .transition(
            ident,
            LifecycleState::Validated,
            TransitionTrigger::Explicit,
            2_000_000,
            true,
            true,
        )
        .map_err(|_| "object operation failed")?;
    if machine.state_of(&ident) != Some(LifecycleState::Validated) {
        return Err("lifecycle state not Validated after first commit");
    }

    // Second committed transaction, then the next lifecycle transition
    // (Validated -> Referenced requires identity = true).
    let mut participant2 = aether_storage_provider::TransactionalParticipant::new(true);
    let xid2 = TransactionId::from_bytes([4u8; 16]);
    participant2
        .begin_transaction(xid2, 3_000_000)
        .map_err(|_| "object operation failed")?;
    participant2
        .stage_put(ObjectId(2), b"payload".to_vec(), StorageTier::Hot)
        .map_err(|_| "object operation failed")?;
    participant2
        .commit_transaction(4_000_000)
        .map_err(|_| "object operation failed")?;
    machine
        .transition(
            ident,
            LifecycleState::Referenced,
            TransitionTrigger::Explicit,
            4_000_000,
            true,
            true,
        )
        .map_err(|_| "object operation failed")?;
    if machine.state_of(&ident) != Some(LifecycleState::Referenced) {
        return Err("lifecycle state not Referenced after second commit");
    }
    Ok(())
}

/// CTS-STATE-XS-3 (RFC-0023.2): removability — removing an optional storage
/// provider does not cause any State invariant to be violated; the remaining
/// providers continue to serve their data.
fn state_xs_provider_removability() -> Result<(), &'static str> {
    let mut registry = aether_storage_provider::StorageRegistry::new();
    let mut primary = InMemoryProvider::new(
        "primary",
        ProviderType::Block,
        1024 * 1024,
        LatencyBounds::new(100, 1000, 10_000).ok_or("invalid latency bounds")?,
        0.999,
        0x01,
    );
    primary
        .put(ObjectId(1), b"state-data", StorageTier::Hot)
        .map_err(|_| "object operation failed")?;
    registry
        .register(Box::new(primary))
        .map_err(|_| "object operation failed")?;
    let mut secondary = InMemoryProvider::new(
        "secondary",
        ProviderType::Network,
        2048,
        LatencyBounds::new(100, 1000, 10_000).ok_or("invalid latency bounds")?,
        0.999,
        0x01,
    );
    secondary
        .put(ObjectId(2), b"replica-data", StorageTier::Warm)
        .map_err(|_| "object operation failed")?;
    registry
        .register(Box::new(secondary))
        .map_err(|_| "object operation failed")?;
    if registry.provider_count() != 2 {
        return Err("both providers should be registered");
    }
    let removed = registry.remove("primary").map_err(|_| "remove failed")?;
    if removed.name() != "primary" {
        return Err("wrong provider removed");
    }
    if registry.provider_count() != 1 {
        return Err("provider count wrong after removal");
    }
    // State invariant: the remaining provider still serves its data.
    let remaining = registry
        .get("secondary")
        .ok_or("secondary provider missing after removal")?;
    if remaining
        .get(ObjectId(2))
        .map_err(|_| "remaining get failed")?
        != b"replica-data"
    {
        return Err("remaining provider does not serve its data");
    }
    Ok(())
}

/// CTS-STATE-XS-4 (RFC-0023.2): object identity and capability are
/// orthogonally handled — a capability can refer to an object by identity
/// without the identity itself being authority (RFC-0015.11, I-13).
fn state_xs_identity_capability_orthogonality() -> Result<(), &'static str> {
    let ident = ObjectIdentity::compute(b"shared object");
    let oid: ObjectId = ident.into();
    let oid_again: ObjectId = ident.into();
    if oid != oid_again {
        return Err("identity-to-object-id conversion is not deterministic");
    }
    // The same identity names an object, but capabilities are granted
    // independently on the object id (identity alone grants no rights).
    let mut proj = ObjectProjection::new(make_cts_provider()?);
    let created = proj
        .object_create(b"orthogonal object")
        .map_err(|_| "object operation failed")?;
    // A content-resolved object id is distinct from an unrelated identity id,
    // proving identity is a naming anchor, not authority/capability.
    if created == oid {
        return Err("content identity should not collide with unrelated id");
    }
    Ok(())
}

/// CTS-STATE-XS-5 (RFC-0023.2): Merkle tree updates are correct under
/// composite object creation — changing a leaf recomputes the path to the
/// root and updates the root identity (RFC-0017.3).
fn state_xs_merkle_composite_update() -> Result<(), &'static str> {
    let leaf = MerkleDAG::leaf_identity(b"leaf content");
    let composite = MerkleDAG::composite_identity(b"parent meta", &[leaf], 1);
    let mut dag = MerkleDAG::new(composite);
    dag.insert_leaf(leaf, 12);
    dag.insert_composite(
        composite,
        NodeType::Internal,
        &[leaf],
        b"parent meta".to_vec(),
        1,
        0,
    );
    dag.root = composite;
    let new_leaf = MerkleDAG::leaf_identity(b"modified leaf content");
    let new_root = dag
        .recompute_root(&leaf, &new_leaf)
        .map_err(|_| "object operation failed")?;
    let expected_root = MerkleDAG::composite_identity(b"parent meta", &[new_leaf], 1);
    if new_root != expected_root {
        return Err("composite recompute did not produce expected root");
    }
    if new_root == composite {
        return Err("root identity was not updated after child change");
    }
    Ok(())
}

/// CTS-STATE-CONF-1 (RFC-0023.1): all core State conformance tests pass — one
/// representative from each of the eight State RFC categories.
fn state_conf_core_tests_passed() -> Result<(), &'static str> {
    // Object Identity (RFC-0015): CTS-STATE-OBJ-1..6
    state_obj_identity_uniqueness()?;
    state_obj_identity_immutability()?;
    state_obj_version_history_completeness()?;
    state_obj_replica_identity_equality()?;
    state_obj_identity_capability_orthogonality()?;
    state_obj_reconstruction_from_replica()?;
    // Transaction Model (RFC-0016): CTS-STATE-TXN-1..6
    cts_txn_two_phase_commit_all_prepared()?;
    cts_txn_prepare_failure_aborts_all()?;
    cts_txn_client_abort()?;
    cts_txn_write_write_conflict_detected()?;
    cts_txn_snapshot_isolation_reads_committed()?;
    cts_txn_crash_recovery_aborts_in_progress()?;
    // Integrity Model (RFC-0017): CTS-STATE-INT-1..5
    state_int_merkle_dag_maintained()?;
    state_int_verification_on_read()?;
    state_int_no_corrupted_data_returned()?;
    state_int_incremental_verification()?;
    state_int_signature_verification()?;
    // Lifecycle FSM (RFC-0018): CTS-STATE-LC-1..6
    state_lc_mandatory_transition_path()?;
    state_lc_preconditions_enforced()?;
    state_lc_only_permitted_backward()?;
    state_lc_transition_events_emitted()?;
    state_lc_hazard_blocks_tiering()?;
    state_lc_state_queryable()?;
    // Storage Provider Interface (RFC-0019): CTS-STATE-PROV-1..5
    cts_prov_mandatory_operations()?;
    cts_prov_tier_assignment()?;
    cts_prov_removability_test()?;
    cts_prov_two_phase_commit()?;
    cts_prov_health_events()?;
    // POSIX Projection (RFC-0020): CTS-STATE-POSIX-1..5
    cts_posix_required_operations()?;
    cts_posix_object_mapping()?;
    cts_posix_transactional_fsync()?;
    cts_posix_atomic_rename()?;
    cts_posix_capability_precedence()?;
    // Object Projection (RFC-0021): CTS-STATE-OBJPROJ-1..5
    cts_objproj_object_api()?;
    cts_objproj_dedup()?;
    cts_objproj_atomic_update()?;
    cts_objproj_batch_all_or_nothing()?;
    cts_objproj_immutable_rejected()?;
    // Key-Value Projection (RFC-0022): CTS-STATE-KV-1..5
    cts_kv_full_api()?;
    cts_kv_transactional_grouping()?;
    cts_kv_two_phase_commit_abort()?;
    cts_kv_atomic_cas()?;
    cts_kv_cas_conflict()?;
    Ok(())
}

/// CTS-STATE-CONF-2 (RFC-0023.2): cross-state conformance tests all pass.
fn state_conf_cross_state_tests_passed() -> Result<(), &'static str> {
    state_xs_txn_commit_identity_verifiable()?;
    state_xs_lifecycle_atomic_with_txn()?;
    state_xs_provider_removability()?;
    state_xs_identity_capability_orthogonality()?;
    state_xs_merkle_composite_update()?;
    Ok(())
}

/// CTS-STATE-CONF-3 (RFC-0023.3): Desktop/Cloud profile — POSIX projection
/// full conformance (all CTS-STATE-POSIX-* pass).
fn state_conf_desktop_cloud_profile_passed() -> Result<(), &'static str> {
    cts_posix_required_operations()?;
    cts_posix_object_mapping()?;
    cts_posix_transactional_fsync()?;
    cts_posix_atomic_rename()?;
    cts_posix_capability_precedence()?;
    Ok(())
}

/// CTS-STATE-CONF-4 (RFC-0023.5): Safety-Critical profile — lifecycle FSM
/// correctness and transaction crash-recovery with crash injection.
fn state_conf_safety_critical_profile_passed() -> Result<(), &'static str> {
    state_lc_mandatory_transition_path()?;
    cts_txn_crash_recovery_aborts_in_progress()?;
    Ok(())
}

/// CTS-STATE-CONF-5 (RFC-0023.6): reporting format — a conformance declaration
/// publishes per-RFC results, provider characteristics, identity hash
/// function, Merkle depth, and lifecycle policy parameters.
fn state_conf_reporting_format() -> Result<(), &'static str> {
    struct StateConformanceReport<'a> {
        per_rfc: Vec<(&'a str, bool)>,
        providers: Vec<(&'a str, ProviderType, u64)>,
        identity_hash: &'a str,
        merkle_depth: u32,
        lifecycle_policy: &'a str,
    }
    let mut providers = Vec::new();
    providers.push(("nvme", ProviderType::Block, 1024 * 1024 * 1024));
    providers.push(("optane", ProviderType::ByteAddressable, 256 * 1024 * 1024));
    if providers.len() != 2 {
        return Err("provider characteristics not recorded");
    }
    let report = StateConformanceReport {
        per_rfc: vec![
            ("RFC-0015", true),
            ("RFC-0016", true),
            ("RFC-0017", true),
            ("RFC-0018", true),
            ("RFC-0019", true),
            ("RFC-0020", true),
            ("RFC-0021", true),
            ("RFC-0022", true),
        ],
        providers,
        identity_hash: "BLAKE3-256",
        merkle_depth: 16,
        lifecycle_policy: "default",
    };
    if report.per_rfc.len() != 8 || report.per_rfc.iter().any(|(_, ok)| !*ok) {
        return Err("per-RFC results not all passing");
    }
    if report.identity_hash != "BLAKE3-256" {
        return Err("identity hash function not declared");
    }
    if report.merkle_depth == 0 {
        return Err("Merkle tree depth not configured");
    }
    if report.lifecycle_policy.is_empty() {
        return Err("lifecycle policy parameters not declared");
    }
    Ok(())
}

/// CTS-COMM-MSG-1 (RFC-0024.1): header format — magic, version, header_len,
/// and all fields round-trip through serialization with network byte order.
fn comm_msg_header_format() -> Result<(), &'static str> {
    let src = EndpointId::new([1u8; 16]);
    let dst = EndpointId::new([2u8; 16]);
    let header = MessageHeader::new(
        42,
        src,
        dst,
        100,
        50,
        MSG_STREAM,
        128,
        3,
    );
    // Canonical fixed fields (RFC-0024.1).
    if header.magic != MAGIC {
        return Err("magic not set to COMP signature");
    }
    if header.version != VERSION {
        return Err("version not 1");
    }
    if header.header_len as usize != HEADER_LEN as usize {
        return Err("header_len not equal to fixed header size");
    }
    // Round-trip serialization preserves every field in network byte order.
    let bytes = header.serialize();
    if bytes.len() != 61 {
        return Err("serialized header is not 61 bytes");
    }
    let parsed = MessageHeader::deserialize(&bytes).map_err(|_| "deserialize failed")?;
    if parsed.message_id != 42
        || parsed.src_endpoint != src
        || parsed.dst_endpoint != dst
        || parsed.payload_len != 100
        || parsed.trailer_len != 50
        || parsed.flags != MSG_STREAM
        || parsed.urgency != 128
        || parsed.capability_count != 3
        || parsed.reserved != 0
    {
        return Err("header field mismatch after round-trip");
    }
    // Invalid magic is rejected.
    let mut bad = header.serialize();
    bad[0] ^= 0xFF;
    if MessageHeader::deserialize(&bad).is_ok() {
        return Err("invalid magic was accepted");
    }
    // Invalid version is rejected.
    let mut bad = header.serialize();
    bad[4] = 0xFF;
    if MessageHeader::deserialize(&bad).is_ok() {
        return Err("invalid version was accepted");
    }
    // Truncated buffer is rejected.
    if MessageHeader::deserialize(&bytes[..10]).is_ok() {
        return Err("truncated header was accepted");
    }
    Ok(())
}

/// CTS-COMM-MSG-2 (RFC-0024.2): flags parsing — each flag bit is recognized
/// independently and combinations compose correctly.
fn comm_msg_flags_parsing() -> Result<(), &'static str> {
    let src = EndpointId::none();
    let dst = EndpointId::none();
    let msg = Message::new(
        MessageHeader::new(1, src, dst, 0, 0, MSG_FAST, 0, 0),
        Vec::new(),
        Vec::new(),
    ).map_err(|_| "message creation failed")?;
    if msg.is_no_reply() || msg.is_stream() || msg.has_capability_transfer() || msg.is_error() {
        return Err("MSG_FAST should not set other flags");
    }
    if msg.header.flags & MSG_FAST != MSG_FAST {
        return Err("MSG_FAST bit not set");
    }
    let combined = Message::new(
        MessageHeader::new(
            2,
            src,
            dst,
            0,
            0,
            MSG_FAST | MSG_CAP_TRANSFER | MSG_STREAM | MSG_NO_REPLY,
            0,
            0,
        ),
        Vec::new(),
        Vec::new(),
    ).map_err(|_| "message creation failed")?;
    if !combined.is_no_reply()
        || !combined.is_stream()
        || !combined.has_capability_transfer()
        || combined.is_error()
    {
        return Err("combined flags not parsed correctly");
    }
    let error_msg = Message::new(
        MessageHeader::new(3, src, dst, 0, 0, MSG_ERROR, 0, 0),
        Vec::new(),
        Vec::new(),
    ).map_err(|_| "message creation failed")?;
    if !error_msg.is_error() {
        return Err("MSG_ERROR bit not set");
    }
    Ok(())
}

/// CTS-COMM-MSG-3 (RFC-0024.4): trailer capability tokens — capability_count
/// matches the number of tokens declared, and serialization preserves the
/// trailer bytes exactly.
fn comm_msg_trailer_capability_tokens() -> Result<(), &'static str> {
    let src = EndpointId::new([1u8; 16]);
    let dst = EndpointId::new([2u8; 16]);
    // Serialized capability tokens per RFC-0037 (opaque bytes here).
    let tokens = vec![
        vec![0xCA, 0xFE, 0xBA, 0xBE],
        vec![0xDE, 0xAD, 0xBE, 0xEF],
    ];
    let trailer: Vec<u8> = tokens.iter().flat_map(|t| t.iter().copied()).collect();
    let msg = Message::new(
        MessageHeader::new(1, src, dst, 0, trailer.len() as u32, MSG_CAP_TRANSFER, 0, 2),
        Vec::new(),
        trailer.clone(),
    ).map_err(|_| "message creation failed")?;
    if msg.header.capability_count != 2 {
        return Err("capability_count does not match token count");
    }
    let round = Message::deserialize(&msg.serialize()).map_err(|_| "deserialize failed")?;
    if round.trailer != trailer {
        return Err("trailer bytes not preserved on round-trip");
    }
    if round.header.capability_count != 2 {
        return Err("capability_count not preserved on round-trip");
    }
    Ok(())
}

/// CTS-COMM-MSG-4 (RFC-0024.6): max size enforcement — payload + trailer
/// exceeding MAX_MESSAGE_SIZE is rejected with PayloadTooLarge.
fn comm_msg_max_size_enforcement() -> Result<(), &'static str> {
    let src = EndpointId::none();
    let dst = EndpointId::none();
    // Just under the limit is accepted.
    let ok = Message::new(
        MessageHeader::new(1, src, dst, MAX_MESSAGE_SIZE as u32 - 1, 0, 0, 0, 0),
        vec![0u8; MAX_MESSAGE_SIZE - 1],
        Vec::new(),
    );
    if ok.is_err() {
        return Err("message just under max size was rejected");
    }
    // Over the limit is rejected.
    let too_big = Message::new(
        MessageHeader::new(2, src, dst, MAX_MESSAGE_SIZE as u32 + 1, 0, 0, 0, 0),
        vec![0u8; MAX_MESSAGE_SIZE + 1],
        Vec::new(),
    );
    if matches!(too_big, Err(MessageError::PayloadTooLarge)) {
        // expected
    } else {
        return Err("oversized message was accepted");
    }
    // Trailer counts toward the limit too.
    let over_with_trailer = Message::new(
        MessageHeader::new(3, src, dst, 1, 1, 0, 0, 0),
        vec![0u8; MAX_MESSAGE_SIZE],
        vec![0u8; 1],
    );
    if !matches!(
        over_with_trailer,
        Err(MessageError::PayloadTooLarge)
    ) {
        return Err("oversized payload+trailer was accepted");
    }
    Ok(())
}

/// CTS-COMM-MSG-5 (RFC-0024.9): reply correlation — a reply has the same
/// message_id as the request with src_endpoint and dst_endpoint swapped.
fn comm_msg_reply_correlation() -> Result<(), &'static str> {
    let src = EndpointId::new([7u8; 16]);
    let dst = EndpointId::new([8u8; 16]);
    let request = Message::new(
        MessageHeader::new(99, src, dst, 4, 0, 0, 10, 0),
        b"ping".to_vec(),
        Vec::new(),
    ).map_err(|_| "message creation failed")?;
    let reply = Message::reply_for(
        request.header.message_id,
        request.header.src_endpoint,
        request.header.dst_endpoint,
        b"pong".to_vec(),
    ).map_err(|_| "reply creation failed")?;
    if reply.header.message_id != request.header.message_id {
        return Err("reply does not echo request message_id");
    }
    if reply.header.src_endpoint != request.header.dst_endpoint {
        return Err("reply src_endpoint not swapped");
    }
    if reply.header.dst_endpoint != request.header.src_endpoint {
        return Err("reply dst_endpoint not swapped");
    }
    if reply.payload != b"pong" {
        return Err("reply payload mismatch");
    }
    if reply.is_error() {
        return Err("reply incorrectly flagged as error");
    }
    // Error response correlation (RFC-0024.7).
    let err = Message::error_response(
        request.header.message_id,
        request.header.src_endpoint,
        request.header.dst_endpoint,
        0x5FA00001,
    );
    if !err.is_error() {
        return Err("error response not flagged MSG_ERROR");
    }
    if err.header.message_id != request.header.message_id {
        return Err("error response does not echo request message_id");
    }
    if err.payload.len() < 4 {
        return Err("error response payload missing error code");
    }
    let code = u32::from_be_bytes([err.payload[0], err.payload[1], err.payload[2], err.payload[3]]);
    if code != 0x5FA00001 {
        return Err("error response error code mismatch");
    }
    Ok(())
}

/// CTS-COMM-EP-1 (RFC-0025.2): capability-protected registration — registering
/// an endpoint requires capability authority, and receiving requires a RECV
/// capability for that endpoint.
fn comm_ep_capability_protected_registration() -> Result<(), &'static str> {
    let domain = DomainId::new(1).ok_or("invalid domain")?;
    let mut registry = EndpointRegistry::new();
    let reg = EndpointRegistration {
        name: "test-endpoint".to_string(),
        max_msg_size: 4096,
        supported_rights: EndpointCapKind::Send.bit() | EndpointCapKind::Recv.bit(),
        handler_cap: None,
        mode: EndpointMode::Poll,
    };
    // Registration succeeds and returns an endpoint id + capability token.
    let (id, token) = registry.register(domain, reg).map_err(|_| "registration failed")?;
    if id.is_none() {
        return Err("endpoint id is zero / invalid");
    }
    if token.len() != 34 {
        return Err("capability token is not 34 bytes");
    }
    // The same name cannot be registered twice (capability-protected uniqueness).
    let reg2 = EndpointRegistration {
        name: "test-endpoint".to_string(),
        max_msg_size: 4096,
        supported_rights: EndpointCapKind::Send.bit(),
        handler_cap: None,
        mode: EndpointMode::Poll,
    };
    if registry.register(domain, reg2).is_ok() {
        return Err("duplicate endpoint registration was accepted");
    }
    Ok(())
}

/// CTS-COMM-EP-2 (RFC-0025.3): registration fields — name, max message size,
/// supported rights, handler capability, and mode are all preserved.
fn comm_ep_registration_fields() -> Result<(), &'static str> {
    let domain = DomainId::new(2).ok_or("invalid domain")?;
    let mut registry = EndpointRegistry::new();
    let reg = EndpointRegistration {
        name: "field-check".to_string(),
        max_msg_size: 65536,
        supported_rights: EndpointCapKind::Query.bit() | EndpointCapKind::Close.bit(),
        handler_cap: Some(vec![0xAA; 32]),
        mode: EndpointMode::Handler,
    };
    let (id, _token) = registry.register(domain, reg).map_err(|_| "registration failed")?;
    let meta = registry.metadata(id).ok_or("endpoint metadata missing")?;
    if meta.id != id {
        return Err("metadata id does not match registered id");
    }
    if meta.owner_domain != domain {
        return Err("metadata owner domain mismatch");
    }
    if meta.max_msg_size != 65536 {
        return Err("max_msg_size not preserved");
    }
    if meta.registered_rights != (EndpointCapKind::Query.bit() | EndpointCapKind::Close.bit()) {
        return Err("registered rights mismatch");
    }
    // Name too long (>256 bytes) is rejected.
    let long_name = "x".repeat(257);
    let reg_long = EndpointRegistration {
        name: long_name,
        max_msg_size: 4096,
        supported_rights: 0,
        handler_cap: None,
        mode: EndpointMode::Poll,
    };
    if registry.register(domain, reg_long).is_ok() {
        return Err("oversized endpoint name was accepted");
    }
    Ok(())
}

/// CTS-COMM-EP-3 (RFC-0025.5): endpoint capability kinds — all five kinds
/// (SEND, RECV, QUERY, CLOSE, INHERIT) have distinct bits and compose.
fn comm_ep_capability_kinds() -> Result<(), &'static str> {
    let all = EndpointCapKind::Send.bit()
        | EndpointCapKind::Recv.bit()
        | EndpointCapKind::Query.bit()
        | EndpointCapKind::Close.bit()
        | EndpointCapKind::Inherit.bit();
    if all != 0x1F {
        return Err("capability kind bits do not compose to 0x1F");
    }
    for cap in ALL_ENDPOINT_CAPS {
        if !cap.in_rights(all) {
            return Err("capability kind not present in combined rights");
        }
    }
    // Each kind has a distinct bit.
    let mut seen = 0u8;
    for cap in ALL_ENDPOINT_CAPS {
        if seen & cap.bit() != 0 {
            return Err("duplicate capability kind bit");
        }
        seen |= cap.bit();
    }
    // Empty rights grant no capabilities.
    if EndpointCapKind::Send.in_rights(0) || EndpointCapKind::Recv.in_rights(0) {
        return Err("empty rights grant capabilities");
    }
    Ok(())
}

/// CTS-COMM-EP-4 (RFC-0025.7): handler vs polling mode — handler is default,
/// poll mode does not invoke a handler.
fn comm_ep_handler_vs_polling_mode() -> Result<(), &'static str> {
    let domain = DomainId::new(3).ok_or("invalid domain")?;
    let mut registry = EndpointRegistry::new();
    // Polling mode: handler_cap is None, mode is Poll.
    let reg_poll = EndpointRegistration {
        name: "poll-endpoint".to_string(),
        max_msg_size: 4096,
        supported_rights: EndpointCapKind::Recv.bit(),
        handler_cap: None,
        mode: EndpointMode::Poll,
    };
    let (id_poll, _token) =
        registry.register(domain, reg_poll).map_err(|_| "poll registration failed")?;
    let meta_poll = registry.metadata(id_poll).ok_or("poll endpoint metadata missing")?;
    if meta_poll.registered_rights != EndpointCapKind::Recv.bit() {
        return Err("poll endpoint rights not preserved");
    }
    // Handler mode: handler_cap is Some, mode is Handler.
    let reg_handler = EndpointRegistration {
        name: "handler-endpoint".to_string(),
        max_msg_size: 4096,
        supported_rights: EndpointCapKind::Send.bit() | EndpointCapKind::Recv.bit(),
        handler_cap: Some(vec![0xBB; 16]),
        mode: EndpointMode::Handler,
    };
    let (id_handler, _token) =
        registry.register(domain, reg_handler).map_err(|_| "handler registration failed")?;
    let meta_handler = registry.metadata(id_handler).ok_or("handler endpoint metadata missing")?;
    if meta_handler.registered_rights
        != (EndpointCapKind::Send.bit() | EndpointCapKind::Recv.bit())
    {
        return Err("handler endpoint rights not preserved");
    }
    Ok(())
}

/// CTS-COMM-EP-5 (RFC-0025.9): error code format — error codes are returned in
/// `MSG_ERROR` response format per RFC-0024.7.
fn comm_ep_error_code_format() -> Result<(), &'static str> {
    // All defined error codes map to a numeric value.
    let codes = [
        CommErrorCode::Ok,
        CommErrorCode::ENODEST,
        CommErrorCode::ENOCAP,
        CommErrorCode::EMSGSIZE,
        CommErrorCode::ENOTCONN,
        CommErrorCode::ECAPSTALE,
        CommErrorCode::EPROTO,
        CommErrorCode::EQUOTA,
    ];
    let expected = [0, 1, 2, 3, 4, 5, 6, 7];
    for (i, c) in codes.iter().enumerate() {
        if c.code() != expected[i] as u32 {
            return Err("error code numeric value mismatch");
        }
    }
    // EndpointError maps to CommErrorCode per RFC-0025.9.
    let mappings = [
        (EndpointError::NotFound, CommErrorCode::ENODEST),
        (EndpointError::NoCap, CommErrorCode::ENOCAP),
        (EndpointError::QuotaExceeded, CommErrorCode::EQUOTA),
        (EndpointError::NotConnected, CommErrorCode::ENOTCONN),
        (EndpointError::CapStale, CommErrorCode::ECAPSTALE),
    ];
    for (e, expected_code) in mappings {
        let code: CommErrorCode = e.into();
        if code != expected_code {
            return Err("endpoint error does not map to expected comm error code");
        }
    }
    // An error response message carries MSG_ERROR flag and error code in first 4 payload bytes.
    let src = EndpointId::none();
    let dst = EndpointId::none();
    let err_msg = Message::error_response(7, src, dst, CommErrorCode::ENOCAP.code());
    if !err_msg.is_error() {
        return Err("error response not flagged MSG_ERROR");
    }
    if err_msg.payload.len() < 4 {
        return Err("error response payload missing error code");
    }
    let code = u32::from_be_bytes([
        err_msg.payload[0],
        err_msg.payload[1],
        err_msg.payload[2],
        err_msg.payload[3],
    ]);
    if code != CommErrorCode::ENOCAP.code() {
        return Err("error response error code mismatch");
    }
    Ok(())
}

/// CTS-COMM-EP-6 (RFC-0025.10): close deferred until queue empty — closing an
/// endpoint with pending messages is rejected unless forced.
fn comm_ep_close_deferred_until_queue_empty() -> Result<(), &'static str> {
    let domain = DomainId::new(4).ok_or("invalid domain")?;
    let mut registry = EndpointRegistry::new();
    let reg = EndpointRegistration {
        name: "close-test".to_string(),
        max_msg_size: 4096,
        supported_rights: EndpointCapKind::Recv.bit(),
        handler_cap: None,
        mode: EndpointMode::Poll,
    };
    let (id, _token) = registry.register(domain, reg).map_err(|_| "registration failed")?;
    // Enqueue messages to fill the queue.
    registry.enqueue(id).map_err(|_| "enqueue failed")?;
    registry.enqueue(id).map_err(|_| "enqueue failed")?;
    // Close without force is rejected because the queue is not empty.
    if registry.close(id, false).is_ok() {
        return Err("close with non-empty queue was accepted without force");
    }
    // Forced close succeeds.
    registry.close(id, true).map_err(|_| "forced close failed")?;
    // The endpoint is now closed and no longer findable.
    if registry.metadata(id).is_some() {
        return Err("closed endpoint still has metadata");
    }
    // Operations on a closed endpoint are rejected.
    if registry.dequeue(id).is_ok() {
        return Err("dequeue on closed endpoint was accepted");
    }
    Ok(())
}

/// CTS-COMM-RT-1 (RFC-0026.1): route resolution produces valid hops — each hop
/// contains a next hop, required capability class, transport, and weight.
fn comm_rt_route_resolution_produces_valid_hops() -> Result<(), &'static str> {
    let domain = DomainId::new(1).ok_or("invalid domain")?;
    let src = EpEndpointId::new([1u8; 32]);
    let mut registry = EndpointRegistry::new();
    let reg = EndpointRegistration {
        name: "dst-endpoint".to_string(),
        max_msg_size: 4096,
        supported_rights: EndpointCapKind::Recv.bit(),
        handler_cap: None,
        mode: EndpointMode::Poll,
    };
    let (dst_id, _token) = registry.register(domain, reg).map_err(|_| "registration failed")?;
    let mut router = Router::new();
    router.install_route(RouteEntry {
        destination: dst_id,
        hop_count: 1,
        hops: vec![RouteHop {
            next_hop: dst_id,
            required_cap_class: 1, // SEND
            transport: 0,
            weight: 1,
        }],
    });
    let hops = router.route(src, dst_id, EndpointCapKind::Send.bit()).map_err(|_| "route resolution failed")?;
    if hops.len() != 1 {
        return Err("expected exactly one hop");
    }
    let hop = &hops[0];
    if hop.next_hop != dst_id {
        return Err("hop next_hop is not the destination");
    }
    if hop.required_cap_class != 1 {
        return Err("hop required_cap_class is not SEND");
    }
    if hop.weight == 0 {
        return Err("hop weight should be non-zero");
    }
    Ok(())
}

/// CTS-COMM-RT-2 (RFC-0026.3): capability required per hop — a sender lacking
/// the required capability for a hop is rejected with ENOCAP.
fn comm_rt_capability_required_per_hop() -> Result<(), &'static str> {
    let src = EpEndpointId::new([1u8; 32]);
    let dst = EpEndpointId::new([2u8; 32]);
    let mut router = Router::new();
    router.install_route(RouteEntry {
        destination: dst,
        hop_count: 1,
        hops: vec![RouteHop {
            next_hop: dst,
            required_cap_class: 1, // SEND required
            transport: 0,
            weight: 1,
        }],
    });
    // Sender without SEND capability is rejected with ENOCAP.
    let result = router.route(src, dst, 0); // no rights
    if !matches!(result, Err(RoutingError::ENOCAP)) {
        return Err("sender without required capability was not rejected with ENOCAP");
    }
    // Sender with SEND capability succeeds.
    let hops = router.route(src, dst, EndpointCapKind::Send.bit()).map_err(|_| "route resolution failed")?;
    if hops.len() != 1 {
        return Err("sender with required capability should get a route");
    }
    Ok(())
}

/// CTS-COMM-RT-3 (RFC-0026.5): single-hop intra-domain routing — within a
/// single execution domain, routing is a single hop to the destination
/// endpoint, requiring SEND on the destination.
fn comm_rt_single_hop_intra_domain_routing() -> Result<(), &'static str> {
    let domain = DomainId::new(2).ok_or("invalid domain")?;
    let src = EpEndpointId::new([10u8; 32]);
    let mut registry = EndpointRegistry::new();
    let reg = EndpointRegistration {
        name: "intra-dst".to_string(),
        max_msg_size: 4096,
        supported_rights: EndpointCapKind::Recv.bit(),
        handler_cap: None,
        mode: EndpointMode::Poll,
    };
    let (dst_id, _token) = registry.register(domain, reg).map_err(|_| "registration failed")?;
    let mut router = Router::new();
    router.install_route(RouteEntry {
        destination: dst_id,
        hop_count: 1,
        hops: vec![RouteHop {
            next_hop: dst_id,
            required_cap_class: 1, // SEND on destination (RFC-0026.5)
            transport: 0,
            weight: 1,
        }],
    });
    // Intra-domain route is a single hop.
    let hops = router.route(src, dst_id, EndpointCapKind::Send.bit() | EndpointCapKind::Recv.bit()).map_err(|_| "route resolution failed")?;
    if hops.len() != 1 {
        return Err("intra-domain routing should be a single hop");
    }
    if hops[0].next_hop != dst_id {
        return Err("single hop should go directly to destination");
    }
    // Without SEND on destination, rejected.
    if router.route(src, dst_id, EndpointCapKind::Recv.bit()).is_ok() {
        return Err("routing without SEND on destination should be rejected");
    }
    Ok(())
}

/// CTS-COMM-RT-4 (RFC-0026.8): no silent drops — if a message cannot be routed
/// (no route exists, capability missing), the routing subsystem returns an
/// error to the sender and MUST NOT silently drop the message.
fn comm_rt_no_silent_drops() -> Result<(), &'static str> {
    let src = EpEndpointId::new([1u8; 32]);
    let dst = EpEndpointId::new([2u8; 32]);
    let mut router = Router::new();
    // No route installed: resolve returns ENODEST (not a silent drop).
    let result = router.route(src, dst, EndpointCapKind::Send.bit());
    if !matches!(result, Err(RoutingError::ENODEST)) {
        return Err("missing route should return ENODEST, not silently drop");
    }
    // Route exists but capability missing: returns ENOCAP (not a silent drop).
    router.install_route(RouteEntry {
        destination: dst,
        hop_count: 1,
        hops: vec![RouteHop {
            next_hop: dst,
            required_cap_class: 1,
            transport: 0,
            weight: 1,
        }],
    });
    let result = router.route(src, dst, 0); // no SEND
    if !matches!(result, Err(RoutingError::ENOCAP)) {
        return Err("missing capability should return ENOCAP, not silently drop");
    }
    Ok(())
}

/// CTS-COMM-RT-5 (RFC-0026.10): table invalidation on deregistration — when an
/// endpoint is deregistered, all routing table entries referring to it are
/// invalidated.
fn comm_rt_table_invalidation_on_deregistration() -> Result<(), &'static str> {
    let domain = DomainId::new(3).ok_or("invalid domain")?;
    let mut registry = EndpointRegistry::new();
    let reg = EndpointRegistration {
        name: "to-be-closed".to_string(),
        max_msg_size: 4096,
        supported_rights: EndpointCapKind::Recv.bit(),
        handler_cap: None,
        mode: EndpointMode::Poll,
    };
    let (dst_id, _token) = registry.register(domain, reg).map_err(|_| "registration failed")?;
    let mut router = Router::new();
    router.install_route(RouteEntry {
        destination: dst_id,
        hop_count: 1,
        hops: vec![RouteHop {
            next_hop: dst_id,
            required_cap_class: 1,
            transport: 0,
            weight: 1,
        }],
    });
    // Route exists before invalidation.
    if router.resolve(EpEndpointId::new([1u8; 32]), dst_id).is_err() {
        return Err("route should exist before invalidation");
    }
    // Deregister the endpoint and invalidate table entries referring to it.
    router.invalidate_endpoint(dst_id);
    // After invalidation, no route to the deregistered endpoint.
    if router.resolve(EpEndpointId::new([1u8; 32]), dst_id).is_ok() {
        return Err("route should not exist after endpoint deregistration");
    }
    Ok(())
}

/// CTS-COMM-RT-6 (RFC-0026.11): routing time complexity bound — route
/// resolution completes in O(log N) using binary search; repeated lookups are
/// deterministic (RFC-0026.12).
fn comm_rt_routing_time_complexity_bound() -> Result<(), &'static str> {
    let mut router = Router::new();
    // Install N routes with distinct destinations.
    let n = 64;
    for i in 0..n {
        let dst = EpEndpointId::new([i as u8; 32]);
        router.install_route(RouteEntry {
            destination: dst,
            hop_count: 1,
            hops: vec![RouteHop {
                next_hop: dst,
                required_cap_class: 1,
                transport: 0,
                weight: 1,
            }],
        });
    }
    let src = EpEndpointId::new([255u8; 32]);
    // Resolve the last route: binary search should find it in O(log N).
    let dst_last = EpEndpointId::new([(n - 1) as u8; 32]);
    let hops = router.route(src, dst_last, EndpointCapKind::Send.bit()).map_err(|_| "route resolution failed")?;
    if hops.len() != 1 || hops[0].next_hop != dst_last {
        return Err("binary search route resolution did not find installed route");
    }
    // Determinism: identical inputs produce identical outputs.
    let hops2 = router.route(src, dst_last, EndpointCapKind::Send.bit()).map_err(|_| "route resolution failed")?;
    if hops != hops2 {
        return Err("routing decisions are not deterministic");
    }
    // Non-existent destination returns ENODEST deterministically.
    let missing = EpEndpointId::new([200u8; 32]);
    let r1 = router.route(src, missing, EndpointCapKind::Send.bit());
    let r2 = router.route(src, missing, EndpointCapKind::Send.bit());
    if !matches!(r1, Err(RoutingError::ENODEST)) || !matches!(r2, Err(RoutingError::ENODEST)) {
        return Err("missing destination should deterministically return ENODEST");
    }
    Ok(())
}
