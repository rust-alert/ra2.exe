//! `validate_session_spec` 负例与通过路径。

use std::sync::Arc;

use ra_engine::{Engine, EngineConfig, SessionSpec, SessionValidationError};
use ra_types::{BuiltinCapability, CapabilitySet, RuntimeDefinitions};

fn engine_with_caps(builtins: Vec<BuiltinCapability>) -> Engine {
    let mut defs = RuntimeDefinitions::default();
    defs.capabilities = CapabilitySet { builtins };
    Engine::new(Arc::new(defs), EngineConfig::default()).expect("引擎应可构造")
}

#[test]
fn validate_rejects_oversized_label() {
    let engine = engine_with_caps(Vec::new());
    let spec = SessionSpec { label: "x".repeat(257), required_capabilities: Vec::new() };
    let err = engine.validate_session_spec(&spec).unwrap_err();
    assert!(matches!(err, SessionValidationError { .. }));
    assert!(err.message.contains("过长"));
}

#[test]
fn validate_rejects_missing_required_capability() {
    let engine = engine_with_caps(vec![BuiltinCapability::Mobile]);
    let spec = SessionSpec { label: "ok".into(), required_capabilities: vec![BuiltinCapability::Weapon] };
    let err = engine.validate_session_spec(&spec).unwrap_err();
    assert!(err.message.contains("Weapon"));
    assert!(engine.create_session(spec).is_err());
}

#[test]
fn validate_accepts_declared_capabilities() {
    let engine = engine_with_caps(vec![BuiltinCapability::Mobile, BuiltinCapability::Weapon]);
    let spec = SessionSpec { label: "skirmish".into(), required_capabilities: vec![BuiltinCapability::Mobile] };
    assert!(engine.validate_session_spec(&spec).is_ok());
    assert!(engine.create_session(spec).is_ok());
}
