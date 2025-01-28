use crate::server::proxy;

use std::net::SocketAddr;
use std::sync::Arc;

use hyper::server::conn::http1;
use hyper::{body, service::service_fn, Request};
use hyper_util::rt::TokioIo;

use tokio::net::TcpListener;

pub async fn start(
    addr: SocketAddr,
    allowed_hosts: &Arc<Vec<String>>,
    auth_credentials: &Arc<Vec<String>>,
    auth_tokens: &Arc<Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Listening on http://{}", addr);

    loop {
        let (stream, addr) = listener.accept().await?;

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
            if let Err(err) = http1::Builder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(TokioIo::new(stream), service)
                .with_upgrades()
                .await
            {
                tracing::error!("\x1B[31mfailed to serve connection: {err:#}\x1B[0m")
            }
        });
    }
}
