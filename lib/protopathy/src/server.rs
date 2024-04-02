// This is free and unencumbered software released into the public domain.

use std::sync::Arc;
use tonic::transport::server::RoutesBuilder;
use url::Url;

pub struct Server {
    pub socket_url: Url,
    pub stop_trigger: Arc<triggered::Trigger>,
    pub stop_listener: triggered::Listener,
    pub routes: RoutesBuilder,
}

impl Server {
    pub fn new(socket_url: Url) -> Self {
        let (trigger, listener) = triggered::trigger();
        Self {
            socket_url,
            stop_trigger: Arc::new(trigger),
            stop_listener: listener,
            routes: RoutesBuilder::default(),
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        match self.socket_url.scheme() {
            #[cfg(feature = "tcp")]
            "tcp" => self.start_tcp_socket(),
            #[cfg(unix)]
            "file" => self.start_file_socket(),
            #[cfg(unix)]
            "fd" => self.start_fd_socket(),
            _ => panic!("unsupported scheme in socket URL"), // FIXME
        }
    }

    #[cfg(feature = "tcp")]
    #[tokio::main]
    /// Binds to a TCP socket using an IPv4/IPv6 address and port
    /// number (e.g., "tcp://[::1]:50051").
    async fn start_tcp_socket(&self) -> Result<(), Box<dyn std::error::Error>> {
        use std::net::SocketAddr;

        let socket_str = format!(
            "{}:{}",
            self.socket_url.host_str().expect("hostname in socket URL"),
            self.socket_url.port().expect("port in socket URL")
        );
        let socket_addr: SocketAddr = socket_str.parse().unwrap();

        let routes = self.routes.clone();

        tonic::transport::Server::builder()
            .add_routes(routes.routes())
            .serve_with_shutdown(socket_addr, self.stop_listener.clone())
            .await?;

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::main]
    /// Binds to a Unix domain socket using a local file path
    /// (e.g., "file:/run/myserver.sock").
    async fn start_file_socket(&self) -> Result<(), Box<dyn std::error::Error>> {
        use tokio::net::UnixListener;
        use tokio_stream::wrappers::UnixListenerStream;

        let socket_path = self.socket_url.to_file_path().unwrap();

        std::fs::create_dir_all(&socket_path.parent().unwrap())?;
        std::fs::remove_file(&socket_path).ok();

        let listener = UnixListener::bind(socket_path)?;
        let listener_stream = UnixListenerStream::new(listener);

        let routes = self.routes.clone();

        tonic::transport::Server::builder()
            .add_routes(routes.routes())
            .serve_with_incoming_shutdown(listener_stream, self.stop_listener.clone())
            .await?;

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::main]
    /// Binds to a byte stream socket using a process-specific file
    /// descriptor (e.g., "fd:3").
    async fn start_fd_socket(&self) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::fd::{FromRawFd, RawFd};
        use tokio::net::UnixListener;
        use tokio_stream::wrappers::UnixListenerStream;

        let socket_fd: RawFd = self
            .socket_url
            .path()
            .parse()
            .expect("file descriptor in socket URL");

        let listener = unsafe { std::os::unix::net::UnixListener::from_raw_fd(socket_fd) };
        let listener = UnixListener::from_std(listener)?;
        let listener_stream = UnixListenerStream::new(listener);

        let routes = self.routes.clone();

        tonic::transport::Server::builder()
            .add_routes(routes.routes())
            .serve_with_incoming_shutdown(listener_stream, self.stop_listener.clone())
            .await?;

        Ok(())
    }
}
