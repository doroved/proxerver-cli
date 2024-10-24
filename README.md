# User-Friendly HTTP and HTTPS (HTTP over TLS) Proxy Server

Create your own HTTP and/or HTTPS (HTTP over TLS) proxy server with one click. Use this proxy to enhance the security of your internet connections or to bypass restrictions.

Available on Linux `x86_64` and `aarch64`. Minimum Ubuntu 22.04.

![proxerver screenshot](screenshot.png)

## How to Install

Just log into your server/VPS terminal and run the command:

```bash
curl -fsSL https://proxerver.pages.dev | bash
```

After installation, be sure to run this command to make proxerver available in the current terminal session:

```bash
export PATH=$PATH:~/.proxerver/bin
```

To update proxerver to the latest version, use the same command that was used for installation.

## Key Features:

- Easy to set up and use.
- Support for HTTP and HTTPS (HTTP over TLS).
- Installation of multiple credentials for authentication.
- Traffic filtering based on hosts.
- Setting a secret token for additional [Proxer Client](https://github.com/doroved/proxer) authentication as a protection against proxy detection.

```
proxerver --help

User Friendly HTTP and HTTPS (HTTP over TLS) proxy server.

Usage: proxerver [OPTIONS]

Options:
      --http-port <u16>   Specify the HTTP port. Default: 8080
      --https-port <u16>  Specify the HTTPS port. Default: 443
      --no-http-server    Disable the HTTP proxy server
      --no-https-server   Disable the HTTPS proxy server
      --auth <string>     Comma-separated list of basic credentials. Example: 'login:password, login2:password2'
      --hosts <string>    Comma-separated list of allowed hosts. Example: 'site.com, *.site.com'
      --token <string>    Secret token to access the HTTP/S proxy server from Proxer Client. The proxy server will only process requests if the client sends an `x-http(s)-secret-token` header with a valid token. Example: mysecrettoken123
      --no-http-token     Disable using the secret token to access the HTTP proxy server from Proxer Client
      --no-https-token    Disable using the secret token to access the HTTPS proxy server from Proxer Client
      --cert <string>     Path to the TLS certificate file. Example: '/path/to/fullchain.(pem|cer|crt|...)'
      --pkey <string>     Path to the TLS private key file. Example: '/path/to/privkey.(pem|key|...)'
  -h, --help              Print help
  -V, --version           Print version
```

## Command Examples to Start the Proxy Server

Starting the HTTP and HTTPS proxy server on ports 8080 and 443 without authentication:

```bash
proxerver --cert cert.crt --pkey private.key
```

Starting the HTTP and HTTPS proxy server on ports 9999 and 8443 with username and password authentication:

```bash
proxerver --http-port 9999 --https-port 8443 --cert cert.crt --pkey private.key --auth login:password
```

Starting the HTTP proxy server (without HTTPS):

```bash
proxerver --no-https-server
```

Starting the HTTP and HTTPS proxy server with multiple credentials authentication and allowing requests only for specific hosts:

```bash
proxerver --cert cert.crt --pkey private.key --hosts '*.example.com,example.com' --auth 'user:pass,user2:pass2'
```

Starting the HTTP and HTTPS proxy server with authentication and setting a secret token for protection against proxy detection. If the [Proxer Client](https://github.com/doroved/proxer) sends a header with an invalid token, the proxy server will respond with a 400 error:

```bash
proxerver --cert cert.crt --pkey private.key --token mysecrettoken123
```

Starting the HTTP and HTTPS proxy server with setting a secret token for protection against proxy detection. Disabling token verification for the HTTPS server:

```bash
proxerver --cert cert.crt --pkey private.key --token mysecrettoken123 --no-https-token
```

To run the proxy server in the background, use nohup, for example:

```bash
nohup proxerver [OPTIONS] >/dev/null 2>&1 &
```

Running the proxy server in the background using nohup and saving the output to a file:

```bash
nohup proxerver [OPTIONS] > ~/.proxerver/log.txt 2>&1 &
```

Remove the background process proxerver:

```bash
kill $(pgrep proxerver)
```

## Local Build via OrbStack

1. Install OrbStack https://orbstack.dev/download and create 2 virtual machines Ubuntu 22.04 x86_64 (amd64) and aarch64 (arm64).
2. On each machine, install rust and all necessary packages for successful program compilation. To do this, run `install_rust.sh` using these commands:

```bash
cd path/to/proxerver
orb -m ubuntu-22.04-amd64 bash install_rust.sh
orb -m ubuntu-22.04-arm64 bash install_rust.sh
```

\* - **ubuntu-22.04-amd/arm64** - this is the name you assign yourself when creating the machine in OrbStack, it may differ from your name.

3. Creating releases. Run `release.sh` and files for different architectures will be created in the `./target/release` folder.

```bash
bash release.sh
```

Or just run in development mode:

```bash
orb -m ubuntu-22.04-amd64 cargo run -- --no-https-server
# OR
ssh ubuntu-22.04-amd64@orb
cd /path/to/proxerver
cargo run -- --no-https-server
```

## Local HTTPS Server Launch

To locally run the HTTPS server, you need to generate a self-signed certificate, add it to the keychain, and start the proxy server:

```bash
cd path/to/proxerver
```

```bash
openssl genpkey -algorithm RSA -out private.key -pkeyopt rsa_keygen_bits:2048
```

```bash
openssl req -new -x509 -key private.key -out cert.crt -days 365 -subj "/C=RU/ST=Moscow/L=Moscow/O=MyOrg/OU=MyUnit/CN=localhost"
```

```bash
cargo run -- --cert cert.crt --pkey private.key --https-port 8443
```

## TODO:

- [ ] Automatic creation and renewal of Let's Encrypt certificates for custom domains
- [ ] Automatic issuance of a public free domain with a Let's Encrypt certificate when creating an HTTPS proxy server
- [ ] GeoIP whitelist with caching for access to the proxy server. Cache IPs and compare network inclusion rather than exact match.
- [ ] Daemonization of the process to run the program in the background.
