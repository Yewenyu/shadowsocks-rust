//! Shadowsocks Local Tunnel Server

use std::{io, sync::Arc, time::Duration};

use futures::{future, FutureExt};
use shadowsocks::{config::Mode, relay::socks5::Address, ServerAddr};

use crate::local::{context::ServiceContext, loadbalancing::PingBalancer};

use super::{tcprelay::TunnelTcpServer, udprelay::TunnelUdpServer};

pub struct TunnelBuilder {
    context: Arc<ServiceContext>,
    forward_addr: Address,
    mode: Mode,
    udp_expiry_duration: Option<Duration>,
    udp_capacity: Option<usize>,
    client_addr: ServerAddr,
    udp_addr: Option<ServerAddr>,
    balancer: PingBalancer,
}

impl TunnelBuilder {
    /// Create a new Tunnel server forwarding to `forward_addr`
    pub fn new(forward_addr: Address, client_addr: ServerAddr, balancer: PingBalancer) -> TunnelBuilder {
        let context = ServiceContext::new();
        TunnelBuilder::with_context(Arc::new(context), forward_addr, client_addr, balancer)
    }

    /// Create a new Tunnel server with context
    pub fn with_context(
        context: Arc<ServiceContext>,
        forward_addr: Address,
        client_addr: ServerAddr,
        balancer: PingBalancer,
    ) -> TunnelBuilder {
        TunnelBuilder {
            context,
            forward_addr,
            mode: Mode::TcpOnly,
            udp_expiry_duration: None,
            udp_capacity: None,
            client_addr,
            udp_addr: None,
            balancer,
        }
    }

    /// Set UDP association's expiry duration
    pub fn set_udp_expiry_duration(&mut self, d: Duration) {
        self.udp_expiry_duration = Some(d);
    }

    /// Set total UDP association to be kept simultaneously in server
    pub fn set_udp_capacity(&mut self, c: usize) {
        self.udp_capacity = Some(c);
    }

    /// Set server mode
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// Set UDP bind address
    pub fn set_udp_bind_addr(&mut self, addr: ServerAddr) {
        self.udp_addr = Some(addr);
    }

    pub async fn build(self) -> io::Result<Tunnel> {
        let mut tcp_server = None;
        if self.mode.enable_tcp() {
            let server = TunnelTcpServer::new(
                self.context.clone(),
                &self.client_addr,
                self.balancer.clone(),
                self.forward_addr.clone(),
            )
            .await?;
            tcp_server = Some(server);
        }

        let mut udp_server = None;
        if self.mode.enable_udp() {
            let udp_addr = self.udp_addr.as_ref().unwrap_or(&self.client_addr);

            let server = TunnelUdpServer::new(
                self.context.clone(),
                udp_addr,
                self.udp_expiry_duration,
                self.udp_capacity,
                self.balancer,
                self.forward_addr,
            )
            .await?;
            udp_server = Some(server);
        }

        Ok(Tunnel { tcp_server, udp_server })
    }
}

/// Tunnel Server
pub struct Tunnel {
    tcp_server: Option<TunnelTcpServer>,
    udp_server: Option<TunnelUdpServer>,
}

impl Tunnel {
    /// TCP server instance
    pub fn tcp_server(&self) -> Option<&TunnelTcpServer> {
        self.tcp_server.as_ref()
    }

    /// UDP server instance
    pub fn udp_server(&self) -> Option<&TunnelUdpServer> {
        self.udp_server.as_ref()
    }

    /// Start serving
    pub async fn run(self) -> io::Result<()> {
        let mut vfut = Vec::new();

        if let Some(tcp_server) = self.tcp_server {
            vfut.push(tcp_server.run().boxed());
        }

        if let Some(udp_server) = self.udp_server {
            vfut.push(udp_server.run().boxed());
        }

        let (res, ..) = future::select_all(vfut).await;
        res
    }
}
