mod http;
mod https;
mod options;
mod utils;

use clap::Parser;
use options::Opt;
use utils::{get_current_server_ip, update_server_ip};

#[tokio::main]
async fn main() {
    // Get server IP or use 0.0.0.0 if failed
    update_server_ip().await;
    let server_ip = get_current_server_ip();

    // Parse and validate CLI arguments
    let options = Opt::parse();
    options.validate();

    // Prepare allowed credentials from CLI options
    let allowed_credentials = if let Some(allowed_credentials) = options.auth {
        allowed_credentials
            .split(',')
            .map(|credentials| credentials.trim().to_string())
            .collect::<Vec<String>>()
    } else {
        Vec::<String>::new()
    };

    // Prepare allowed hosts from CLI options
    let allowed_hosts = if let Some(allowed_hosts) = options.hosts {
        allowed_hosts
            .split(',')
            .map(|host| host.trim().to_string())
            .collect::<Vec<String>>()
    } else {
        Vec::<String>::new()
    };

    // Get secret token from CLI options
    let secret_token = options.token.unwrap_or_default();

    // Create future for HTTP server
    let http_future = async {
        if options.no_http_server {
            return;
        }

        let http_port = options.http_port.unwrap_or(58080);
        let proxy_url = format!("http://{server_ip}:{http_port}");

        if allowed_credentials.is_empty() {
            println!("\n\x1B[34m\x1B[1mRunning HTTP server:\x1B[0m\n{proxy_url}\nTest: curl -v -x {proxy_url} https://api.ipify.org");
        } else {
            println!("\n\x1B[34m\x1B[1mRunning HTTP server with credentials:\x1B[0m");

            for credentials in &allowed_credentials {
                let proxy_url = format!("http://{credentials}@{server_ip}:{http_port}");

                println!(
                    "Proxy Url: {proxy_url}\nTest: curl -v -x {proxy_url} https://api.ipify.org"
                );
            }
        }

        // Print allowed hosts
        if !allowed_hosts.is_empty() {
            println!("Allowed Hosts: {allowed_hosts:?}");
        }

        // Print secret token
        if !secret_token.is_empty() {
            println!("Secret Token: {secret_token}");
        }

        let bind_addr = format!("{}:{}", server_ip, http_port).parse().unwrap();

        if let Err(e) = http::start_proxy(
            bind_addr,
            allowed_credentials.clone(),
            allowed_hosts.clone(),
            secret_token.clone(),
        )
        .await
        {
            println!("Error starting HTTP server: {e}");
        }
    };

    // Create future for HTTPS server
    let https_future = async {
        if options.no_https_server {
            return;
        }

        let https_port = options.https_port.unwrap_or(443);
        let host = if cfg!(debug_assertions) {
            format!("localhost:{https_port}")
        } else {
            format!("YOUR_DOMAIN:{https_port}")
        };

        if allowed_credentials.is_empty() {
            println!("\n\x1B[34m\x1B[1mRunning HTTPS server:\x1B[0m\nhttps://{host}\nTest: curl -v -x https://{host} https://api.ipify.org");
        } else {
            println!("\n\x1B[34m\x1B[1mRunning HTTPS server with credentials:\x1B[0m");
            for credentials in &allowed_credentials {
                println!("Proxy Url: https://{credentials}@{host}\nTest: curl -v -x https://{credentials}@{host} https://api.ipify.org");
            }
        }

        // Print allowed hosts
        if !allowed_hosts.is_empty() {
            println!("Allowed Hosts: {allowed_hosts:?}");
        }

        // Print secret token
        if !secret_token.is_empty() {
            println!("Secret Token: {secret_token}");
        }

        let bind_addr = format!("{}:{}", server_ip, https_port).parse().unwrap();

        if let Err(e) = https::start_proxy(
            bind_addr,
            allowed_credentials.clone(),
            allowed_hosts.clone(),
            secret_token.clone(),
            options.cert.unwrap(),
            options.pkey.unwrap(),
        )
        .await
        {
            println!("Error starting HTTPS server: {e}");
        }
    };

    // Join futures and wait for them to complete
    tokio::join!(http_future, https_future);
}
