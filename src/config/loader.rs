
use std::fs;



#[derive(Debug)]
pub struct Node {
    // fields for the node
    NodeAddress: String,
    Status: String,
    Region: String,
    ID: String,
    AnchorAddress: String,
}

impl Node  {
    // methods for the node
    fn LoadNodesFromFile(file_path: &str) -> Result<Self, Box<dyn std::error::Error>>  {
        let file_content = fs::read_to_string(file_path).expect("Failed to read file");
        
        let mut NodeAddresses: String = String::new();
        let mut Statuses: String = String::new();
        let mut Regions: String = String::new();
        let mut IDs: String = String::new();
        let mut AnchorAddresses: String = String::new();

        for line in file_content.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let (key, value) = line
                .split_once('=')
                .ok_or("Invalid config line")?;

            match key.trim() {
                "NodeAddress" => NodeAddresses = value.trim().to_string(),
                "Status" => Statuses = value.trim().to_string(),
                "Region" => Regions = value.trim().to_string(),
                "ID" => IDs = value.trim().to_string(),
                "AnchorAddress" => AnchorAddresses = value.trim().to_string(),
                _ => {}
            }
            
        }
        Ok(Node { NodeAddress: NodeAddresses, Status: Statuses, Region: Regions, ID: IDs, AnchorAddress: AnchorAddresses })

        
    }
}

