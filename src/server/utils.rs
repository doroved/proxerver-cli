use crate::options::Opt;
use clap::Parser;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    net::{IpAddr, Ipv4Addr},
    process::Command,
};
use tokio::process::Command as TokioCommand;

#[derive(Debug, Deserialize)]
pub struct ProxyServerConfig {
    pub http: HttpConfig,
    pub https: HttpsConfig,
}

#[derive(Debug, Deserialize)]
pub struct HttpConfig {
    pub enabled: bool,
    pub port: u16,
    pub allowed_hosts: Vec<String>,
    pub auth: AuthConfig,
}

#[derive(Debug, Deserialize)]
pub struct HttpsConfig {
    pub enabled: bool,
    pub port: u16,
    pub allowed_hosts: Vec<String>,
    pub auth: AuthConfig,
    pub tls: TlsConfig,
}

#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    pub credentials: Vec<String>,
    pub tokens: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct TlsConfig {
    pub cert: Option<String>,
    pub key: Option<String>,
}

pub fn to_sha256(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}

pub async fn get_server_ip() -> IpAddr {
    let output = Command::new("sh")
        .arg("-c")
        .arg("hostname -I | awk '{print $1}'")
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        match String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<IpAddr>()
        {
            Ok(ip) => ip,
            Err(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
        }
    } else {
        panic!("Failed to get Server IP: {:?}", output.status);
    }
}

pub async fn terminate_process_on_port(port: u16) {
    let output = TokioCommand::new("lsof")
        .arg("-t")
        .arg(format!("-i:{}", port))
        .output()
        .await
        .expect("Failed to execute lsof command");

    if !output.stdout.is_empty() {
        let pid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        TokioCommand::new("kill")
            .arg("-9")
            .arg(&pid)
            .output()
            .await
            .expect("Failed to terminate proxerver");
        tracing::info!("Proxerver on port {} has been terminated", port);

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}

pub fn load_proxy_config() -> ProxyServerConfig {
    let options = Opt::parse();
    let home = std::env::var("HOME").unwrap();
    let file_path = options
        .config
        .unwrap_or(format!("{}/.proxerver-cli/config.toml", home));

    let content = fs::read_to_string(file_path).expect("Failed to read configuration file");
    let proxy_config: ProxyServerConfig =
        toml::de::from_str(&content).expect("Failed to parse configuration file");

    ProxyServerConfig {
        http: HttpConfig {
            enabled: proxy_config.http.enabled,
            port: proxy_config.http.port,
            allowed_hosts: proxy_config.http.allowed_hosts,
            auth: AuthConfig {
                credentials: proxy_config.http.auth.credentials,
                tokens: hash_tokens(&proxy_config.http.auth.tokens),
            },
        },
        https: HttpsConfig {
            enabled: proxy_config.https.enabled,
            port: proxy_config.https.port,
            allowed_hosts: proxy_config.https.allowed_hosts,
            auth: AuthConfig {
                credentials: proxy_config.https.auth.credentials,
                tokens: hash_tokens(&proxy_config.https.auth.tokens),
            },
            tls: TlsConfig {
                cert: proxy_config.https.tls.cert,
                key: proxy_config.https.tls.key,
            },
        },
    }
}

fn hash_tokens(tokens: &[String]) -> Vec<String> {
    tokens.iter().map(|token| to_sha256(token.trim())).collect()
}
