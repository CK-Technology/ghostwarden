pub mod bridge;
pub mod diagnostics;
pub mod docker;
pub mod nftables;

pub use bridge::BridgeDiagnostics;
pub use diagnostics::{DiagnosticLevel, DiagnosticReport, DiagnosticResult};
pub use docker::DockerDiagnostics;
pub use nftables::NftablesDiagnostics;

/// Main troubleshooting interface
pub struct Troubleshooter {
    nft: NftablesDiagnostics,
    docker: DockerDiagnostics,
    bridge: BridgeDiagnostics,
}

impl Troubleshooter {
    pub fn new() -> Self {
        Self {
            nft: NftablesDiagnostics::new(),
            docker: DockerDiagnostics::new(),
            bridge: BridgeDiagnostics::new(),
        }
    }

    /// Run all diagnostics and generate comprehensive report
    pub async fn run_all(&self) -> anyhow::Result<DiagnosticReport> {
        let mut report = DiagnosticReport::new();

        // Run nftables diagnostics
        let nft_results = self.nft.diagnose().await?;
        report.add_section("nftables/iptables", nft_results);

        // Run Docker diagnostics
        let docker_results = self.docker.diagnose().await?;
        report.add_section("Docker Networking", docker_results);

        // Run bridge diagnostics
        let bridge_results = self.bridge.diagnose().await?;
        report.add_section("Bridge Configuration", bridge_results);

        Ok(report)
    }

    /// Check specific issue type
    pub async fn check_nftables(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        self.nft.diagnose().await
    }

    pub async fn check_docker(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        self.docker.diagnose().await
    }

    pub async fn check_bridges(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        self.bridge.diagnose().await
    }
}

impl Default for Troubleshooter {
    fn default() -> Self {
        Self::new()
    }
}
