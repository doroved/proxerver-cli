use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Error as IoError, ErrorKind};
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::sync::Arc;

use clap::Parser;
use hyper::header::{HeaderName, HeaderValue};
use hyper::{Body, StatusCode};
use hyper_tls::HttpsConnector;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::read_one;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio_rustls::TlsAcceptor;

use crate::options::Opt;
use crate::utils::{
    create_basic_auth_response, formatted_time, is_allowed_credentials, is_host_allowed, to_sha256,
};

use hyper::http::HeaderMap;
use hyper::{Client, Request as HttpRequest};
// use hyper_tls::HttpsConnector;

async fn tunnel_to_remote<A>(upgraded: &mut A, addr: String) -> std::io::Result<()>
where
    A: AsyncRead + AsyncWrite + Unpin + ?Sized,
{
    if let Ok(addrs) = addr.to_socket_addrs() {
        for addr in addrs {
            let socket = TcpSocket::new_v4()?;
            if let Ok(mut server) = socket.connect(addr).await {
                tokio::io::copy_bidirectional(upgraded, &mut server).await?;
                return Ok(());
            }
        }
    }
    eprintln!("Failed to connect to {addr}");
    Ok(())
}

fn load_certs(filename: &str) -> std::io::Result<Vec<Certificate>> {
    let cert_file = &mut BufReader::new(File::open(filename)?);
    let certs: Vec<Certificate> = rustls_pemfile::certs(cert_file)
        .filter_map(|item| item.ok())
        .map(|cert| Certificate(cert.to_vec()))
        .collect();

    if certs.is_empty() {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "No valid certs found",
        ));
    }
    Ok(certs)
}

fn load_private_key(filename: &str) -> std::io::Result<PrivateKey> {
    let key_file = &mut BufReader::new(File::open(filename)?);
    let mut keys: Vec<PrivateKey> = Vec::new();

    while let Ok(Some(item)) = read_one(key_file) {
        match item {
            rustls_pemfile::Item::Pkcs8Key(key) => {
                keys.push(PrivateKey(key.secret_pkcs8_der().to_vec()));
            }
            rustls_pemfile::Item::Pkcs1Key(key) => {
                keys.push(PrivateKey(key.secret_pkcs1_der().to_vec()));
            }
            rustls_pemfile::Item::Sec1Key(key) => {
                keys.push(PrivateKey(key.secret_sec1_der().to_vec()));
            }
            _ => continue, // Ignore other key types and items
        }
    }

    if keys.is_empty() {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "No valid private key found",
        ));
    }

    Ok(keys.remove(0))
}

fn create_server_config(certs: Vec<Certificate>, key: PrivateKey) -> Result<ServerConfig, IoError> {
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|err| IoError::new(ErrorKind::InvalidInput, err))?;

    Ok(config)
}

pub async fn start_proxy(
    listen_addr: SocketAddr,
    allowed_credentials: Vec<String>,
    allowed_hosts: Vec<String>,
    secret_token: String,
    cert_file_path: String,
    key_file_path: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let certs = load_certs(&cert_file_path)?;
    let key = load_private_key(&key_file_path)?;

    let config = create_server_config(certs, key)?;

    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind(listen_addr).await?;

    loop {
        let (stream, addr) = listener.accept().await?;
        let acceptor = acceptor.clone();

        let allowed_credentials = allowed_credentials.clone();
        let allowed_hosts = allowed_hosts.clone();
        let secret_token = secret_token.clone();

        tokio::spawn(async move {
            let mut stream = match acceptor.accept(stream).await {
                Ok(s) => s,
                Err(_) => return, // Обработка ошибок TLS
            };

            let mut buffer = vec![0; 1024];
            match stream.read(&mut buffer).await {
                Ok(n) => {
                    let request = String::from_utf8_lossy(&buffer[..n]);

                    let options = Opt::parse();

                    match parse_request(&request) {
                        Ok((method, uri, version, headers)) => {
                            let time = formatted_time();

                            println!("\n\x1b[38;5;28m\x1b[1m[{time}] [HTTPS server] New connection from: {}\x1b[0m", addr);

                            println!("Method: {}", method);
                            println!("URI: {}", uri);
                            println!("Version: {}", version);
                            println!("Headers: {:?}", headers);

                            // Check request for inclusion in the white list of hosts that can be proxied
                            // let host = headers.get("host").unwrap().split(':').next().unwrap_or("");
                            let host = headers
                                .get("host")
                                .and_then(|h| h.split(':').next())
                                .unwrap_or("");
                            if !allowed_hosts.is_empty() && !is_host_allowed(host, &allowed_hosts) {
                                let error_response = create_error_response(StatusCode::BAD_REQUEST);
                                if let Err(e) = stream.write_all(&error_response).await {
                                    eprintln!("Failed to write error response to client: {:?}", e);
                                }
                                return;
                            }

                            // If secret token is not empty and no_http_token is false, check if the secret token is valid
                            if !secret_token.is_empty() && !options.no_https_token {
                                if let Some(secret_token_header) =
                                    headers.get("x-https-secret-token")
                                {
                                    if secret_token_header.trim() != to_sha256(secret_token.trim())
                                    {
                                        let error_response =
                                            create_error_response(StatusCode::BAD_REQUEST);

                                        if let Err(e) = stream.write_all(&error_response).await {
                                            eprintln!(
                                                "Failed to write error response to client: {:?}",
                                                e
                                            );
                                        }
                                        return;
                                    }
                                } else if !headers.contains_key("x-http-secret-token") {
                                    let error_response =
                                        create_error_response(StatusCode::BAD_REQUEST);

                                    if let Err(e) = stream.write_all(&error_response).await {
                                        eprintln!(
                                            "Failed to write error response to client: {:?}",
                                            e
                                        );
                                    }
                                    return;
                                }
                            }

                            // Process authentication if a list of login:password pairs is specified
                            if !allowed_credentials.is_empty() {
                                if let Some(header_credentials) = headers.get("proxy-authorization")
                                {
                                    if !is_allowed_credentials(
                                        header_credentials,
                                        allowed_credentials,
                                    ) {
                                        let auth_response = create_basic_auth_response();
                                        if let Err(e) = stream.write_all(&auth_response).await {
                                            eprintln!("Failed to write authentication response to client: {:?}", e);
                                        }
                                        return;
                                    }
                                } else {
                                    let auth_response = create_basic_auth_response();
                                    if let Err(e) = stream.write_all(&auth_response).await {
                                        eprintln!("Failed to write authentication response to client: {:?}", e);
                                    }
                                    return;
                                }
                            }
                        }
                        Err(err) => {
                            println!("Error parsing request: {}", err);
                        }
                    }

                    // Process request method and call the appropriate handler
                    if request.starts_with("CONNECT") {
                        // Process CONNECT request
                        let parts: Vec<&str> = request.split_whitespace().collect();
                        if parts.len() >= 2 {
                            let remote_addr = parts[1].to_string();

                            // Send confirmation of connection setup
                            let response = "HTTP/1.1 200 Connection Established\r\n\r\n";
                            if let Err(e) = stream.write_all(response.as_bytes()).await {
                                eprintln!("Failed to write response to client {}: {:?}", addr, e);
                                return;
                            }

                            // Create a tunnel
                            if let Err(e) = tunnel_to_remote(&mut stream, remote_addr).await {
                                eprintln!("Tunneling error for {}: {:?}", addr, e);
                            }
                        } else {
                            eprintln!("Invalid CONNECT request from {}", addr);
                        }
                    } else {
                        // Process regular HTTP requests
                        handle_http_request(stream, request.to_string()).await;
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from client {}: {:?}", addr, e);
                }
            }

            // println!("Connection closed: {}", addr);
        });
    }
    // Ok(())
}

fn create_error_response(status_code: StatusCode) -> Vec<u8> {
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: 0\r\n\r\n",
        status_code.as_u16(),
        status_code.canonical_reason().unwrap_or("Unknown")
    );
    response.into_bytes()
}

