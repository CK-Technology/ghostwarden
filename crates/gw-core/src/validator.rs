use anyhow::{Context, Result};
use std::collections::HashSet;
use std::net::IpAddr;

use crate::topology::{Network, Topology};

/// Validates a topology for correctness and safety
pub struct TopologyValidator<'a> {
    topology: &'a Topology,
}

impl<'a> TopologyValidator<'a> {
    pub fn new(topology: &'a Topology) -> Self {
        Self { topology }
    }

    /// Run all validations
    pub fn validate(&self) -> Result<Vec<ValidationWarning>> {
        let mut warnings = Vec::new();

        // Check for CIDR overlaps
        warnings.extend(self.check_cidr_overlaps()?);

        // Validate port ranges
        warnings.extend(self.validate_port_ranges()?);

        // Validate IP addresses
        warnings.extend(self.validate_ip_addresses()?);

        // Check for conflicts
        warnings.extend(self.check_naming_conflicts()?);

        // Validate network references
        warnings.extend(self.validate_network_references()?);

        Ok(warnings)
    }

    /// Check for CIDR overlaps between networks
    fn check_cidr_overlaps(&self) -> Result<Vec<ValidationWarning>> {
        let mut warnings = Vec::new();
        let mut cidrs: Vec<(&str, &str)> = Vec::new();

        // Collect all CIDRs
        for (name, network) in &self.topology.networks {
            if let Network::Routed(routed) = network {
                cidrs.push((name.as_str(), routed.cidr.as_str()));
            }
        }

        // Check for overlaps
        for i in 0..cidrs.len() {
            for j in (i + 1)..cidrs.len() {
                if Self::cidrs_overlap(cidrs[i].1, cidrs[j].1)? {
                    warnings.push(ValidationWarning::CidrOverlap {
                        net1: cidrs[i].0.to_string(),
                        cidr1: cidrs[i].1.to_string(),
                        net2: cidrs[j].0.to_string(),
                        cidr2: cidrs[j].1.to_string(),
                    });
                }
            }
        }

        Ok(warnings)
    }

    /// Check if two CIDRs overlap
    fn cidrs_overlap(cidr1: &str, cidr2: &str) -> Result<bool> {
        use std::net::Ipv4Addr;

        // Parse CIDR1
        let parts1: Vec<&str> = cidr1.split('/').collect();
        if parts1.len() != 2 {
            anyhow::bail!("Invalid CIDR format: {}", cidr1);
        }
        let ip1: Ipv4Addr = parts1[0]
            .parse()
            .context(format!("Invalid IP in CIDR: {}", cidr1))?;
        let prefix1: u8 = parts1[1]
            .parse()
            .context(format!("Invalid prefix in CIDR: {}", cidr1))?;

        // Parse CIDR2
        let parts2: Vec<&str> = cidr2.split('/').collect();
        if parts2.len() != 2 {
            anyhow::bail!("Invalid CIDR format: {}", cidr2);
        }
        let ip2: Ipv4Addr = parts2[0]
            .parse()
            .context(format!("Invalid IP in CIDR: {}", cidr2))?;
        let prefix2: u8 = parts2[1]
            .parse()
            .context(format!("Invalid prefix in CIDR: {}", cidr2))?;

        // Calculate network addresses
        let mask1 = (!0u32) << (32 - prefix1);
        let mask2 = (!0u32) << (32 - prefix2);

        let net1 = u32::from(ip1) & mask1;
        let net2 = u32::from(ip2) & mask2;

        // Check if one network contains the other
        let smaller_prefix = prefix1.min(prefix2);
        let smaller_mask = (!0u32) << (32 - smaller_prefix);

        Ok((net1 & smaller_mask) == (net2 & smaller_mask))
    }

    /// Validate port ranges in port forwards
    fn validate_port_ranges(&self) -> Result<Vec<ValidationWarning>> {
        let mut warnings = Vec::new();

        for (net_name, network) in &self.topology.networks {
            if let Network::Routed(routed) = network {
                for forward in &routed.forwards {
                    // Validate public port
                    if let Err(e) = Self::validate_port_spec(&forward.public) {
                        warnings.push(ValidationWarning::InvalidPort {
                            network: net_name.clone(),
                            port_spec: forward.public.clone(),
                            reason: e.to_string(),
                        });
                    }

                    // Validate destination port
                    if let Err(e) = Self::validate_destination(&forward.dst) {
                        warnings.push(ValidationWarning::InvalidDestination {
                            network: net_name.clone(),
                            dst_spec: forward.dst.clone(),
                            reason: e.to_string(),
                        });
                    }
                }
            }
        }

        Ok(warnings)
    }

    /// Validate a port spec (e.g., ":4022/tcp", "0.0.0.0:8080/udp")
    fn validate_port_spec(spec: &str) -> Result<()> {
        // Remove protocol suffix
        let without_proto = spec.split('/').next().unwrap_or(spec);

        // Parse host:port
        let parts: Vec<&str> = without_proto.rsplit(':').collect();
        if parts.is_empty() {
            anyhow::bail!("Invalid port spec: {}", spec);
        }

        let port_str = parts[0];
        let port: u16 = port_str.parse().context("Invalid port number")?;

        if port == 0 {
            anyhow::bail!("Port 0 is not allowed");
        }

        // Validate protocol
        if spec.contains('/') {
            let proto = spec.split('/').nth(1).unwrap_or("");
            if proto != "tcp" && proto != "udp" && proto != "sctp" {
                anyhow::bail!("Invalid protocol: {}", proto);
            }
        }

        Ok(())
    }

