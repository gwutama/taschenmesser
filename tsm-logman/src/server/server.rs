use std::net::SocketAddr;
use std::io;
use tokio::net::UdpSocket;
use crate::server::syslog;


pub struct UdpServer {
    socket: UdpSocket,
    buf: Vec<u8>,
    to_send: Option<(usize, SocketAddr)>,
}


impl UdpServer {
    pub fn new(socket: UdpSocket) -> UdpServer {
        UdpServer {
            socket,
            buf: vec![0; 4096],
            to_send: None,
        }
    }

    pub async fn run(self) -> Result<(), io::Error> {
        let UdpServer {
            socket,
            mut buf,
            mut to_send,
        } = self;

        loop {
            // First we check to see if there's a message we need to process.
            if let Some((size, peer)) = to_send {
                let buffer = &mut buf[..size];

                // Parse syslog message
                match syslog::parse(peer, size, buffer) {
                    Some(msg) => println!("{:?}", msg),
                    None => {
                        match std::str::from_utf8(buffer) {
                            Ok(s) => eprintln!("error parsing: {}", s),
                            Err(e) => eprintln!("received message not parseable and not UTF-8: {}", e),
                        }
                    }
                }
            }

            // If we're here then `to_send` is `None`, so we take a look for the next message to process.
            to_send = Some(socket.recv_from(&mut buf).await?);
        }
    }
}