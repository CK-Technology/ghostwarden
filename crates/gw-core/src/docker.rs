use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Docker network information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerNetwork {
    pub name: String,
    pub id: String,
    pub driver: String,
    pub scope: String,
    pub bridge_name: Option<String>,
}

/// Docker bridge manager for compatibility with Docker networking
pub struct DockerBridgeManager;

impl DockerBridgeManager {
    pub fn new() -> Self {
        Self
    }

    /// List all Docker networks
    pub async fn list_networks(&self) -> Result<Vec<DockerNetwork>> {
        use tokio::process::Command;

        let output = Command::new("docker")
            .arg("network")
            .arg("ls")
            .arg("--format")
            .arg("{{json .}}")
            .output()
            .await
            .context("Failed to run docker network ls")?;

        if !output.status.success() {
            // Docker might not be installed or not running
            return Ok(vec![]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut networks = Vec::new();

        for line in stdout.lines() {
            if let Ok(network) = serde_json::from_str::<DockerNetwork>(line) {
                networks.push(network);
            }
        }

        Ok(networks)
    }

    /// Get Docker bridge names (docker0, br-*)
    pub async fn get_docker_bridges(&self) -> Result<Vec<String>> {
        use tokio::process::Command;

        let output = Command::new("docker")
            .arg("network")
            .arg("inspect")
            .arg("bridge")
            .output()
            .await
            .context("Failed to inspect docker bridge network")?;

        if !output.status.success() {
            return Ok(vec!["docker0".to_string()]); // Assume default docker0 bridge
        }

        // Parse bridge name from inspect output
        let _stdout = String::from_utf8_lossy(&output.stdout);
        let mut bridges = vec!["docker0".to_string()];

        // Also check for custom bridge networks
        if let Ok(networks) = self.list_networks().await {
            for network in networks {
                if network.driver == "bridge" && network.name != "bridge" {
                    // Docker bridge networks are named br-<hash>
                    bridges.push(format!("br-{}", &network.id[..12]));
                }
            }
        }

        Ok(bridges)
    }

    /// Check if Docker is running
    pub async fn is_docker_running(&self) -> bool {
        use tokio::process::Command;

        let output = Command::new("docker").arg("info").output().await;

        output.map(|o| o.status.success()).unwrap_or(false)
    }

    /// Check if a bridge is managed by Docker
    pub async fn is_docker_bridge(&self, bridge_name: &str) -> Result<bool> {
        if bridge_name == "docker0" {
            return Ok(true);
        }

        if bridge_name.starts_with("br-") {
            let docker_bridges = self.get_docker_bridges().await?;
            return Ok(docker_bridges.contains(&bridge_name.to_string()));
        }

        Ok(false)
    }

    /// Create a Docker network that uses a Ghostwarden bridge
    /// This allows Docker containers to use bridges managed by Ghostwarden
    pub async fn create_docker_network_on_bridge(
        &self,
        network_name: &str,
        bridge_name: &str,
        subnet: &str,
        gateway: &str,
    ) -> Result<()> {
        use tokio::process::Command;

        // Create Docker network using macvlan or bridge driver
        let output = Command::new("docker")
            .arg("network")
            .arg("create")
            .arg("--driver=bridge")
            .arg(&format!("--subnet={}", subnet))
            .arg(&format!("--gateway={}", gateway))
            .arg(&format!(
                "--opt=com.docker.network.bridge.name={}",
                bridge_name
            ))
            .arg(network_name)
            .output()
            .await
            .context("Failed to create Docker network")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create Docker network: {}", stderr);
        }

        println!(
            "Created Docker network '{}' on bridge {}",
            network_name, bridge_name
        );
        Ok(())
    }

    /// Attach a Docker container to a Ghostwarden bridge
    pub async fn attach_container_to_bridge(&self, container: &str, network: &str) -> Result<()> {
        use tokio::process::Command;

        let output = Command::new("docker")
            .arg("network")
            .arg("connect")
            .arg(network)
            .arg(container)
            .output()
            .await
            .context("Failed to connect container to network")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to attach container: {}", stderr);
        }

        println!(
            "Attached container '{}' to network '{}'",
            container, network
        );
        Ok(())
    }

    /// List containers on a Docker network
    pub async fn list_containers_on_network(&self, network: &str) -> Result<Vec<String>> {
        use tokio::process::Command;

        let output = Command::new("docker")
            .arg("network")
            .arg("inspect")
            .arg(network)
            .arg("--format={{range $k, $v := .Containers}}{{$v.Name}} {{end}}")
            .output()
            .await
            .context("Failed to inspect Docker network")?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let containers: Vec<String> = stdout
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        Ok(containers)
    }
}

impl Default for DockerBridgeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_docker_bridge_manager() {
        let mgr = DockerBridgeManager::new();

        // These tests will only pass if Docker is installed
        if mgr.is_docker_running().await {
            let bridges = mgr.get_docker_bridges().await.unwrap();
            assert!(!bridges.is_empty());
            assert!(bridges.contains(&"docker0".to_string()));
        }
    }
}
