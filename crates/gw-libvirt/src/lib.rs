use anyhow::{Context, Result};

/// Network interface model types
#[derive(Debug, Clone, Copy)]
pub enum InterfaceModel {
    Virtio,
    E1000,
    Rtl8139,
    VmxNet3,
}

impl InterfaceModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            InterfaceModel::Virtio => "virtio",
            InterfaceModel::E1000 => "e1000",
            InterfaceModel::Rtl8139 => "rtl8139",
            InterfaceModel::VmxNet3 => "vmxnet3",
        }
    }
}

/// Interface attachment options
#[derive(Debug, Clone)]
pub struct InterfaceOptions {
    pub model: InterfaceModel,
    pub mac_address: Option<String>,
    pub bandwidth_in_kbps: Option<u32>,
    pub bandwidth_out_kbps: Option<u32>,
    pub live: bool, // Hot-plug if VM is running
}

impl Default for InterfaceOptions {
    fn default() -> Self {
        Self {
            model: InterfaceModel::Virtio,
            mac_address: None,
            bandwidth_in_kbps: None,
            bandwidth_out_kbps: None,
            live: false,
        }
    }
}

pub struct LibvirtManager;

impl LibvirtManager {
    pub fn new() -> Self {
        Self
    }

    /// Generate a random MAC address in the range 52:54:00:xx:xx:xx (libvirt default)
    pub fn generate_mac_address() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: [u8; 3] = rng.r#gen();
        format!(
            "52:54:00:{:02x}:{:02x}:{:02x}",
            bytes[0], bytes[1], bytes[2]
        )
    }

    /// List all VMs and their network interfaces
    pub async fn list_vms(&self) -> Result<Vec<VmInfo>> {
        use tokio::process::Command;

        let output = Command::new("virsh")
            .arg("list")
            .arg("--all")
            .output()
            .await
            .context("Failed to run virsh list")?;

        if !output.status.success() {
            anyhow::bail!("virsh list failed");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut vms = vec![];

        for line in stdout.lines().skip(2) {
            // Skip header lines
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let id = parts[0].parse().ok();
                let name = parts[1].to_string();
                let state = parts[2..].join(" ");

                vms.push(VmInfo {
                    id,
                    name: name.clone(),
                    state,
                    interfaces: self.get_vm_interfaces(&name).await?,
                });
            }
        }

        Ok(vms)
    }

    async fn get_vm_interfaces(&self, vm_name: &str) -> Result<Vec<String>> {
        use tokio::process::Command;

        let output = Command::new("virsh")
            .arg("domiflist")
            .arg(vm_name)
            .output()
            .await?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut interfaces = vec![];

        for line in stdout.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                interfaces.push(parts[0].to_string());
            }
        }

        Ok(interfaces)
    }

    /// Attach VM to a bridge network (simple version)
    pub async fn attach_vm_to_bridge(
        &self,
        vm_name: &str,
        bridge: &str,
        tap_name: Option<&str>,
    ) -> Result<()> {
        let options = InterfaceOptions::default();
        self.attach_vm_to_bridge_advanced(vm_name, bridge, tap_name, &options)
            .await
    }

    /// Attach VM to a bridge network with advanced options
    pub async fn attach_vm_to_bridge_advanced(
        &self,
        vm_name: &str,
        bridge: &str,
        tap_name: Option<&str>,
        options: &InterfaceOptions,
    ) -> Result<()> {
        use tokio::process::Command;

        println!(
            "Attaching VM {} to bridge {} with model {}",
            vm_name,
            bridge,
            options.model.as_str()
        );

        // Generate MAC address if not provided
        let mac = options
            .mac_address
            .clone()
            .unwrap_or_else(Self::generate_mac_address);

        // Build interface XML
        let mut xml = format!(
            r#"<interface type='bridge'>
  <source bridge='{}'/>
  <model type='{}'/>
  <mac address='{}'/>
"#,
            bridge,
            options.model.as_str(),
            mac
        );

        // Add target device if specified
        if let Some(tap) = tap_name {
            xml.push_str(&format!("  <target dev='{}'/>\n", tap));
        }

        // Add bandwidth limiting if specified
        if options.bandwidth_in_kbps.is_some() || options.bandwidth_out_kbps.is_some() {
            xml.push_str("  <bandwidth>\n");
            if let Some(inbound) = options.bandwidth_in_kbps {
                xml.push_str(&format!("    <inbound average='{}'/>\n", inbound));
            }
            if let Some(outbound) = options.bandwidth_out_kbps {
                xml.push_str(&format!("    <outbound average='{}'/>\n", outbound));
            }
            xml.push_str("  </bandwidth>\n");
        }

        xml.push_str("</interface>");

        // Write XML to temp file
        let temp_file = "/tmp/gw-interface.xml";
        std::fs::write(temp_file, &xml)?;

        // Attach interface
        let mut cmd = Command::new("virsh");
        cmd.arg("attach-device").arg(vm_name).arg(temp_file);

        if options.live {
            cmd.arg("--live").arg("--config"); // Hot-plug and persist
        } else {
            cmd.arg("--config"); // Only update config
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to attach interface: {}", stderr);
        }

        println!(
            "✅ Attached VM {} to bridge {} (MAC: {})",
            vm_name, bridge, mac
        );
        Ok(())
    }

    /// Create a libvirt network definition from a Ghostwarden bridge
    pub async fn create_libvirt_network(
        &self,
        network_name: &str,
        bridge_name: &str,
        cidr: &str,
    ) -> Result<()> {
        use tokio::process::Command;

        // Parse CIDR to extract network address and gateway
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid CIDR format: {}", cidr);
        }

        let network_addr = parts[0];
        let prefix = parts[1];

        // Generate network XML
        let xml = format!(
            r#"<network>
  <name>{}</name>
  <forward mode='bridge'/>
  <bridge name='{}'/>
  <ip address='{}' prefix='{}'>
  </ip>
</network>"#,
            network_name, bridge_name, network_addr, prefix
        );

        // Write XML to temp file
        let temp_file = "/tmp/gw-network.xml";
        std::fs::write(temp_file, &xml)?;

        // Define network
        let output = Command::new("virsh")
            .arg("net-define")
            .arg(temp_file)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to define libvirt network: {}", stderr);
        }

        // Start and autostart network
        Command::new("virsh")
            .arg("net-start")
            .arg(network_name)
            .output()
            .await?;

        Command::new("virsh")
            .arg("net-autostart")
            .arg(network_name)
            .output()
            .await?;

        println!(
            "✅ Created libvirt network '{}' on bridge {}",
            network_name, bridge_name
        );
        Ok(())
    }

    /// Delete a libvirt network
    pub async fn delete_libvirt_network(&self, network_name: &str) -> Result<()> {
        use tokio::process::Command;

        // Stop network
        let _ = Command::new("virsh")
            .arg("net-destroy")
            .arg(network_name)
            .output()
            .await;

        // Undefine network
        let output = Command::new("virsh")
            .arg("net-undefine")
            .arg(network_name)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore "network not found" errors
            if !stderr.contains("Network not found") {
                anyhow::bail!("Failed to delete libvirt network: {}", stderr);
            }
        }

        println!("Deleted libvirt network: {}", network_name);
        Ok(())
    }

    /// Detach VM from a bridge
    pub async fn detach_vm_interface(&self, vm_name: &str, interface: &str) -> Result<()> {
        use tokio::process::Command;

        println!("Detaching interface {} from VM {}", interface, vm_name);

        let output = Command::new("virsh")
            .arg("detach-interface")
            .arg(vm_name)
            .arg("bridge")
            .arg("--config")
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to detach interface: {}", stderr);
        }

        println!("✅ Detached interface from VM {}", vm_name);
        Ok(())
    }
}

impl Default for LibvirtManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct VmInfo {
    pub id: Option<i32>,
    pub name: String,
    pub state: String,
    pub interfaces: Vec<String>,
}
