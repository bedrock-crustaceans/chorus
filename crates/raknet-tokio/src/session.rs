use raknet::prelude::{RakSession as RakSessionIntl, *};
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::time::{Instant, sleep};
use tracing::debug;

pub struct RakSession {
    pub(crate) packet_rx: UnboundedReceiver<Box<[u8]>>,
}

impl RakSession {
    pub fn spawn(session: RakSessionIntl, datagram_tx: UnboundedSender<(Vec<u8>, SocketAddr)>) -> (Self, UnboundedSender<RakSessionInput>) {
        let (packet_tx, packet_rx) = unbounded_channel();
        let (tx, rx) = unbounded_channel();

        tokio::spawn(async move {
            let mut rx = rx;
            let mut session = session;

            let timeout = sleep(Duration::ZERO);
            tokio::pin!(timeout);

            loop {
                tokio::select! {
                    Some(recv) = rx.recv() => {
                        session.handle(recv).unwrap();
                    }
                    _ = &mut timeout => {
                        let now = SystemTime::now();

                        session.handle(RakSessionInput::Timeout(now)).unwrap();
                    }
                }

                while let Some(msg) = session.poll() {
                    match msg {
                        RakSessionOutput::Timeout(when) => timeout.as_mut().reset(Instant::now() + when),
                        RakSessionOutput::Datagram(buf, addr) => {
                            datagram_tx.send((buf, addr)).unwrap();
                        }
                        RakSessionOutput::Packet(buf) => {
                            let Some(&b) = buf.first() else {
                                continue;
                            };
                            debug!("received packet 0x{:02X} from {}", b, session.addr);

                            packet_tx.send(buf.into_boxed_slice()).unwrap();
                        }
                        RakSessionOutput::Disconnected(..) => return,
                    }
                }
            }
        });

        (Self { packet_rx }, tx)
    }
}
