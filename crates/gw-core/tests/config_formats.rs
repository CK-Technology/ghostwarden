use gw_core::{Network, Plan, Topology};

#[test]
fn parses_toml_topology_example() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let topology = Topology::from_file(&repo_root.join("examples/ghostnet.toml")).unwrap();

    assert_eq!(topology.version, 1);
    assert_eq!(topology.interfaces.get("uplink").unwrap(), "enp6s0");
    assert!(matches!(
        topology.networks.get("nat_dev"),
        Some(Network::Routed(_))
    ));
    assert!(matches!(
        topology.networks.get("br_work"),
        Some(Network::Bridge(_))
    ));

    let plan = Plan::from_topology(&topology).unwrap();
    assert!(!plan.actions.is_empty());
}

#[test]
fn keeps_yaml_topology_compatibility() {
    let yaml = r#"
version: 1
interfaces:
  uplink: enp6s0
networks:
  nat_dev:
    type: routed
    cidr: 10.33.0.0/24
    gw_ip: 10.33.0.1
    dhcp: true
    masq_out: enp6s0
"#;

    let topology = Topology::from_yaml(yaml).unwrap();
    assert!(matches!(
        topology.networks.get("nat_dev"),
        Some(Network::Routed(_))
    ));
}
