//! MDM signature verification.

use color_eyre::eyre::WrapErr as _;

/// Verify the Mdm-Signature header and extract the signing certificate.
///
/// The Mdm-Signature header contains a base64-encoded PKCS#7 detached signature
/// over the request body.
///
/// Note: Full CMS verification is complex. This is a placeholder implementation.
pub fn verify_mdm_signature(
    signature_header: &str,
    _body: &[u8],
) -> color_eyre::eyre::Result<Vec<u8>> {
    use base64::Engine as _;

    // Decode the base64 signature
    let signature_der = base64::engine::general_purpose::STANDARD
        .decode(signature_header)
        .wrap_err("failed to decode Mdm-Signature base64")?;

    // In production, we would:
    // 1. Parse CMS SignedData structure
    // 2. Extract the signing certificate
    // 3. Verify the signature over the body
    // 4. Validate the certificate chain

    tracing::warn!("MDM signature verification is not fully implemented");

    // For now, just return the raw signature data
    // A full implementation would use the cms crate properly
    Ok(signature_der)
}

/// Signature verifier using a trusted CA.
pub struct SignatureVerifier {
    /// Trusted CA certificate (DER-encoded).
    #[allow(dead_code)]
    ca_cert: Vec<u8>,
}

impl SignatureVerifier {
    /// Create a new verifier with a trusted CA.
    pub fn new(ca_cert_der: Vec<u8>) -> Self {
        Self {
            ca_cert: ca_cert_der,
        }
    }

    /// Verify that a certificate was signed by the trusted CA.
    ///
    /// Note: This is a placeholder. Full implementation needs proper X.509 verification.
    pub fn verify_cert(&self, _cert_der: &[u8]) -> color_eyre::eyre::Result<()> {
        // In production, use x509-parser's verify_signature or webpki
        tracing::warn!("certificate verification is not fully implemented");
        Ok(())
    }
}
