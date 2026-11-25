//! HTTPS for Elomar-Lang's network code (`std.http` and the AI providers).
//!
//! A server's certificate is trusted when it chains to one of the roots
//! bundled with the program (Mozilla's list) or to one the system trusts.
//! The second covers the antivirus programs and company proxies that read
//! HTTPS traffic by putting their own root in the system's store: without
//! it, every connection they touch fails with `UnknownIssuer`.

use std::sync::{Arc, OnceLock};

/// An agent builder that trusts the bundled and the system roots.
pub fn agent_builder() -> ureq::AgentBuilder {
    ureq::AgentBuilder::new().tls_config(config())
}

fn config() -> Arc<rustls::ClientConfig> {
    static CONFIG: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let config = rustls::ClientConfig::builder_with_provider(
                rustls::crypto::ring::default_provider().into(),
            )
            .with_protocol_versions(&[&rustls::version::TLS12, &rustls::version::TLS13])
            .expect("ring supports TLS 1.2 and 1.3")
            .with_root_certificates(roots())
            .with_no_client_auth();
            Arc::new(config)
        })
        .clone()
}

/// The bundled roots, then the system's (any it cannot read are skipped).
pub fn roots() -> rustls::RootCertStore {
    let mut store = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let system = rustls_native_certs::load_native_certs();
    store.add_parsable_certificates(system.certs);
    store
}

#[cfg(test)]
mod tests {
    #[test]
    fn has_the_bundled_roots_at_least() {
        assert!(super::roots().len() >= webpki_roots::TLS_SERVER_ROOTS.len());
    }

    #[test]
    fn builds_an_agent() {
        let _ = super::agent_builder().redirects(0).build();
    }
}
