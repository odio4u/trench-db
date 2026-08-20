use uuid::Uuid;
use std::fmt;

pub struct NodeIdentity {
    pub id: Uuid,
    pub region: String,
    pub address: String,
    pub status: String,
    pub issuer: IssuerIdentity,
    pub fingerprint: String,
    
}

pub struct IssuerIdentity {
    pub id: Uuid,
    pub fingerprint: String,
    pub address: String,
    pub region: String,
    pub status: String,
    pub issuer_bootstraped: bool,
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

    pub fn bootstrap_node(&self) -> Result<Self, Box<dyn std::error::Error>> {
        

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
        })
    }
}