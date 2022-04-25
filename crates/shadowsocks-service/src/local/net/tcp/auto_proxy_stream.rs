//! A `ProxyStream` that bypasses or proxies data through proxy server automatically

use std::{
    io::{self, IoSlice},
    net::SocketAddr,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::Arc,
    task::{self, Poll},
    thread,
};

use futures::executor::block_on;
use log::debug;
use nix::sys::socket::SockAddr;
use pin_project::pin_project;
use shadowsocks::{
    net::TcpStream,
    relay::{socks5::Address, tcprelay::proxy_stream::ProxyClientStream},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    sync::Mutex,
};

use crate::{
    local::{context::ServiceContext, loadbalancing::ServerIdent},
    net::MonProxyStream,
};

use super::auto_proxy_io::AutoProxyIo;

/// Unified stream for bypassed and proxied connections
#[allow(clippy::large_enum_variant)]
#[pin_project(project = AutoProxyClientStreamProj)]
pub enum AutoProxyClientStream {
    Proxied {
        para: ProxyPara,
        #[pin]
        stream: ProxyClientStream<MonProxyStream<TcpStream>>,
    },
    Bypassed {
        para: ProxyPara,
        #[pin]
        stream: TcpStream,
    },
}
pub struct ProxyPara {
    context: Arc<ServiceContext>,
    is_dns: bool,
    dnsByte: Arc<Mutex<Vec<u8>>>,
}
impl ProxyPara {
    fn default(context: Arc<ServiceContext>, isdns: bool) -> ProxyPara {
        ProxyPara {
            context: context,
            is_dns: isdns,
            dnsByte: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl AutoProxyClientStream {
    /// Connect to target `addr` via shadowsocks' server configured by `svr_cfg`
    pub async fn connect<A>(
        context: Arc<ServiceContext>,
        server: &ServerIdent,
        addr: A,
    ) -> io::Result<AutoProxyClientStream>
    where
        A: Into<Address>,
    {
        let addr = addr.into();
        if context.check_target_bypassed(&addr).await {
            AutoProxyClientStream::connect_bypassed(context, addr).await
        } else {
            AutoProxyClientStream::connect_proxied(context, server, addr).await
        }
    }

    /// Connect directly to target `addr`
    pub async fn connect_bypassed<A>(context: Arc<ServiceContext>, addr: A) -> io::Result<AutoProxyClientStream>
    where
        A: Into<Address>,
    {
        // Connect directly.

        let addr: Address = addr.into();
        let port = addr.port();

        let stream =
            TcpStream::connect_remote_with_opts(context.context_ref(), &addr, context.connect_opts_ref()).await?;

        Ok(AutoProxyClientStream::Bypassed {
            para: ProxyPara::default(context, port == 53),
            stream: stream,
        })
    }

    /// Connect to target `addr` via shadowsocks' server configured by `svr_cfg`
    pub async fn connect_proxied<A>(
        context: Arc<ServiceContext>,
        server: &ServerIdent,
        addr: A,
    ) -> io::Result<AutoProxyClientStream>
    where
        A: Into<Address>,
    {
        let addr: Address = addr.into();
        let port = addr.port();
        let flow_stat = context.flow_stat();
        let stream = match ProxyClientStream::connect_with_opts_map(
            context.context(),
            server.server_config(),
            addr,
            context.connect_opts_ref(),
            |stream| MonProxyStream::from_stream(stream, flow_stat),
        )
        .await
        {
            Ok(s) => s,
            Err(err) => {
                server.tcp_score().report_failure().await;
                return Err(err);
            }
        };

        Ok(AutoProxyClientStream::Proxied {
            para: ProxyPara::default(context, port == 53),
            stream: stream,
        })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        match *self {
            AutoProxyClientStream::Proxied { para: _, stream: ref s } => s.get_ref().get_ref().local_addr(),
            AutoProxyClientStream::Bypassed { para: _, stream: ref s } => s.local_addr(),
        }
    }

    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        match *self {
            AutoProxyClientStream::Proxied { para: _, stream: ref s } => s.get_ref().get_ref().set_nodelay(nodelay),
            AutoProxyClientStream::Bypassed { para: _, stream: ref s } => s.set_nodelay(nodelay),
        }
    }
}

impl AutoProxyIo for AutoProxyClientStream {
    fn is_proxied(&self) -> bool {
        matches!(*self, AutoProxyClientStream::Proxied { para: _, stream: _ })
    }
}

impl AutoProxyClientStream {
    fn getPara(&self) -> &ProxyPara {
        match self {
            AutoProxyClientStream::Proxied { para, stream: _ } => para,
            AutoProxyClientStream::Bypassed { para, stream: _ } => para,
        }
    }

    async fn check_dns_msg(para: &ProxyPara, data: Vec<u8>) {
        let lenbyte = &data[0..2];
        let len: u16 = ((lenbyte[0] as u16) << 2) + (lenbyte[1] as u16);
        let s = len as usize;
        let ss = 2..(s + 2);
        let mut data = &data[ss];
        let byte = &mut *para.dnsByte.lock().await;
        if byte.len() > 0 {
            byte.append(&mut data.to_vec());
            data = byte;
        } else {
            byte.append(&mut data.to_vec());
        }
        let context = para.context.clone();
        let acl = &mut *context.acl.lock().await;
        match acl {
            Some(acl) => {
                if acl.check_dns_msg(data) {
                    byte.clear();
                }
            }
            None => {}
        }
    }
}

impl AsyncRead for AutoProxyClientStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut task::Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let mut isDns = false;
        let mut cpara: Option<&mut ProxyPara> = None;
        let result: Poll<io::Result<()>> = match self.project() {
            AutoProxyClientStreamProj::Proxied { para, stream: s } => {
                let r = s.poll_read(cx, buf);
                cpara = Some(para);
                r
            }
            AutoProxyClientStreamProj::Bypassed { para, stream: s } => {
                let r = s.poll_read(cx, buf);
                cpara = Some(para);
                r
            }
        };
        match result {
            Poll::Ready(Ok(_)) => {
                let para = cpara.unwrap();
                if para.is_dns {
                    let data = buf.filled();
                    let mut nd: Vec<u8> = Vec::new();
                    if data.len() > 2 {
                        for d in data {
                            nd.push(*d);
                        }
                        block_on(AutoProxyClientStream::check_dns_msg(para, nd.clone()));
                    }
                }
            }
            _ => {}
        }

        return result;
    }
}

