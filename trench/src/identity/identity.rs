use uuid::Uuid;
use std::{fmt, vec, writeln};
use byteser_derive::ByteSerializable;

#[derive(Debug, ByteSerializable)]
pub struct NodeIdentity {
    pub id: Uuid,
    pub region: String,
    pub address: String,
    pub status: String,
    pub issuer: IssuerIdentity,
    pub pubkey: vec::Vec<u8>,
    pub fingerprint: String,
    pub signature: String,
    pub bootstraped: bool,
}

#[derive(Debug, Clone, ByteSerializable)]
pub struct IssuerIdentity {
    pub id: Uuid,
    pub pubkey: vec::Vec<u8>,
    pub address: String,
    pub region: String,
    pub status: String,
    pub issuer_bootstraped: bool,
}

struct TrenchConfig {
    pub node_address: String,
    pub issuer_address: String,
    pub region: String,
    pub identity_vault_path: String,
}

impl fmt::Display for NodeIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "╔════════════════════════════════════╗")?;
        writeln!(f, "║           NODE DETAILS             ║")?;
        writeln!(f, "╠════════════════════════════════════╣")?;
        writeln!(f, "║ ID      : {}", self.id)?;
        writeln!(f, "║ Status  : {}", self.status)?;
        writeln!(f, "║ Region  : {}", self.region)?;
        writeln!(f, "║ Address : {}", self.address)?;
        writeln!(f, "║ Fingerprint : {}", self.fingerprint)?;
        writeln!(f, "║ bootstrapped : {}", self.bootstraped)?;
        writeln!(f, "║ Signature : {}", self.signature)?;
        writeln!(f, "╚════════════════════════════════════╝")?;
        Ok(())
    }
}


impl NodeIdentity {

    pub async fn new(bootstraped: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let config = Self::load_config()?;
        let vault = super::vault::IdentityVault::new(&config.identity_vault_path);

        if !vault.exists() {
            if bootstraped {
                let identity = Self::bootstrap_node(config).await?;
                identity.save_to_vault(&vault)?;
                return Ok(identity);
            }
            let identity = Self::full_node(config).await?;
            identity.save_to_vault(&vault)?;
            return Ok(identity);
        }

        if vault.exists() {
            vault.copy_certs_to_working_dir()?;
            return Self::from_vault(&vault);
        }

        Err(format!(
            "no identity vault found at {}. Run with --bootstrap to create one.",
            config.identity_vault_path
        )
        .into())
    }

    async fn bootstrap_node(config: TrenchConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let id = Uuid::new_v4();
        let region = if config.region.is_empty() {
            "us-east-1".to_string()
        } else {
            config.region
        };
        let address = config.node_address;
        let status = "active".to_string();

        super::certs::create_certificates(id)?;
        let pubkey = super::certs::get_shareable_public_key()?;

        let issuer = IssuerIdentity {
            id,
            pubkey: pubkey.clone(),
            address: address.clone(),
            region: region.clone(),
            status: status.clone(),
            issuer_bootstraped: true,
        };

        let fingerprint = super::certs::key_to_fingerprint(pubkey.clone())?;
        let signature = super::sig::call_issuer(id, fingerprint.clone(), issuer.clone(), true   )
        .await?;

        Ok(NodeIdentity {
            id,
            region,
            address,
            status,
            issuer,
            pubkey: pubkey,
            fingerprint: fingerprint,
            signature,
            bootstraped: true,
        })
    }

    async fn full_node(config: TrenchConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let id = Uuid::new_v4();
        let region = if config.region.is_empty() {
            "us-east-1".to_string()
        } else {
            config.region
        };
        let address = config.node_address;
        let status = "active".to_string();
        let issuer_address = config.issuer_address;

        super::certs::create_certificates(id)?;
        let pubkey = super::certs::get_shareable_public_key()?;
        

        let issuer = IssuerIdentity {
            id,
            pubkey: pubkey.clone(),
            address: issuer_address.clone(),
            region: region.clone(),
            status: status.clone(),
            issuer_bootstraped: false,
        };

        let fingerprint = super::certs::key_to_fingerprint(pubkey.clone())?;
        let signature = super::sig::call_issuer(id, fingerprint.clone(), issuer.clone(), false)
        .await?;

        Ok(NodeIdentity {
            id,
            region,
            address,
            status,
            issuer,
            pubkey: pubkey,
            fingerprint: fingerprint,
            signature,
            bootstraped: false,
        })
    }

    

    fn load_config() -> Result<TrenchConfig, Box<dyn std::error::Error>> {
        let config_path = "config.trench";
        let config_content = std::fs::read_to_string(config_path)?;
        if config_content.trim().is_empty() {
            return Err(format!("Config file {config_path} is empty").into());
        }

        let mut node_address = String::new();
        let mut issuer_address = String::new();
        let mut region = String::new();
        let mut identity_vault_path = String::from("identity");

        for line in config_content.lines() {
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
                "IssuerAddress" => issuer_address = value.to_string(),
                "Region" => region = value.to_string(),
                "IdentityVaultPath" => identity_vault_path = value.to_string(),
                _ => {}
            }
        }

        if node_address.is_empty() {
            return Err("NodeAddress is required in config.trench".into());
        }

        Ok(TrenchConfig {
            node_address,
            issuer_address: issuer_address,
            region,
            identity_vault_path,
        })
    }

    pub fn save_to_vault(&self, vault: &super::vault::IdentityVault) -> Result<(), Box<dyn std::error::Error>> {
        vault.save(self)
    }

    pub fn from_vault(vault: &super::vault::IdentityVault) -> Result<Self, Box<dyn std::error::Error>> {
        vault.load_identity()
    }

}