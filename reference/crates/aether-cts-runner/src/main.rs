//! Minimal machine-readable capability conformance runner.

use aether_cap_model::{BootstrapAuthority as ModelBootstrap, CapabilityModel};
use aether_capspace::{BootstrapAuthority as SystemBootstrap, CapabilitySystem};
use aether_state_model::{
    EffectClass, EffectIntent, EffectStatus, Mutation, OperationId, StateEngine, VersionId,
};
use aether_types::{Bounds, CapError, Capability, DomainId, Handle, ObjectId, ObjectType, Rights};

fn main() {
    let cases = [
        ("CTS-CAP-NARROW-001", narrow_rights_and_bounds()),
        ("CTS-CAP-XFER-001", explicit_transfer()),
        ("CTS-CAP-REVOKE-001", revoke_across_transfer()),
        ("CTS-CAP-FORGE-001", reject_forged_integer()),
        ("CTS-CAP-STALE-001", reject_stale_generation()),
        ("CTS-CAP-BOOTSTRAP-001", reject_foreign_bootstrap()),
        ("CTS-CAP-DOMAIN-001", reject_unregistered_destination()),
        ("CTS-CAP-REVOKED-DERIVE-001", reject_revoked_parent_use()),
        ("CTS-STATE-ATOMIC-001", state_commit_is_failure_atomic()),
        (
            "CTS-EFFECT-UNCERTAIN-001",
            uncertain_effect_is_recoverable(),
        ),
        ("CTS-STATE-REPLAY-001", reject_replayed_transaction()),
        ("CTS-STATE-DUPLICATE-001", reject_duplicate_mutation()),
    ];
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
    let model_result = model.transfer(domain(1), model_root, domain(2), Rights::READ, bounds(0, 1));
    let system_result = system.transfer(
        domain(1),
        system_root,
        domain(2),
        Rights::READ,
        bounds(0, 1),
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
        model.derive(domain(1), model_root, Rights::READ, bounds(0, 1)),
        system.derive(domain(1), system_root, Rights::READ, bounds(0, 1)),
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
    let m = model.derive(domain(1), model_root, Rights::READ, bounds(64, 128));
    let s = system.derive(domain(1), system_root, Rights::READ, bounds(64, 128));
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
        .transfer(domain(1), mr, domain(2), Rights::READ, bounds(100, 200))
        .map_err(|_| "model transfer failed")?;
    let sh = system
        .transfer(domain(1), sr, domain(2), Rights::READ, bounds(100, 200))
        .map_err(|_| "provider transfer failed")?;
    compare_auth(
        model.authorize(
            domain(2),
            mh,
            ObjectType::Memory,
            Rights::READ,
            bounds(120, 130),
        ),
        system.authorize(
            domain(2),
            sh,
            ObjectType::Memory,
            Rights::READ,
            bounds(120, 130),
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
        .transfer(domain(1), mr, domain(2), Rights::READ, bounds(0, 10))
        .map_err(|_| "model transfer failed")?;
    let sh = system
        .transfer(domain(1), sr, domain(2), Rights::READ, bounds(0, 10))
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
        ),
        system.authorize(
            domain(2),
            sh,
            ObjectType::Memory,
            Rights::READ,
            bounds(0, 1),
        ),
    )
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
        ),
        system.authorize(
            domain(1),
            Handle::from_untrusted(0xdead_beef),
            ObjectType::Memory,
            Rights::READ,
            bounds(0, 1),
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
