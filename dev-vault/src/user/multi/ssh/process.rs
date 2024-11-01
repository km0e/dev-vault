use std::{pin::Pin, task::Poll};

use russh::{client::Msg, Channel, ChannelMsg, CryptoVec};
use snafu::{whatever, ResultExt};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_util::sync::ReusableBoxFuture;
use tracing::{debug, trace};

use crate::error;

use super::{BoxedPtyProcess, PtyProcessImpl};

pub struct Process<Tx> {
    channel: Channel<Msg>,
    tx: Tx,
    buffer: CryptoVec,
    idx: usize,
    exit_status: Option<u32>,
}

impl From<Channel<Msg>> for BoxedPtyProcess {
    fn from(value: Channel<Msg>) -> Self {
        let tx = value.make_writer();
        Box::new(Process {
            channel: value,
            tx,
            buffer: CryptoVec::default(),
            idx: 0,
            exit_status: None,
        })
    }
}

impl<Tx: Unpin> AsyncRead for Process<Tx> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if self.exit_status.is_some() {
            return Poll::Ready(Ok(()));
        }
        if self.idx == self.buffer.len() {
            loop {
                let ready = match { ReusableBoxFuture::new(self.channel.wait()) }.poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(ready) => ready,
                };
                match ready {
                    Some(msg) => match msg {
                        ChannelMsg::Data { data } => {
                            let len = data.len();
                            let remaining = buf.remaining();
                            if len > remaining {
                                buf.put_slice(&data[..remaining]);
                                self.idx = remaining;
                            } else {
                                buf.put_slice(&data[..len]);
                            }
                            break;
                        }
                        ChannelMsg::ExitStatus { exit_status } => {
                            self.exit_status = Some(exit_status);
                        }
                        _ => {}
                    },
                    None => {
                        return Poll::Ready(Ok(()));
                    }
                }
            }
        } else {
            let readable = buf.remaining().min(self.buffer.len() - self.idx);
            buf.put_slice(&self.buffer[self.idx..self.idx + readable]);
            self.idx += readable;
        }
        Poll::Ready(Ok(()))
    }
}
impl<Tx: AsyncWrite + Unpin> AsyncWrite for Process<Tx> {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.tx).poll_write(cx, buf)
    }
    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.tx).poll_flush(cx)
    }
    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.tx).poll_shutdown(cx)
    }
    fn is_write_vectored(&self) -> bool {
        self.tx.is_write_vectored()
    }
    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.tx).poll_write_vectored(cx, bufs)
    }
}

#[async_trait::async_trait]
impl<Tx: Unpin + AsyncWrite + Send + Sync> PtyProcessImpl for Process<Tx> {
    async fn sync(
        &mut self,
        mut reader: Box<dyn AsyncRead + Unpin + Send>,
        mut writer: Box<dyn AsyncWrite + Unpin + Send>,
    ) -> crate::Result<i32> {
        trace!("start sync local and remote");
        let mut reader_closed = false;
        let mut buf = vec![0; 1024];
        loop {
            tokio::select! {
                r = reader.read(&mut buf), if !reader_closed => {
                    match r {
                        Ok(0) => {
                            reader_closed = true;
                            self.channel.eof().await?;
                        },
                        Ok(n) => {
                            debug!("sync {} byte to remote",n);
                            self.channel.data(&buf[..n]).await?;
                        }
                        Err(e) => break Err(e).context(error::IoSnafu{about:"stdin read"}),
                    };
                },
                Some(msg) = self.channel.wait() => {
                    match msg {
                        ChannelMsg::Data { ref data } => {
                            debug!("sync {} byte to local",data.len());
                            writer.write_all(data).await.context(error::IoSnafu{about:"writer write"})?;
                            writer.flush().await.context(error::IoSnafu{about:"writer flush"})?
                        }
                        ChannelMsg::ExitStatus { exit_status } => {
                            if !reader_closed {
                                self.channel.eof().await?;
                            }
                            break Ok(exit_status as i32);
                        }
                        _ => {}
                    }
                },
            }
        }
    }
    async fn wait(&mut self) -> crate::Result<i32> {
        self.channel.eof().await?;
        loop {
            match self.channel.wait().await {
                Some(msg) => {
                    if let ChannelMsg::ExitStatus { exit_status } = msg {
                        self.exit_status = Some(exit_status);
                        break Ok(exit_status as i32);
                    }
                }
                None => {
                    whatever!("unexpected exit")
                }
            }
        }
    }
    async fn window_change(
        &self,
        width: u32,
        height: u32,
        pix_width: u32,
        pix_height: u32,
    ) -> crate::Result<()> {
        Ok(self
            .channel
            .window_change(width, height, pix_width, pix_height)
            .await?)
    }
}
