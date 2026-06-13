//! Policy profile parser fixtures (TOML and YAML).
//!
//! Pure parsing checks; no root required. Complements `config_formats.rs`, which
//! covers topology parsing, by guarding the policy-profile side of the loader.

use std::path::Path;

use gw_core::{PolicyAction, PolicyProfile, ProfileLoader, Protocol};

#[test]
fn loads_all_toml_policy_examples() {
    let loader = ProfileLoader::new();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/policies");

    for name in ["l2-lan", "public-web", "routed-tight"] {
        let profile = loader
            .load_profile(&dir.join(format!("{name}.toml")))
            .unwrap_or_else(|e| panic!("load {name}.toml: {e}"));

        assert_eq!(profile.name, name);
        assert!(matches!(profile.default_action, PolicyAction::Drop));
        assert!(!profile.services.is_empty(), "{name} should list services");
    }
}

#[test]
fn loads_directory_of_toml_profiles() {
    let loader = ProfileLoader::new();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/policies");

    let profiles = loader
        .load_profiles_from_dir(&dir)
        .expect("load policies dir");

    assert!(profiles.contains_key("l2-lan"));
    assert!(profiles.contains_key("public-web"));
    assert!(profiles.contains_key("routed-tight"));
}

#[test]
fn policy_profile_yaml_compatibility() {
    // YAML profiles must keep parsing after the TOML migration.
    let yaml = r#"
name: yaml-compat
description: legacy yaml policy
default_action: drop
allowed_egress_cidrs:
  - 0.0.0.0/0
services:
  - protocol: tcp
    port: 443
  - protocol: udp
    port: 53
"#;

    let profile: PolicyProfile =
        gw_core::config_format::from_str(yaml, gw_core::config_format::ConfigFormat::Yaml)
            .expect("parse yaml policy");

    assert_eq!(profile.name, "yaml-compat");
    assert_eq!(profile.services.len(), 2);
    assert!(matches!(profile.services[0].protocol, Protocol::Tcp));
    assert_eq!(profile.services[1].port, 53);
}
