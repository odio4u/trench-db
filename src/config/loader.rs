
use std::fs;
use std::fmt;


#[derive(Debug)]
pub struct Node {
    // fields for the node
    node_address: String,
    status: String,
    region: String,
    id: String,
    anchor_address: String,
}

impl fmt::Display for Node {
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

impl Node  {
    // methods for the node
    pub fn load_node_from_file(file_path: &str) -> Result<Self, Box<dyn std::error::Error>>  {
        let file_content = fs::read_to_string(file_path).expect("Failed to read file");
        
        let mut node_address: String = String::new();
        let mut status: String = String::new();
        let mut region: String = String::new();
        let mut id: String = String::new();
        let mut anchor_address: String = String::new();

        for line in file_content.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let (key, value) = line
                .split_once('=')
                .ok_or("Invalid config line")?;

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
            return Err("ID is required create the identifier follow this steps".into());
        }

        Ok(Node { node_address, status, region, id, anchor_address })

        
    }
}

