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
    pub wal_path: String,
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
        writeln!(f, "╚════════════════════════════════════╝")?;
        Ok(())
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
        let mut wal_path = String::new();

        for line in file_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("Invalid config line: {line}"))?;
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match key.trim() {
                "NodeAddress" => node_address = value.to_string(),
                "Status" => status = value.to_string(),
                "Region" => region = value.to_string(),
                "ID" => id = value.to_string(),
                "AnchorAddress" => anchor_address = value.to_string(),
                "WalPath" => wal_path = value.to_string(),
                _ => {}
            }
        }

        if id.is_empty() {
            return Err(format!("ID is required in {}", file_path.display()).into());
        }

        if wal_path.is_empty() {
            wal_path = format!("data/{}/wal.log", id);
        }

        Ok(NodeConfig {
            node_address,
            status,
            region,
            id,
            anchor_address,
            wal_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_from_file_custom_path() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "NodeAddress=\"127.0.0.1:9001\"\nRegion=\"eu-west-1\"\nAnchorAddress=\"127.0.0.1:9000\"\nWalPath=\"data/custom/wal.log\"\nID=\"node-custom\""
        )
        .unwrap();

        let config = NodeConfig::from_file(file.path()).unwrap();
        assert_eq!(config.node_address, "127.0.0.1:9001");
        assert_eq!(config.region, "eu-west-1");
        assert_eq!(config.anchor_address, "127.0.0.1:9000");
        assert_eq!(config.wal_path, "data/custom/wal.log");
        assert_eq!(config.id, "node-custom");
    }

    #[test]
    fn test_from_file_missing_id() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "NodeAddress=\"127.0.0.1:9001\"").unwrap();

        let result = NodeConfig::from_file(file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ID is required"));
    }
}

