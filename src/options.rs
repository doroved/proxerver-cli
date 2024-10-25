use clap::Parser;
use std::process::exit;

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Opt {
    #[clap(
        long,
        value_name = "u16",
        help = "Specify the HTTP port. Default: 58080"
    )]
    pub http_port: Option<u16>,

    #[clap(
        long,
        value_name = "u16",
        help = "Specify the HTTPS port. Default: 443"
    )]
    pub https_port: Option<u16>,

    #[clap(
        long,
        default_value_t = false,
        conflicts_with = "no_https_server",
        help = "Disable the HTTP proxy server"
    )]
    pub no_http_server: bool,

    #[clap(
        long,
        default_value_t = false,
        conflicts_with = "no_http_server",
        help = "Disable the HTTPS proxy server"
    )]
    pub no_https_server: bool,

    #[clap(
        long,
        value_name = "string",
        help = "Comma-separated list of basic credentials. Example: 'login:password, login2:password2'"
    )]
    pub auth: Option<String>,

    #[clap(
        long,
        value_name = "string",
        help = "Comma-separated list of allowed hosts. Example: 'site.com, *.site.com'"
    )]
    pub hosts: Option<String>,

    #[clap(
        long,
        value_name = "string",
        help = "Secret token to access the HTTP/S proxy server from Proxer Client. The proxy server will only process requests if the client sends an `x-http(s)-secret-token` header with a valid token. Example: mysecrettoken123"
    )]
    pub token: Option<String>,

    #[clap(
        long,
        default_value_t = false,
        conflicts_with = "no_http_server",
        help = "Disable using the secret token to access the HTTP proxy server from Proxer Client"
    )]
    pub no_http_token: bool,

    #[clap(
        long,
        default_value_t = false,
        conflicts_with = "no_https_server",
        help = "Disable using the secret token to access the HTTPS proxy server from Proxer Client"
    )]
    pub no_https_token: bool,

    #[clap(
        long,
        help = "Path to the TLS certificate file. Example: '/path/to/fullchain.(pem|cer|crt|...)'",
        value_name = "string",
        required_unless_present("no_https_server")
    )]
    pub cert: Option<String>,

    #[clap(
        long,
        help = "Path to the TLS private key file. Example: '/path/to/privkey.(pem|key|...)'",
        value_name = "string",
        required_unless_present("no_https_server")
    )]
    pub pkey: Option<String>,
}

impl Opt {
    pub fn validate(&self) {
        if self.no_https_server && (self.cert.is_some() || self.pkey.is_some()) {
            eprintln!("Error: --cert or --pkey cannot be used with --no-https");
            exit(1);
        }
    }
}
