use crate::{
    options::Opt,
    utils::{
        formatted_time, get_rand_ipv4_socket_addr, is_credentials_allowed, is_host_allowed,
        require_basic_auth, to_sha256,
    },
};

use clap::Parser;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};

use hyper::{
    client::HttpConnector,
    header::PROXY_AUTHORIZATION,
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Client, Method, Request, Response, Server, StatusCode,
};

use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpSocket,
};

#[derive(Debug, Clone)]
pub(crate) struct Proxy {
    pub allowed_credentials: Vec<String>,
    pub allowed_hosts: Vec<String>,
    pub secret_token: String,
}

impl Proxy {
    pub(crate) async fn proxy(
        self,
        req: Request<Body>,
        server_ip: IpAddr,
    ) -> Result<Response<Body>, hyper::Error> {
        println!("Method: {:?}", req.method());
        println!("URI: {:?}", req.uri());
        println!("Version: {:?}", req.version());
        println!("Headers: {:?}", req.headers());
        println!("Body: {:?}", req.body());

        // Check request for inclusion in the white list of hosts that can be proxied
        if let Err(response) = self.check_allowed_hosts(&req).await {
            return Ok(response);
        }

        // If secret token is not empty and no_http_token is false, check if the secret token is valid
        if let Err(response) = self.check_secret_token(&req).await {
            return Ok(response);
        }

        // Process authentication if a list of login:password pairs is specified
        if let Err(response) = self.check_credentials(&req).await {
            return Ok(response);
        }

        // Process method and call the appropriate handler
        match req.method() {
            &Method::CONNECT => self.process_connect(req, server_ip).await,
            _ => self.process_request(req, server_ip).await,
        }
    }

    async fn check_allowed_hosts(&self, req: &Request<Body>) -> Result<(), Response<Body>> {
        let host = req.uri().host().unwrap_or("");
        if !self.allowed_hosts.is_empty() && !is_host_allowed(host, &self.allowed_hosts) {
            return Err(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::empty())
                .unwrap());
        }
        Ok(())
    }

    async fn check_secret_token(&self, req: &Request<Body>) -> Result<(), Response<Body>> {
        let options = Opt::parse();

        if !self.secret_token.is_empty() && !options.no_http_token {
            if let Some(secret_token_header) = req.headers().get("x-http-secret-token") {
                if secret_token_header.to_str().unwrap_or_default().trim()
                    != to_sha256(self.secret_token.trim())
                {
                    return Err(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::empty())
                        .unwrap());
                }
            } else if req.headers().get("x-https-secret-token").is_none() {
                return Err(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::empty())
                    .unwrap());
            }
        }
        Ok(())
    }

    async fn check_credentials(&self, req: &Request<Body>) -> Result<(), Response<Body>> {
        if !self.allowed_credentials.is_empty() {
            if let Some(auth_header) = req.headers().get(PROXY_AUTHORIZATION) {
                let header_credentials = auth_header.to_str().unwrap_or_default();
                if !is_credentials_allowed(header_credentials, &self.allowed_credentials) {
                    return Err(require_basic_auth());
                }
            } else {
                return Err(require_basic_auth());
            }
        }
        Ok(())
    }

    async fn process_connect(
        self,
        req: Request<Body>,
        server_ip: IpAddr,
    ) -> Result<Response<Body>, hyper::Error> {
        tokio::task::spawn(async move {
            let remote_addr = req.uri().authority().map(|auth| auth.to_string()).unwrap();
            let mut upgraded = hyper::upgrade::on(req).await.unwrap();

            self.tunnel(&mut upgraded, remote_addr, server_ip).await
        });

        Ok(Response::new(Body::empty()))
    }

    async fn process_request(
        self,
        req: Request<Body>,
        server_ip: IpAddr,
    ) -> Result<Response<Body>, hyper::Error> {
        let mut http = HttpConnector::new();
        http.set_local_address(Some(server_ip));

        let client = Client::builder()
            .http1_title_case_headers(true)
            .http1_preserve_header_case(true)
            .build(http);
        let res = client.request(req).await?;

        Ok(res)
    }

    async fn tunnel<A>(
        self,
        upgraded: &mut A,
        addr_str: String,
        server_ip: IpAddr,
    ) -> std::io::Result<()>
    where
        A: AsyncRead + AsyncWrite + Unpin + ?Sized,
    {
        if let Ok(addrs) = addr_str.to_socket_addrs() {
            for addr in addrs {
                let socket = TcpSocket::new_v4()?;
                let bind_addr = get_rand_ipv4_socket_addr(server_ip);

                if socket.bind(bind_addr).is_ok() {
                    if let Ok(mut server) = socket.connect(addr).await {
                        tokio::io::copy_bidirectional(upgraded, &mut server).await?;
                        return Ok(());
                    }
                }
            }
        } else {
            println!("error: {addr_str}");
        }

        Ok(())
    }
}

pub async fn start_proxy(
    listen_addr: SocketAddr,
    allowed_credentials: Vec<String>,
    allowed_hosts: Vec<String>,
    secret_token: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let proxy = Proxy {
        allowed_credentials,
        allowed_hosts,
        secret_token,
    };

    let make_service = make_service_fn(move |addr: &AddrStream| {
        let server_ip = listen_addr.ip();
        let proxy_clone = proxy.clone();
        let time = formatted_time();

        println!(
            "\n\x1b[1m[{time}] [HTTP server] New connection from: {}\x1b[0m",
            addr.remote_addr()
        );

        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                proxy_clone.clone().proxy(req, server_ip)
            }))
        }
    });

    Server::bind(&listen_addr)
        .http1_preserve_header_case(true)
        .http1_title_case_headers(true)
        .serve(make_service)
        .await
        .map_err(Into::into)
}
