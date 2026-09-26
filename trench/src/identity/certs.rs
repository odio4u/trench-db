use std::fs;
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
use uuid::Uuid;

pub(super) fn get_shareable_public_key() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let cert_path = "node-cert.pem";
    let pem = fs::read(cert_path)?;
    let mut reader = std::io::BufReader::new(pem.as_slice());

    let cert = rustls_pemfile::certs(&mut reader)
    .next()
    .ok_or("No certificate found")??;

    let cert_to_vec = cert.to_vec();
    Ok(cert_to_vec)
}

pub(super) fn create_certificates(node_id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
    let mut params = CertificateParams::default();

    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, &node_id.to_string());
    params.distinguished_name = dn;

    params.subject_alt_names = vec![
        SanType::URI(format!("urn:uuid:{}", node_id).try_into()?),
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
    let key_pair = KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).expect("failed to create certificate");
        
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
