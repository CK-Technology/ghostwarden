use anyhow::Result;
use axum::{Router, routing::get};
use prometheus::{Encoder, IntCounterVec, IntGaugeVec, Opts, Registry, TextEncoder};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Metrics collector for GhostWarden
pub struct MetricsCollector {
    registry: Arc<Registry>,

    // Bridge metrics
    bridge_status: IntGaugeVec,

    // nftables metrics
    nft_tables_count: IntGaugeVec,
    nft_chains_count: IntGaugeVec,
    nft_rules_count: IntGaugeVec,

    // DHCP metrics
    dhcp_leases_count: IntGaugeVec,

    // Apply/rollback metrics
    apply_success: IntCounterVec,
    apply_failure: IntCounterVec,
    rollback_triggered: IntCounterVec,
}

impl MetricsCollector {
    pub fn new() -> Result<Self> {
        let registry = Arc::new(Registry::new());

        // Bridge metrics
        let bridge_status = IntGaugeVec::new(
            Opts::new(
                "ghostwarden_bridge_status",
                "Bridge interface status (1=up, 0=down)",
            ),
            &["bridge_name"],
        )?;
        registry.register(Box::new(bridge_status.clone()))?;

        // nftables metrics
        let nft_tables_count = IntGaugeVec::new(
            Opts::new("ghostwarden_nft_tables_count", "Number of nftables tables"),
            &["family"],
        )?;
        registry.register(Box::new(nft_tables_count.clone()))?;

        let nft_chains_count = IntGaugeVec::new(
            Opts::new("ghostwarden_nft_chains_count", "Number of nftables chains"),
            &["table_name"],
        )?;
        registry.register(Box::new(nft_chains_count.clone()))?;

        let nft_rules_count = IntGaugeVec::new(
            Opts::new("ghostwarden_nft_rules_count", "Number of nftables rules"),
            &["table_name"],
        )?;
        registry.register(Box::new(nft_rules_count.clone()))?;

        // DHCP metrics
        let dhcp_leases_count = IntGaugeVec::new(
            Opts::new(
                "ghostwarden_dhcp_leases_count",
                "Number of active DHCP leases",
            ),
            &["network"],
        )?;
        registry.register(Box::new(dhcp_leases_count.clone()))?;

        // Apply/rollback metrics
        let apply_success = IntCounterVec::new(
            Opts::new(
                "ghostwarden_apply_success_total",
                "Total successful apply operations",
            ),
            &["topology"],
        )?;
        registry.register(Box::new(apply_success.clone()))?;

        let apply_failure = IntCounterVec::new(
            Opts::new(
                "ghostwarden_apply_failure_total",
                "Total failed apply operations",
            ),
            &["topology", "reason"],
        )?;
        registry.register(Box::new(apply_failure.clone()))?;

        let rollback_triggered = IntCounterVec::new(
            Opts::new(
                "ghostwarden_rollback_triggered_total",
                "Total rollback operations",
            ),
            &["topology", "reason"],
        )?;
        registry.register(Box::new(rollback_triggered.clone()))?;

        Ok(Self {
            registry,
            bridge_status,
            nft_tables_count,
            nft_chains_count,
            nft_rules_count,
            dhcp_leases_count,
            apply_success,
            apply_failure,
            rollback_triggered,
        })
    }

    /// Update bridge metrics from network status
    pub fn update_bridge_metrics(&self, bridges: &[gw_core::BridgeStatus]) -> Result<()> {
        for bridge in bridges {
            // Update status (1 for UP, 0 for DOWN)
            let status_value = if bridge.state.to_uppercase() == "UP" {
                1
            } else {
                0
            };
            self.bridge_status
                .with_label_values(&[&bridge.name])
                .set(status_value);
        }
        Ok(())
    }

    /// Update nftables metrics
    pub fn update_nft_metrics(&self, nft_status: &[gw_core::NftTableStatus]) -> Result<()> {
        // Count tables by family
        let mut family_counts = std::collections::HashMap::new();
        for table in nft_status {
            *family_counts.entry(&table.family).or_insert(0) += 1;

            self.nft_chains_count
                .with_label_values(&[&table.name])
                .set(table.chains as i64);

            self.nft_rules_count
                .with_label_values(&[&table.name])
                .set(table.rules as i64);
        }

        for (family, count) in family_counts {
            self.nft_tables_count
                .with_label_values(&[family])
                .set(count);
        }

        Ok(())
    }

    /// Update DHCP lease metrics
    pub fn update_dhcp_metrics(&self, leases: &[gw_core::DhcpLease], network: &str) -> Result<()> {
        self.dhcp_leases_count
            .with_label_values(&[network])
            .set(leases.len() as i64);
        Ok(())
    }

    /// Record successful apply
    pub fn record_apply_success(&self, topology: &str) {
        self.apply_success.with_label_values(&[topology]).inc();
    }

    /// Record failed apply
    pub fn record_apply_failure(&self, topology: &str, reason: &str) {
        self.apply_failure
            .with_label_values(&[topology, reason])
            .inc();
    }

    /// Record rollback
    pub fn record_rollback(&self, topology: &str, reason: &str) {
        self.rollback_triggered
            .with_label_values(&[topology, reason])
            .inc();
    }

    /// Get the registry for HTTP server
    pub fn registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }

    /// Render metrics in Prometheus text format
    pub fn render_metrics(&self) -> Result<String> {
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics collector")
    }
}

/// HTTP server for Prometheus metrics endpoint
pub struct MetricsServer {
    collector: Arc<RwLock<MetricsCollector>>,
    addr: std::net::SocketAddr,
}

impl MetricsServer {
    pub fn new(collector: MetricsCollector, port: u16) -> Self {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        Self {
            collector: Arc::new(RwLock::new(collector)),
            addr,
        }
    }

    /// Start the metrics HTTP server
    pub async fn serve(self) -> Result<()> {
        let collector = self.collector.clone();

        let app = Router::new().route(
            "/metrics",
            get(move || {
                let collector = collector.clone();
                async move {
                    let collector = collector.read().await;
                    match collector.render_metrics() {
                        Ok(metrics) => metrics,
                        Err(e) => format!("# Error rendering metrics: {}", e),
                    }
                }
            }),
        );

        println!(
            "📊 Metrics server listening on http://{}/metrics",
            self.addr
        );

        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        assert!(collector.is_ok());
    }

    #[test]
    fn test_metrics_rendering() {
        let collector = MetricsCollector::new().unwrap();

        // Set some test values first
        collector
            .bridge_status
            .with_label_values(&["test-br0"])
            .set(1);
        collector
            .nft_tables_count
            .with_label_values(&["inet"])
            .set(1);
        collector
            .dhcp_leases_count
            .with_label_values(&["test-net"])
            .set(5);

        let metrics = collector.render_metrics();
        assert!(metrics.is_ok());

        let output = metrics.unwrap();
        // Now these should be present
        assert!(output.contains("ghostwarden_bridge_status"));
        assert!(output.contains("ghostwarden_nft_tables_count"));
        assert!(output.contains("ghostwarden_dhcp_leases_count"));
    }
}
