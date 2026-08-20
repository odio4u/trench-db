use uuid::Uuid;
use std::fmt;

pub struct NodeIdentity {
    pub id: Uuid,
    pub region: String,
    pub address: String,
    pub status: String,
    pub issuer: IssuerIdentity,
    pub fingerprint: String,
    pub bootstraped: bool,
    
}

pub struct IssuerIdentity {
    pub id: Uuid,
    pub fingerprint: String,
    pub address: String,
    pub region: String,
    pub status: String,
    pub issuer_bootstraped: bool,
}

struct TrenchConfig {
    pub node_address: String,
    pub region: String,
    pub issuer: IssuerIdentity
}

impl fmt::Display for NodeIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "╔════════════════════════════════════╗")?;
        writeln!(f, "║           NODE DETAILS            ║")?;
        writeln!(f, "╠════════════════════════════════════╣")?;
        writeln!(f, "║ ID      : {}", self.id)?;
        writeln!(f, "║ Status  : {}", self.status)?;
        writeln!(f, "║ Region  : {}", self.region)?;
        writeln!(f, "║ Address : {}", self.address)?;
        writeln!(f, "║ Anchor  : {}", self.fingerprint)?;
        writeln!(f, "╚════════════════════════════════════╝")?;
        Ok(())
    }
}


impl NodeIdentity {

    pub fn new(&self, bootstraped: bool) -> Result<Self, Box<dyn std::error::Error>> {

        if bootstraped {
            self.bootstrap_node()
        } else {
            self.full_node()
        }
    }

    fn bootstrap_node(&self) -> Result<Self, Box<dyn std::error::Error>> {
        let id = Uuid::new_v4();
        let region = "us-east-1".to_string();
        let address = "".to_string();
        let status = "active".to_string();


        super::certs::create_certificates(self)?;
        let fingerprint = super::certs::build_fingerprint_from_public_key()?;
        let issuer = IssuerIdentity {
            id,
            fingerprint: fingerprint.clone(),
            address: address.clone(),
            region: region.clone(),
            status: status.clone(),
            issuer_bootstraped: true,
        };

        Ok(NodeIdentity {
            id,
            region,
            address,
            status,
            issuer,
            fingerprint,
            bootstraped: true,
        })
    }

    fn full_node(&self) -> Result<Self, Box<dyn std::error::Error>> {
        let id = Uuid::new_v4();
        let status = "active".to_string();


        let config = self.load_config()?;
        super::certs::create_certificates(self)?;
        let fingerprint = super::certs::build_fingerprint_from_public_key()?;

        let issuer = IssuerIdentity {
            id: config.issuer.id.clone(),
            fingerprint: config.issuer.fingerprint.clone(),
            address: config.issuer.address.clone(),
            region: config.issuer.region.clone(),
            status: config.issuer.status.clone(),
            issuer_bootstraped: config.issuer.issuer_bootstraped,
        };

        Ok(NodeIdentity {
            id: id,
            region: config.region.clone(),
            address: config.node_address.clone(),
            status: status,
            issuer,
            fingerprint: fingerprint,
            bootstraped: false,
        })

    }

    fn load_config(&self) -> Result<TrenchConfig, Box<dyn std::error::Error>> {
        let config_path = "config.trench";
        let config_content = std::fs::read_to_string(config_path)?;

        let mut node_address = String::new();
        let mut region = String::new();
        let mut issuer_id = String::new();
        let mut issuer_fingerprint = String::new();
        let mut issuer_address = String::new();
        let mut issuer_region = String::new();
        let mut issuer_status = String::new();
        let mut issuer_bootstraped = false;

        for line in config_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("Invalid config line: {line}"))?;
            match key.trim() {
                "NodeAddress" => node_address = value.trim().to_string(),
                "Region" => region = value.trim().to_string(),
                "IssuerID" => issuer_id = value.trim().to_string(),
                "IssuerFingerprint" => issuer_fingerprint = value.trim().to_string(),
                "IssuerAddress" => issuer_address = value.trim().to_string(),
                "IssuerRegion" => issuer_region = value.trim().to_string(),
                "IssuerStatus" => issuer_status = value.trim().to_string(),
                "IssuerBootstraped" => issuer_bootstraped = value.trim().parse::<bool>()?,
                _ => {}
            }
        }
        Ok(TrenchConfig {
            node_address,
            region,
            issuer: IssuerIdentity {
                id: Uuid::parse_str(&issuer_id)?,
                fingerprint: issuer_fingerprint,
                address: issuer_address,
                region: issuer_region,
                status: issuer_status,
                issuer_bootstraped,
            },
        })
    }
}