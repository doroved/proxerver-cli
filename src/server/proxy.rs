use std::net::IpAddr;
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD as b64, Engine};
use bytes::Bytes;
use http::header::{PROXY_AUTHENTICATE, PROXY_AUTHORIZATION};
use http::{Method, Request, Response};
use http_body_util::{combinators::BoxBody, Empty};
use http_body_util::{BodyExt, Full};
use hyper::client::conn::http1::Builder;
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use wildmatch::WildMatch;

use crate::server::tunnel::tunnel_direct;

pub async fn handle_request(
    req: Request<hyper::body::Incoming>,
    client_ip: IpAddr,
    allowed_hosts: Arc<Vec<String>>,
    auth_credentials: Arc<Vec<String>>,
    auth_tokens: Arc<Vec<String>>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    tracing::info!("\x1B[34m{client_ip}\x1B[0m → {req:?}");

    let host = match req.uri().host() {
        Some(host) => host,
        None => {
            tracing::error!("\x1B[31mURI has no host\x1B[0m");
            return Ok(bad_request_response());
        }
    };

    let headers = req.headers();

    // Check allowed hosts
    if let Some(resp) = check_allowed_hosts(host, &allowed_hosts).await {
        return Ok(resp);
    }

    // Check auth credentials
    if let Some(resp) = check_auth_credentials(headers, &auth_credentials).await {
        return Ok(resp);
    }

    // Check auth tokens
    if let Some(resp) = check_auth_tokens(headers, &auth_tokens).await {
        return Ok(resp);
    }

    // HTTPS request
    if Method::CONNECT == req.method() {
        if let Some(addr) = host_addr(req.uri()) {
            tokio::spawn(async move {
                match hyper::upgrade::on(req).await {
                    Ok(upgraded) => {
                        if let Err(e) = tunnel_direct(upgraded, &addr).await {
                            tracing::error!("\x1B[31m{addr} → TUNNEL connection error: {e}\x1B[0m");
                        }
                    }
                    Err(e) => tracing::error!("\x1B[31m{addr} → UPGRADE error: {e}\x1B[0m"),
                }
            });

            Ok(Response::new(empty()))
        } else {
            tracing::error!(
                "\x1B[31mCONNECT host is not socket addr: {:?}\x1B[0m",
                req.uri()
            );
            Ok(bad_request_response())
        }
    } else {
        // HTTP request
        tracing::info!("{:?} {:?} → HTTP connection", req.method(), req.uri());

        let port = req.uri().port_u16().unwrap_or(80);

        match TcpStream::connect((host, port)).await {
            Ok(stream) => {
                let (mut sender, conn) = Builder::new()
                    .preserve_header_case(true)
                    .title_case_headers(true)
                    .handshake(TokioIo::new(stream))
                    .await?;

                tokio::spawn(async move {
                    if let Err(err) = conn.await {
                        tracing::error!("\x1B[31mConnection failed: {:?}\x1B[0m", err);
                    }
                });

                let resp = sender.send_request(req).await?;
                Ok(resp.map(|b| b.boxed()))
            }
            Err(e) => {
                tracing::error!("\x1B[31m{host}:{port} → Failed to connect: {:?}\x1B[0m", e);
                Ok(bad_request_response())
            }
        }
    }
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

fn _full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

fn bad_request_response() -> Response<BoxBody<Bytes, hyper::Error>> {
    let mut resp = Response::new(empty());
    *resp.status_mut() = http::StatusCode::BAD_REQUEST;
    resp
}

fn require_basic_auth() -> Response<BoxBody<Bytes, hyper::Error>> {
    let mut resp = Response::new(empty());
    *resp.status_mut() = http::StatusCode::PROXY_AUTHENTICATION_REQUIRED;
    resp.headers_mut().insert(
        PROXY_AUTHENTICATE,
        http::HeaderValue::from_static("Basic realm=\"proxerver-cli\""),
    );
    resp
}

fn host_addr(uri: &http::Uri) -> Option<String> {
    uri.authority().map(|auth| auth.to_string())
}

fn is_host_allowed(req_host: &str, allowed_hosts: &[String]) -> bool {
    for allowed_host in allowed_hosts {
        if WildMatch::new(allowed_host).matches(req_host) {
            return true;
        }
    }
    false
}

fn is_credentials_allowed(credentials_header: &str, credentials_allowed: &[String]) -> bool {
    for credentials in credentials_allowed {
        let credentials_allowed = b64.encode(credentials);

        if credentials_header.contains(&credentials_allowed) {
            return true;
        }
    }
    false
}

async fn check_allowed_hosts(
    host: &str,
    allowed_hosts: &[String],
) -> Option<Response<BoxBody<Bytes, hyper::Error>>> {
    if !allowed_hosts.is_empty() && !is_host_allowed(host, allowed_hosts) {
        tracing::error!("\x1B[31m{host} not allowed\x1B[0m");
        return Some(bad_request_response());
    }
    None
}

async fn check_auth_credentials(
    headers: &http::HeaderMap,
    auth_credentials: &[String],
) -> Option<Response<BoxBody<Bytes, hyper::Error>>> {
    if !auth_credentials.is_empty() {
        if let Some(auth_credentials_header) = headers.get(PROXY_AUTHORIZATION) {
            let auth_credentials_header = auth_credentials_header.to_str().unwrap_or("");

            if !is_credentials_allowed(auth_credentials_header, auth_credentials) {
                tracing::error!(
                    "\x1B[31mNot allowed auth credentials: {auth_credentials_header}\x1B[0m"
                );
                return Some(require_basic_auth());
            }
        } else {
            tracing::error!("\x1B[31mNo auth credentials provided\x1B[0m");
            return Some(require_basic_auth());
        }
    }

    None
}

async fn check_auth_tokens(
    headers: &http::HeaderMap,
    auth_tokens: &[String],
) -> Option<Response<BoxBody<Bytes, hyper::Error>>> {
    if !auth_tokens.is_empty() {
        if let Some(auth_token_header) = headers.get("x-auth-token") {
            let auth_token_header = auth_token_header.to_str().unwrap_or("");

            if !auth_tokens.contains(&auth_token_header.to_string()) {
                tracing::error!("\x1B[31mNot allowed auth token: {auth_token_header}\x1B[0m");
                return Some(bad_request_response());
            }
        } else {
            tracing::error!("\x1B[31mNo auth token provided\x1B[0m");
            return Some(bad_request_response());
        }
    }

    None
}
