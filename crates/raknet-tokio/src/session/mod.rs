pub mod command;

use crate::session::command::RakSessionMsg;
use raknet::prelude::{RakSession as RakSessionIntl, *};
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::oneshot;
use tokio::time::{Instant, sleep};
use tracing::debug;

pub struct RakSession {
    pub(crate) msg_tx: UnboundedSender<RakSessionMsg>,
    pub(crate) buf_rx: UnboundedReceiver<Box<[u8]>>,
}

impl RakSession {
    pub fn spawn(session: RakSessionIntl, datagram_tx: UnboundedSender<(Box<[u8]>, SocketAddr)>) -> (Self, UnboundedSender<RakSessionInput>) {
        let (msg_tx, msg_rx) = unbounded_channel();
        let (buf_tx, buf_rx) = unbounded_channel();
        let (tx, rx) = unbounded_channel();

        tokio::spawn(async move {
            let mut msg_rx = msg_rx;
            let mut rx = rx;
            let mut session = session;

            let timeout = sleep(Duration::ZERO);
            tokio::pin!(timeout);

            loop {
                tokio::select! {
                    Some(msg) = msg_rx.recv() => {
                        match msg {
                            RakSessionMsg::Send(buf, reliability, priority) => {
                                let now = SystemTime::now();

                                session.handle(RakSessionInput::Send(buf, reliability, priority, now)).unwrap();
                            }
                            RakSessionMsg::Close(sender) => {
                                sender.send(()).unwrap();
                                return;
                            }
                            _ => {},
                        }
                    }
                    Some(recv) = rx.recv() => {
                        session.handle(recv).unwrap();
                    }
                    _ = &mut timeout => {
                        let now = SystemTime::now();

                        session.handle(RakSessionInput::Timeout(now)).unwrap();
                    }
                }

                while let Some(out) = session.poll() {
                    match out {
                        RakSessionOutput::Timeout(when) => timeout.as_mut().reset(Instant::now() + when),
                        RakSessionOutput::Datagram(buf, addr) => {
                            datagram_tx.send((buf, addr)).unwrap();
                        }
                        RakSessionOutput::Packet(buf) => {
                            let Some(&b) = buf.first() else {
                                continue;
                            };
                            debug!("received packet 0x{:02X} from {}", b, session.addr);

                            buf_tx.send(buf).unwrap();
                        }
                        RakSessionOutput::Disconnected(..) => return,
                    }
                }
            }
        });

        (Self { msg_tx, buf_rx }, tx)
    }

    pub async fn recv<T>(&mut self) -> Option<T>
    where 
        Box<[u8]>: Into<T>
    {
        self.buf_rx.recv().await.map(Into::into)
    }
    
    pub fn try_recv<T>(&mut self) -> Option<T>
    where
        Box<[u8]>: Into<T>
    {
        self.buf_rx.try_recv().ok().map(Into::into)
    }

    pub fn send<T>(&self, buf: T, reliability: RakReliability, priority: RakPriority)
    where 
        T: Into<Box<[u8]>>
    {
        let _ = self.msg_tx.send(RakSessionMsg::Send(buf.into(), reliability, priority));
    }
    
    pub async fn close(&self) {
        let (tx, rx) = oneshot::channel();
        if self.msg_tx.send(RakSessionMsg::Close(tx)).is_err() { 
            return; 
        }
        let _ = rx.await;
    }
    
    pub async fn is_closed(&self) -> bool {
        let (tx, rx) = oneshot::channel();
        if self.msg_tx.send(RakSessionMsg::IsClosed(tx)).is_err() { 
            return true;
        }
        rx.await.unwrap_or_else(|_| true)
    }
}
