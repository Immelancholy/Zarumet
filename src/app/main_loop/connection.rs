use mpd_client::Client;
use tokio::net::TcpStream;

#[cfg(unix)]
use tokio::net::UnixStream;

pub async fn connect_to_mpd(
    address: &str,
) -> color_eyre::Result<(Client, mpd_client::client::ConnectionEvents)> {
    let is_unix_socket = address.contains('/');

    if is_unix_socket {
        #[cfg(unix)]
        {
            let connection = UnixStream::connect(address).await?;
            Ok(Client::connect(connection).await?)
        }
        #[cfg(not(unix))]
        {
            Err(color_eyre::eyre::eyre!(
                "Unix sockets are not supported on this platform"
            ))
        }
    } else {
        let connection = TcpStream::connect(address).await?;
        Ok(Client::connect(connection).await?)
    }
}
