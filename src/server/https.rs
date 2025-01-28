use crate::server::proxy;

use std::net::SocketAddr;
use std::sync::Arc;

use hyper::{body, service::service_fn, Request};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;

use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use super::utils::load_proxy_config;

pub async fn start(
    addr: SocketAddr,
    allowed_hosts: &Arc<Vec<String>>,
    auth_credentials: &Arc<Vec<String>>,
    auth_tokens: &Arc<Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = create_server_config()?;
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Listening on https://{}", addr);

    loop {
        let (stream, addr) = listener.accept().await?;
        let acceptor = acceptor.clone();

        let allowed_hosts = Arc::clone(allowed_hosts);
        let auth_credentials = Arc::clone(auth_credentials);
        let auth_tokens = Arc::clone(auth_tokens);

        let service = service_fn(move |req: Request<body::Incoming>| {
            let allowed_hosts = Arc::clone(&allowed_hosts);
            let auth_credentials = Arc::clone(&auth_credentials);
            let auth_tokens = Arc::clone(&auth_tokens);
            async move {
                proxy::handle_request(req, addr.ip(), allowed_hosts, auth_credentials, auth_tokens)
                    .await
            }
        });

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(stream).await {
                Ok(tls_stream) => tls_stream,
                Err(_) => return,
            };

            if let Err(err) = Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(TokioIo::new(tls_stream), service)
                .await
            {
                tracing::error!("\x1B[31mfailed to serve connection: {err:#}\x1B[0m")
            }
        });
    }
}

fn create_server_config() -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let proxy_config = load_proxy_config();

    let cert = CertificateDer::pem_file_iter(proxy_config.https.tls.cert.unwrap())
        .expect("cannot open certificate file")
        .map(|result| result.unwrap())
        .collect();
    let private_key = PrivateKeyDer::from_pem_file(proxy_config.https.tls.key.unwrap())
        .expect("cannot open private key file");

    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert, private_key)
        .expect("failed to create server configuration");

    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(config)
}
