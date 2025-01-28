# Proxerver

> **Subscribe to us on [Telegram](https://t.me/macproxer) to receive notifications about new versions and updates.**

Create your own HTTP and/or HTTPS (HTTP over TLS) proxy server with one click. Use this proxy to enhance the security of your internet connections or to bypass restrictions.

Available on Linux `x86_64` and ~~`aarch64`~~ (temporarily unavailable in installer).

![proxerver screenshot](screenshot.png)

## How to Install

Just log into your server/VPS terminal and run the command:

```bash
curl -fsSL https://proxerver-cli.pages.dev | bash
```

After installation, be sure to run this command to make proxerver available in the current terminal session:

```bash
export PATH=$PATH:~/.proxerver-cli/bin
```

To update proxerver to the latest version, use the same command that was used for installation.

## Key Features:

- Easy to set up and use.
- Support for HTTP and HTTPS (HTTP over TLS).
- List of allowed hosts that can be proxied. Use [wildcard matching](#wildcard-matching-usage-rules).
- Using multiple credentials for authentication.
- Authentication via tokens supported by the [Proxer CLI](https://github.com/doroved/proxer-cli) client. It is recommended for use as it helps protect your proxies from proxy server detection through active probing.

```
proxerver-cli --help

User Friendly HTTP and HTTPS (HTTP over TLS) proxy server.

Usage: proxerver-cli [OPTIONS]

Options:
      --config <PATH>  Path to the configuration file. Default: '~/.proxerver-cli/config.toml'
  -h, --help           Print help
  -V, --version        Print version
```

## Configuration File

A configuration file is used to store the proxy server settings. The default configuration file is located at `~/.proxerver-cli/config.toml`. You can specify a different path using the `--config` option.

To change the default configuration file, use the following command:

```bash
nano ~/.proxerver-cli/config.toml
```

```toml
# HTTP server configuration
[http]
enabled = true
port = 8080
allowed_hosts = []

[http.auth]
credentials = []
tokens = []

# HTTPS server configuration
[https]
enabled = false
port = 443
allowed_hosts = []

[https.auth]
credentials = []
tokens = []

[https.tls]
cert = ""
key = ""
```

The configuration file is in TOML format and contains the following sections:

- `http`: HTTP server configuration
- `https`: HTTPS server configuration

### HTTP Server Configuration

- `enabled`: Enables or disables the HTTP server. Default: `true`
- `port`: The port to listen on. Default: `8080`
- `allowed_hosts`: A list of allowed hosts. Default: `[]`
- `auth`: Authentication configuration
  - `credentials`: A list of allowed credentials. Default: `[]`
  - `tokens`: A list of allowed tokens. Default: `[]`

### HTTPS Server Configuration

- `enabled`: Enables or disables the HTTPS server. Default: `false`
- `port`: The port to listen on. Default: `443`
- `allowed_hosts` and `auth`: Same as HTTP server configuration
- `tls`: TLS configuration
  - `cert`: Path to the certificate file. Default: `""`
  - `key`: Path to the private key file. Default: `""`

See example configuration files in the [`config.example.toml`](./config.example.toml) file for more details.

## Starting the Proxy Server

To quickly start the HTTP proxy server on port 8080, use the following command:

```bash
proxerver-cli
```

Create configuration files with the parameters you need and start the proxy server using the command:

```bash
proxerver-cli --config custom.toml
```

To run the proxy server in the background, use nohup, for example:

```bash
nohup proxerver-cli [OPTIONS] >/dev/null 2>&1 &
```

Running the proxy server in the background using nohup and saving the output to a file:

```bash
nohup proxerver-cli [OPTIONS] > ~/.proxerver-cli/log.txt 2>&1 &
```

Remove the background process proxerver-cli:

```bash
pkill proxerver-cli
```

## Let's Encrypt Certificate Generation

Install certbot, test automatic renewal, and check for a timer for automatic certificate updates.

```bash
apt-get install certbot -y
certbot certonly --standalone --agree-tos --register-unsafely-without-email -d yourdomain.com
certbot renew --dry-run
systemctl list-timers | grep certbot
```

`yourdomain.com` - Your domain pointing to the server's IP

## Wildcard Matching Usage Rules

1. Use `*` to replace any number of characters. For example, `*.example.com` will match all subdomains of `example.com`.
2. Use `?` to replace a single character. For example, `*.example?.com` will match `*.example1.com`, `*.exampleA.com`, but not `*.example.com`.
3. You can combine `*` and `?` for more complex patterns. For example, `*example?.com` will match `example1.com`, `myexampleA.com`, but not `example.com`.
4. Be cautious when using wildcard matching, as it can lead to unwanted access if not configured properly.
5. Check allowed hosts for duplicates to avoid conflicts in rules.

## TODO:

- [ ] Automatic creation and renewal of Let's Encrypt certificates for custom domains and [IP](https://letsencrypt.org/2025/01/16/6-day-and-ip-certs/).
- [ ] Daemonization of the process to run the program in the background.
