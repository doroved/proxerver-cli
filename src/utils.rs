use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::Command;

use base64::{engine::general_purpose::STANDARD as b64, Engine};
use chrono::Local;
use hyper::{header::PROXY_AUTHENTICATE, Body, Response, StatusCode};
use rand::Rng;
use sha2::{Digest, Sha256};
use wildmatch::WildMatch;

pub fn get_rand_ipv4_socket_addr(server_ip_addr: IpAddr) -> SocketAddr {
    let mut rng = rand::thread_rng();
    SocketAddr::new(server_ip_addr, rng.gen::<u16>())
}

pub fn require_basic_auth() -> Response<Body> {
    Response::builder()
        .status(StatusCode::PROXY_AUTHENTICATION_REQUIRED)
        .header(PROXY_AUTHENTICATE, "Basic realm=\"proxerver\"")
        .body(Body::empty())
        .unwrap()
}

pub fn create_basic_auth_response() -> Vec<u8> {
    let status = StatusCode::PROXY_AUTHENTICATION_REQUIRED;
    let response = format!(
        "HTTP/1.1 {} {}\r\n\
         Proxy-Authenticate: Basic realm=\"proxerver\"\r\n\
         Content-Length: 0\r\n\
         \r\n",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown")
    );
    response.into_bytes()
}

pub fn is_host_allowed(req_host: &str, allowed_hosts: &[String]) -> bool {
    for host in allowed_hosts {
        if WildMatch::new(host.as_str()).matches(req_host) {
            return true;
        }
    }
    false
}

pub fn is_credentials_allowed(credentials_header: &str, credentials_allowed: &[String]) -> bool {
    for credentials in credentials_allowed {
        let credentials_allowed = b64.encode(credentials);

        if credentials_header.contains(&credentials_allowed) {
            return true;
        }
    }
    false
}

pub async fn get_server_ip() -> IpAddr {
    let output = Command::new("sh")
        .arg("-c")
        .arg("hostname -I | awk '{print $1}'")
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        if output.stdout.is_empty() {
            IpAddr::V4(Ipv4Addr::UNSPECIFIED)
        } else {
            if output.stdout.is_empty() {
                return IpAddr::V4(Ipv4Addr::UNSPECIFIED);
            }

            match String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<IpAddr>()
            {
                Ok(ip) => ip,
                Err(e) => panic!("Failed to parse IP address: {}", e),
            }
        }
    } else {
        panic!("Failed to get Server IP: {:?}", output.status);
    }
}

pub fn to_sha256(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);

    let result = hasher.finalize();
    format!("{:x}", result)
}

pub fn formatted_time() -> String {
    let now = Local::now();
    now.format("%Y-%m-%d %H:%M:%S").to_string()
}