// Process regular HTTP requests
async fn handle_http_request(
    mut stream: tokio_rustls::server::TlsStream<TcpStream>,
    request: String,
) {
    match parse_request(&request) {
        Ok((method, uri, _, headers)) => {
            // Create a HTTPS client
            let https = HttpsConnector::new();
            let client = Client::builder().build::<_, hyper::Body>(https);

            // Create a new HTTP request
            let mut http_request = HttpRequest::builder()
                .method(method.as_str())
                .uri(uri)
                .body(Body::empty())
                .expect("Failed to build request");

            // Add the headers from the original request
            *http_request.headers_mut() = hash_map_to_header_map(headers.clone());

            // Send the request to the final server
            match client.request(http_request).await {
                Ok(response) => {
                    // Отправляем ответ обратно клиенту
                    let status = response.status();
                    let response_body = hyper::body::to_bytes(response.into_body()).await.unwrap();

                    let response_header = format!("HTTP/1.1 {}\r\n", status);
                    let response_length = response_body.len();
                    let response = format!(
                        "{}Content-Length: {}\r\n\r\n",
                        response_header, response_length
                    );
                    let full_response = [response.into_bytes(), response_body.to_vec()].concat();

                    // Отправка полного ответа обратно клиенту
                    if let Err(e) = stream.write_all(&full_response).await {
                        eprintln!("Failed to write response to client: {:?}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Error while forwarding request: {:?}", e);
                    // Можно отправить ошибку клиенту
                }
            }
        }
        Err(err) => {
            println!("Error parsing HTTP request: {}", err);
        }
    }
}

type RequestInfo = (String, String, String, HashMap<String, String>);

fn parse_request(request: &str) -> Result<RequestInfo, &'static str> {
    let mut lines = request.lines();

    // Получаем первую строку с методом, URI и версией
    let request_line = lines.next().ok_or("Missing request line")?;
    let mut request_parts = request_line.split_whitespace();

    let method = request_parts.next().ok_or("Missing method")?.to_string();
    let uri = request_parts.next().ok_or("Missing URI")?.to_string();
    let version = request_parts.next().ok_or("Missing version")?.to_string();

    // Инициализируем хэш-карту для заголовков
    let mut headers = HashMap::new();

    // Обрабатываем остальные строки как заголовки
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue; // Пропускаем пустые строки
        }
        let (key, value) = parse_header(line)?;
        headers.insert(key.to_string().to_lowercase(), value.to_string());
    }

    Ok((method, uri, version, headers))
}

fn parse_header(line: &str) -> Result<(String, String), &'static str> {
    let mut parts = line.splitn(2, ':');
    let key = parts.next().ok_or("Missing header key")?.trim();
    let value = parts.next().ok_or("Missing header value")?.trim();

    Ok((key.to_string(), value.to_string()))
}

fn hash_map_to_header_map(headers: HashMap<String, String>) -> HeaderMap {
    let mut header_map = HeaderMap::new();

    for (key, value) in headers {
        // Parse the key into a HeaderName
        let header_name: HeaderName = key.parse().expect("Invalid header name");

        // Parse the value into a HeaderValue
        let header_value: HeaderValue = value.parse().expect("Invalid header value");

        // Insert into the HeaderMap
        header_map.insert(header_name, header_value);
    }

    header_map
}
