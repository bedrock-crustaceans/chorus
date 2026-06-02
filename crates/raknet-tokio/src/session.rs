use raknet::prelude::{RakSession as RakSessionIntl, *};
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tokio::time::{Instant, sleep};
use tracing::debug;

pub struct RakSession {}

pub fn spawn_session(datagram_tx: UnboundedSender<(Vec<u8>, SocketAddr)>, session: RakSessionIntl) -> UnboundedSender<RakSessionInput> {
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
                        debug!("received packet 0x{:02X} from {}", b, session.addr)
                    }
                    RakSessionOutput::Disconnected(..) => return,
                }
            }
        }
    });

    tx
}