impl AsyncWrite for AutoProxyClientStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut task::Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        match self.project() {
            AutoProxyClientStreamProj::Proxied { para: _, stream: s } => s.poll_write(cx, buf),
            AutoProxyClientStreamProj::Bypassed { para: _, stream: s } => s.poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<io::Result<()>> {
        match self.project() {
            AutoProxyClientStreamProj::Proxied { para: _, stream: s } => s.poll_flush(cx),
            AutoProxyClientStreamProj::Bypassed { para: _, stream: s } => s.poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<io::Result<()>> {
        match self.project() {
            AutoProxyClientStreamProj::Proxied { para: _, stream: s } => s.poll_shutdown(cx),
            AutoProxyClientStreamProj::Bypassed { para: _, stream: s } => s.poll_shutdown(cx),
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        match self.project() {
            AutoProxyClientStreamProj::Proxied { para: _, stream: s } => s.poll_write_vectored(cx, bufs),
            AutoProxyClientStreamProj::Bypassed { para: _, stream: s } => s.poll_write_vectored(cx, bufs),
        }
    }
}

// impl From<ProxyClientStream<MonProxyStream<TcpStream>>> for AutoProxyClientStream {
//     fn from(s: ProxyClientStream<MonProxyStream<TcpStream>>) -> Self {
//         AutoProxyClientStream::Proxied(s)
//     }
// }