    /// Validate a destination spec (e.g., "10.33.0.10:22")
    fn validate_destination(spec: &str) -> Result<()> {
        let parts: Vec<&str> = spec.rsplit(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Destination must be in format IP:PORT");
        }

        let port_str = parts[0];
        let ip_str = parts[1];

        // Validate IP
        let _: IpAddr = ip_str.parse().context("Invalid IP address")?;

        // Validate port
        let port: u16 = port_str.parse().context("Invalid port number")?;
        if port == 0 {
            anyhow::bail!("Port 0 is not allowed");
        }

        Ok(())
    }

    /// Validate IP addresses and CIDR notations
    fn validate_ip_addresses(&self) -> Result<Vec<ValidationWarning>> {
        let mut warnings = Vec::new();

        for (net_name, network) in &self.topology.networks {
            match network {
                Network::Routed(routed) => {
                    // Validate CIDR
                    if let Err(e) = Self::validate_cidr(&routed.cidr) {
                        warnings.push(ValidationWarning::InvalidCidr {
                            network: net_name.clone(),
                            cidr: routed.cidr.clone(),
                            reason: e.to_string(),
                        });
                    }

                    // Validate gateway IP is in CIDR range
                    if let Err(e) =
                        Self::validate_gateway_in_cidr(&routed.gw_ip.to_string(), &routed.cidr)
                    {
                        warnings.push(ValidationWarning::GatewayNotInCidr {
                            network: net_name.clone(),
                            gateway: routed.gw_ip.to_string(),
                            cidr: routed.cidr.clone(),
                            reason: e.to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(warnings)
    }

    /// Validate CIDR notation
    fn validate_cidr(cidr: &str) -> Result<()> {
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            anyhow::bail!("CIDR must be in format IP/PREFIX");
        }

        let ip_str = parts[0];
        let prefix_str = parts[1];

        // Validate IP
        let ip: IpAddr = ip_str.parse().context("Invalid IP address")?;

        // Validate prefix length
        let prefix: u8 = prefix_str.parse().context("Invalid prefix length")?;

        match ip {
            IpAddr::V4(_) => {
                if prefix > 32 {
                    anyhow::bail!("IPv4 prefix must be 0-32");
                }
            }
            IpAddr::V6(_) => {
                if prefix > 128 {
                    anyhow::bail!("IPv6 prefix must be 0-128");
                }
            }
        }

        Ok(())
    }

    /// Validate that gateway IP is within CIDR range
    fn validate_gateway_in_cidr(gateway: &str, cidr: &str) -> Result<()> {
        use std::net::Ipv4Addr;

        let gw_ip: Ipv4Addr = gateway.parse().context("Invalid gateway IP")?;

        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid CIDR");
        }

        let net_ip: Ipv4Addr = parts[0].parse().context("Invalid network IP")?;
        let prefix: u8 = parts[1].parse().context("Invalid prefix")?;

        let mask = (!0u32) << (32 - prefix);
        let net_addr = u32::from(net_ip) & mask;
        let gw_addr = u32::from(gw_ip) & mask;

        if net_addr != gw_addr {
            anyhow::bail!("Gateway {} is not in network {}", gateway, cidr);
        }

        Ok(())
    }

    /// Check for naming conflicts (duplicate network names, bridge names, etc.)
    fn check_naming_conflicts(&self) -> Result<Vec<ValidationWarning>> {
        let mut warnings = Vec::new();
        let mut iface_names = HashSet::new();

        for (net_name, network) in &self.topology.networks {
            if let Network::Bridge(bridge) = network {
                if !iface_names.insert(bridge.iface.clone()) {
                    warnings.push(ValidationWarning::DuplicateInterfaceName {
                        name: bridge.iface.clone(),
                        networks: vec![net_name.clone()], // Could track all conflicts
                    });
                }
            }
        }

        Ok(warnings)
    }

    /// Validate network references (e.g., uplink interfaces exist)
    fn validate_network_references(&self) -> Result<Vec<ValidationWarning>> {
        let warnings: Vec<ValidationWarning> = Vec::new();

        // Check if uplink interface is specified in routed networks
        // This is optional for now

        Ok(warnings)
    }
}

/// Validation warnings that don't prevent apply but should be shown
#[derive(Debug, Clone)]
pub enum ValidationWarning {
    CidrOverlap {
        net1: String,
        cidr1: String,
        net2: String,
        cidr2: String,
    },
    InvalidPort {
        network: String,
        port_spec: String,
        reason: String,
    },
    InvalidDestination {
        network: String,
        dst_spec: String,
        reason: String,
    },
    InvalidCidr {
        network: String,
        cidr: String,
        reason: String,
    },
    GatewayNotInCidr {
        network: String,
        gateway: String,
        cidr: String,
        reason: String,
    },
    DuplicateInterfaceName {
        name: String,
        networks: Vec<String>,
    },
}

impl ValidationWarning {
    pub fn display(&self) {
        match self {
            Self::CidrOverlap {
                net1,
                cidr1,
                net2,
                cidr2,
            } => {
                println!("⚠️  CIDR overlap detected:");
                println!("   {} ({}) overlaps with {} ({})", net1, cidr1, net2, cidr2);
            }
            Self::InvalidPort {
                network,
                port_spec,
                reason,
            } => {
                println!("⚠️  Invalid port in network '{}':", network);
                println!("   Port spec: {}", port_spec);
                println!("   Reason: {}", reason);
            }
            Self::InvalidDestination {
                network,
                dst_spec,
                reason,
            } => {
                println!("⚠️  Invalid destination in network '{}':", network);
                println!("   Destination: {}", dst_spec);
                println!("   Reason: {}", reason);
            }
            Self::InvalidCidr {
                network,
                cidr,
                reason,
            } => {
                println!("⚠️  Invalid CIDR in network '{}':", network);
                println!("   CIDR: {}", cidr);
                println!("   Reason: {}", reason);
            }
            Self::GatewayNotInCidr {
                network,
                gateway,
                cidr,
                reason,
            } => {
                println!("⚠️  Gateway not in CIDR range for network '{}':", network);
                println!("   Gateway: {}", gateway);
                println!("   CIDR: {}", cidr);
                println!("   Reason: {}", reason);
            }
            Self::DuplicateInterfaceName { name, networks } => {
                println!("⚠️  Duplicate interface name: {}", name);
                println!("   Used by networks: {}", networks.join(", "));
            }
        }
    }

    pub fn is_error(&self) -> bool {
        match self {
            Self::InvalidPort { .. }
            | Self::InvalidDestination { .. }
            | Self::InvalidCidr { .. }
            | Self::GatewayNotInCidr { .. } => true,
            Self::CidrOverlap { .. } | Self::DuplicateInterfaceName { .. } => false, // Warnings only
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cidr_overlap_detection() {
        // Same network
        assert!(TopologyValidator::cidrs_overlap("10.0.0.0/24", "10.0.0.0/24").unwrap());

        // Overlapping networks
        assert!(TopologyValidator::cidrs_overlap("10.0.0.0/24", "10.0.0.0/16").unwrap());
        assert!(TopologyValidator::cidrs_overlap("192.168.1.0/24", "192.168.0.0/16").unwrap());

        // Non-overlapping networks
        assert!(!TopologyValidator::cidrs_overlap("10.0.0.0/24", "10.1.0.0/24").unwrap());
        assert!(!TopologyValidator::cidrs_overlap("192.168.1.0/24", "172.16.0.0/16").unwrap());
    }

    #[test]
    fn test_port_validation() {
        assert!(TopologyValidator::validate_port_spec(":22/tcp").is_ok());
        assert!(TopologyValidator::validate_port_spec("0.0.0.0:8080/udp").is_ok());
        assert!(TopologyValidator::validate_port_spec(":65535/tcp").is_ok());

        assert!(TopologyValidator::validate_port_spec(":0/tcp").is_err());
        assert!(TopologyValidator::validate_port_spec(":99999/tcp").is_err());
        assert!(TopologyValidator::validate_port_spec(":22/invalid").is_err());
    }

    #[test]
    fn test_destination_validation() {
        assert!(TopologyValidator::validate_destination("10.0.0.1:22").is_ok());
        assert!(TopologyValidator::validate_destination("192.168.1.100:8080").is_ok());

        assert!(TopologyValidator::validate_destination("10.0.0.1").is_err());
        assert!(TopologyValidator::validate_destination("invalid:22").is_err());
        assert!(TopologyValidator::validate_destination("10.0.0.1:0").is_err());
    }

    #[test]
    fn test_cidr_validation() {
        assert!(TopologyValidator::validate_cidr("10.0.0.0/24").is_ok());
        assert!(TopologyValidator::validate_cidr("192.168.1.0/16").is_ok());
        assert!(TopologyValidator::validate_cidr("172.16.0.0/12").is_ok());

        assert!(TopologyValidator::validate_cidr("10.0.0.0").is_err());
        assert!(TopologyValidator::validate_cidr("10.0.0.0/33").is_err());
        assert!(TopologyValidator::validate_cidr("invalid/24").is_err());
    }

    #[test]
    fn test_gateway_in_cidr() {
        assert!(TopologyValidator::validate_gateway_in_cidr("10.0.0.1", "10.0.0.0/24").is_ok());
        assert!(
            TopologyValidator::validate_gateway_in_cidr("192.168.1.1", "192.168.1.0/24").is_ok()
        );

        assert!(TopologyValidator::validate_gateway_in_cidr("10.0.1.1", "10.0.0.0/24").is_err());
        assert!(
            TopologyValidator::validate_gateway_in_cidr("192.168.2.1", "192.168.1.0/24").is_err()
        );
    }
}
