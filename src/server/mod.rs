use std::{net::SocketAddr, sync::Arc};

mod http;
mod https;
mod proxy;
mod tunnel;
mod utils;

use utils::{get_server_ip, load_proxy_config, terminate_process_on_port};

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let proxy_config = load_proxy_config();

    tracing::info!(
        "Default connection limit (soft, hard): {:?}",
        rlimit::Resource::NOFILE.get()?
    );

    // Set max open connection limit
    match rlimit::increase_nofile_limit(rlimit::INFINITY) {
        Ok(limit) => {
            tracing::info!("Setting max open connection limit to {}", limit);
        }
        Err(e) => {
            tracing::error!("\x1B[31mFailed to increase the open connection limit: {e}\x1B[0m");
            std::process::exit(1);
        }
    }

    let ip = get_server_ip().await;

    let http_future = async {
        if !proxy_config.http.enabled {
            return;
        }

        let port = proxy_config.http.port;
        let addr = SocketAddr::new(ip, port);

        terminate_process_on_port(port).await;

        let allowed_hosts = Arc::new(proxy_config.http.allowed_hosts);
        let auth_credentials = Arc::new(proxy_config.http.auth.credentials);
        let auth_tokens = Arc::new(proxy_config.http.auth.tokens);

        tracing::info!("[HTTP] Allowed hosts: {:?}", allowed_hosts);
        tracing::info!("[HTTP] Allowed tokens: {:?}", auth_tokens);

        if let Err(e) = http::start(addr, &allowed_hosts, &auth_credentials, &auth_tokens).await {
            tracing::error!("\x1B[31mHTTP server failed to start: {e}\x1B[0m");
            std::process::exit(1);
        }
    };

    let https_future = async {
        if !proxy_config.https.enabled {
            return;
        }

        let port = proxy_config.https.port;
        let addr = SocketAddr::new(ip, port);

        terminate_process_on_port(port).await;

        let allowed_hosts = Arc::new(proxy_config.https.allowed_hosts);
        let auth_credentials = Arc::new(proxy_config.https.auth.credentials);
        let auth_tokens = Arc::new(proxy_config.https.auth.tokens);

        tracing::info!("[HTTPS] Allowed hosts: {:?}", allowed_hosts);
        tracing::info!("[HTTPS] Allowed tokens: {:?}", auth_tokens);

        if let Err(e) = https::start(addr, &allowed_hosts, &auth_credentials, &auth_tokens).await {
            tracing::error!("\x1B[31mHTTPS server failed to start: {e}\x1B[0m");
            std::process::exit(1);
        }
    };

    tokio::join!(http_future, https_future);
    Ok(())
}
