use uuid::Uuid;
use std::fmt;
use byteser_derive::ByteSerializable;

#[derive(Debug, ByteSerializable)]
pub struct NodeIdentity {
    pub id: Uuid,
    pub region: String,
    pub address: String,
    pub status: String,
    pub issuer: IssuerIdentity,
    pub fingerprint: String,
    pub bootstraped: bool,
}

#[derive(Debug, ByteSerializable)]
pub struct IssuerIdentity {
    pub id: Uuid,
    pub fingerprint: String,
    pub address: String,
    pub region: String,
    pub status: String,
    pub issuer_bootstraped: bool,
}

struct TrenchConfig {
    pub id: Option<Uuid>,
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
    /// Creates a new node identity.
    ///
    /// * `bootstraped` - when `true` a fresh identity (and self-signed issuer)
    ///   is generated; when `false` the identity is loaded from `config.trench`.
    pub fn new(bootstraped: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let config = Self::load_config()?;
        if bootstraped {
            Self::bootstrap_node(config)
        } else {
            Self::full_node(config)
        }
    }

    fn bootstrap_node(config: TrenchConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let id = config.id.unwrap_or_else(Uuid::new_v4);
        let region = if config.region.is_empty() {
            "us-east-1".to_string()
        } else {
            config.region
        };
        let address = config.node_address;
        let status = "active".to_string();

        super::certs::create_certificates(id)?;
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

    fn full_node(config: TrenchConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let status = "active".to_string();

        let id = config
            .id
            .ok_or("ID is required in config.trench for a full node")?;

        super::certs::create_certificates(id)?;
        let fingerprint = super::certs::build_fingerprint_from_public_key()?;

        let issuer = IssuerIdentity {
            id: config.issuer.id,
            fingerprint: config.issuer.fingerprint,
            address: config.issuer.address,
            region: config.issuer.region,
            status: config.issuer.status,
            issuer_bootstraped: config.issuer.issuer_bootstraped,
        };

        Ok(NodeIdentity {
            id,
            region: config.region,
            address: config.node_address,
            status,
            issuer,
            fingerprint,
            bootstraped: false,
        })
    }

    fn load_config() -> Result<TrenchConfig, Box<dyn std::error::Error>> {
        let config_path = "config.trench";
        let config_content = std::fs::read_to_string(config_path)?;
        if config_content.trim().is_empty() {
            return Err(format!("Config file {config_path} is empty").into());
        }

        let mut id: Option<Uuid> = None;
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
                "ID" => {
                    let value = value.trim_matches('"').trim_matches('\'').trim();
                    id = if value.is_empty() {
                        None
                    } else {
                        Some(Uuid::parse_str(value)?)
                    }
                }
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
            id,
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