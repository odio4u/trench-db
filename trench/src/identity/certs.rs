use std::fs;
use rsa::sha2::{Digest, Sha256};
use rcgen::{
    CertificateParams,
    DistinguishedName,
    DnType,
    ExtendedKeyUsagePurpose,
    IsCa,
    KeyPair,
    KeyUsagePurpose,
    SanType,
};
use crate::identity::node_identity::NodeIdentity;

pub(super) fn build_fingerprint_from_public_key() -> Result<String, Box<dyn std::error::Error>> {
    let cert_path = "node-cert.pem";
    let pem = fs::read(cert_path)?;
    let mut reader = std::io::BufReader::new(pem.as_slice());

    let cert = rustls_pemfile::certs(&mut reader)
    .next()
    .ok_or("No certificate found")??;

    let digest = Sha256::digest(cert.as_ref());
    let fingerprint = digest
    .iter()
    .map(|byte| format!("{byte:02X}"))
    .collect::<Vec<_>>()
    .join(":");

    Ok(fingerprint)
}


pub(super) fn create_certificates(node_identity: &NodeIdentity) -> Result<(), Box<dyn std::error::Error>> {

        let mut params = CertificateParams::default();

        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, &node_identity.id.to_string());
        params.distinguished_name = dn;


        params.subject_alt_names = vec![
            SanType::URI(format!("urn:uuid:{}", node_identity.id.to_string()).try_into()?),
        ];

        params.is_ca = IsCa::NoCa;

    // Key usage
        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];

        // TLS usage
        params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ServerAuth,
            ExtendedKeyUsagePurpose::ClientAuth,
        ];

            // Generate RSA certificate key
        let key_pair = KeyPair::generate_for(&rcgen::PKCS_RSA_SHA256)?;
            
        // Self-sign
        let cert = params.self_signed(&key_pair)?;

        std::fs::write(
            "node-cert.pem",
            cert.pem(),
        )?;

        std::fs::write(
            "node-key.pem",
            key_pair.serialize_pem(),
        )?;
        Ok(())
    }
