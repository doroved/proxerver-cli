use std::time::Duration;

use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;

use tokio::{net::TcpStream, time::timeout};

pub async fn tunnel_direct(
    client_connection: Upgraded,
    addr: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("{addr} → TUNNEL connection");

    let mut remote_server = timeout(Duration::from_secs(30), TcpStream::connect(&addr)).await??;

    timeout(
        Duration::from_secs(600),
        tokio::io::copy_bidirectional(&mut TokioIo::new(client_connection), &mut remote_server),
    )
    .await??;

    Ok(())
}
