use std::fs;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub node_address: String,
    pub status: String,
    pub region: String,
    pub id: String,
    pub anchor_address: String,
}

impl fmt::Display for NodeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "╔════════════════════════════════════╗")?;
        writeln!(f, "║           NODE DETAILS            ║")?;
        writeln!(f, "╠════════════════════════════════════╣")?;
        writeln!(f, "║ ID      : {}", self.id)?;
        writeln!(f, "║ Status  : {}", self.status)?;
        writeln!(f, "║ Region  : {}", self.region)?;
        writeln!(f, "║ Address : {}", self.node_address)?;
        writeln!(f, "║ Anchor  : {}", self.anchor_address)?;
        writeln!(f, "╚════════════════════════════════════╝")
    }
}

impl NodeConfig {
    pub fn from_file<P: AsRef<Path>>(file_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let file_path = file_path.as_ref();
        let file_content = fs::read_to_string(file_path)?;

        let mut node_address = String::new();
        let mut status = String::new();
        let mut region = String::new();
        let mut id = String::new();
        let mut anchor_address = String::new();

        for line in file_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("Invalid config line: {line}"))?;
            match key.trim() {
                "NodeAddress" => node_address = value.trim().to_string(),
                "Status" => status = value.trim().to_string(),
                "Region" => region = value.trim().to_string(),
                "ID" => id = value.trim().to_string(),
                "AnchorAddress" => anchor_address = value.trim().to_string(),
                _ => {}
            }
        }

        if id.is_empty() {
            return Err("ID is required in config.trench".into());
        }

        Ok(NodeConfig {
            node_address,
            status,
            region,
            id,
            anchor_address,
        })
    }
}
