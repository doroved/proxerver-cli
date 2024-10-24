use base64::{engine::general_purpose::STANDARD as b64, Engine};
use chrono::Local;
use hyper::{header::PROXY_AUTHENTICATE, Body, Response, StatusCode};
use lazy_static::lazy_static;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::net::{IpAddr, SocketAddr};
use std::process::Command;
use std::sync::Mutex;
use wildmatch::WildMatch;

lazy_static! {
    pub static ref SERVER_IP: Mutex<String> = Mutex::new("0.0.0.0".to_string());
}

pub fn get_rand_ipv4_socket_addr() -> SocketAddr {
    let mut rng = rand::thread_rng();
    let server_ip_addr = get_current_server_ip().parse::<IpAddr>().unwrap();

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
    for allowed_host in allowed_hosts {
        if WildMatch::new(allowed_host.as_str()).matches(req_host) {
            return true;
        }
    }
    false
}

pub fn is_allowed_credentials(credentials_header: &str, allowed_credentials: Vec<String>) -> bool {
    for credentials in allowed_credentials {
        let allowed_credentials = b64.encode(credentials);

        if credentials_header.contains(&allowed_credentials) {
            return true;
        }
    }
    false
}

pub async fn get_server_ip() -> String {
    let output = Command::new("sh")
        .arg("-c")
        .arg("hostname -I | awk '{print $1}'")
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        if output.stdout.is_empty() {
            return "0.0.0.0".to_string();
        } else {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    } else {
        panic!("Failed to get Server IP: {:?}", output.status);
    }
}

pub async fn update_server_ip() {
    let server_ip = get_server_ip().await;

    let mut ip = SERVER_IP.lock().unwrap();
    *ip = server_ip;
}

pub fn get_current_server_ip() -> String {
    SERVER_IP.lock().unwrap().clone()
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
