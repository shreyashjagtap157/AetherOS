//! Minimal machine-readable capability conformance runner.

use aether_cap_model::{BootstrapAuthority as ModelBootstrap, CapabilityModel};
use aether_capspace::{BootstrapAuthority as SystemBootstrap, CapabilitySystem};
use aether_types::{Bounds, CapError, Capability, DomainId, Handle, ObjectId, ObjectType, Rights};

fn main() {
    let cases = [
        ("CTS-CAP-NARROW-001", narrow_rights_and_bounds()),
        ("CTS-CAP-XFER-001", explicit_transfer()),
        ("CTS-CAP-REVOKE-001", revoke_across_transfer()),
        ("CTS-CAP-FORGE-001", reject_forged_integer()),
        ("CTS-CAP-STALE-001", reject_stale_generation()),
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
    let (model, model_bootstrap) = CapabilityModel::bootstrap();
    let (mut system, system_bootstrap) = CapabilitySystem::bootstrap();
    system.add_domain(domain(1));
    system.add_domain(domain(2));
    (model, model_bootstrap, system, system_bootstrap)
}

fn narrow_rights_and_bounds() -> Result<(), &'static str> {
    let (mut model, model_bootstrap, mut system, system_bootstrap) = pair();
    let model_root = model.create_root(&model_bootstrap, domain(1), root());
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
    let mr = model.create_root(&model_bootstrap, domain(1), root());
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
    let mr = model.create_root(&model_bootstrap, domain(1), root());
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
