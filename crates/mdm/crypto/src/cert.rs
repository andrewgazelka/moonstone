//! Certificate utilities.

use color_eyre::eyre::WrapErr as _;
use x509_parser::prelude::*;

/// Extract the APNs topic from a push certificate.
///
/// The topic is stored in the UID attribute of the subject with prefix "com.apple.mgmt.".
pub fn extract_topic_from_cert(cert_der: &[u8]) -> color_eyre::eyre::Result<String> {
    let (_, cert) =
        X509Certificate::from_der(cert_der).wrap_err("failed to parse certificate DER")?;

    // Look for UID attribute containing APNs topic
    for rdn in cert.subject().iter() {
        for attr in rdn.iter() {
            // Try to get the value as a string
            if let Ok(value) = attr.attr_value().as_str() {
                if value.starts_with("com.apple.mgmt.") {
                    return Ok(value.to_string());
                }
            }
        }
    }

    color_eyre::eyre::bail!("no APNs topic found in certificate")
}

/// Parse a certificate from PEM format.
pub fn parse_pem_cert(pem_str: &str) -> color_eyre::eyre::Result<Vec<u8>> {
    let pem_data =
        ::pem::parse(pem_str).map_err(|e| color_eyre::eyre::eyre!("failed to parse PEM: {}", e))?;

    if pem_data.tag() != "CERTIFICATE" {
        color_eyre::eyre::bail!("PEM is not a certificate, got: {}", pem_data.tag());
    }

    Ok(pem_data.into_contents())
}

/// Extract certificate from RFC 9440 header format (colon-delimited base64 DER).
///
/// Format: `:base64EncodedDERCert:`
pub fn extract_rfc9440_cert(header: &str) -> color_eyre::eyre::Result<Vec<u8>> {
    let header = header.trim();

    if !header.starts_with(':') || !header.ends_with(':') {
        color_eyre::eyre::bail!("invalid RFC 9440 format: must be :base64:");
    }

    let b64 = &header[1..header.len() - 1];

    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .wrap_err("failed to decode base64 certificate")
}

/// Extract certificate from URL-escaped PEM header.
pub fn extract_pem_header(header: &str) -> color_eyre::eyre::Result<Vec<u8>> {
    let pem_str = urlencoding::decode(header).wrap_err("failed to URL-decode PEM header")?;

    parse_pem_cert(&pem_str)
}

/// Compute a simple hash of certificate DER for identification.
///
/// Note: This is a simplified implementation. In production, use a proper SHA-256.
pub fn cert_hash(cert_der: &[u8]) -> [u8; 32] {
    // Simple XOR-based hash for identification (not cryptographically secure)
    // In production, use ring::digest::digest or sha2 crate
    let mut result = [0u8; 32];
    for (i, &byte) in cert_der.iter().enumerate() {
        result[i % 32] ^= byte.wrapping_add((i / 32) as u8);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rfc9440_extraction() {
        // Base64 of "test"
        let header = ":dGVzdA==:";
        let result = extract_rfc9440_cert(header).unwrap();
        assert_eq!(result, b"test");
    }
}
