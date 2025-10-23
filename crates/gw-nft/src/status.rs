use anyhow::Result;
use gw_core::NftTableStatus;
use tokio::process::Command;

pub struct NftStatusCollector;

impl NftStatusCollector {
    pub fn new() -> Self {
        Self
    }

    pub async fn collect_table_status(&self) -> Result<Vec<NftTableStatus>> {
        let output = Command::new("nft")
            .arg("-j")
            .arg("list")
            .arg("tables")
            .output()
            .await?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let json_output = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&json_output)?;

        let mut tables = vec![];
        if let Some(nftables) = parsed.get("nftables").and_then(|n| n.as_array()) {
            for item in nftables {
                if let Some(table) = item.get("table") {
                    if let (Some(name), Some(family)) = (
                        table.get("name").and_then(|n| n.as_str()),
                        table.get("family").and_then(|f| f.as_str()),
                    ) {
                        // Get chain and rule counts for this table
                        let (chains, rules) = self.count_chains_and_rules(family, name).await?;

                        tables.push(NftTableStatus {
                            name: name.to_string(),
                            family: family.to_string(),
                            chains,
                            rules,
                        });
                    }
                }
            }
        }

        Ok(tables)
    }

    async fn count_chains_and_rules(&self, family: &str, table: &str) -> Result<(usize, usize)> {
        let output = Command::new("nft")
            .arg("-j")
            .arg("list")
            .arg("table")
            .arg(family)
            .arg(table)
            .output()
            .await?;

        if !output.status.success() {
            return Ok((0, 0));
        }

        let json_output = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&json_output)?;

        let mut chain_count = 0;
        let mut rule_count = 0;

        if let Some(nftables) = parsed.get("nftables").and_then(|n| n.as_array()) {
            for item in nftables {
                if item.get("chain").is_some() {
                    chain_count += 1;
                }
                if item.get("rule").is_some() {
                    rule_count += 1;
                }
            }
        }

        Ok((chain_count, rule_count))
    }
}

impl Default for NftStatusCollector {
    fn default() -> Self {
        Self::new()
    }
}
