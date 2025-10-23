use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub family: String,
    pub name: String,
    pub chains: Vec<Chain>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    pub name: String,
    pub r#type: String,
    pub hook: Option<String>,
    pub priority: Option<i32>,
    pub policy: Option<String>,
}

impl Table {
    pub fn new(name: &str) -> Self {
        Self {
            family: "inet".to_string(),
            name: name.to_string(),
            chains: vec![],
        }
    }
}
