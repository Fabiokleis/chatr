use std::fmt;
use std::io::Read;
use std::io::Write;
use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

const MESSAGE_SIZE: usize = 64;

#[derive(Debug)]
enum ChannelMessageType {
    Connected,
    IncommingMessage,
    ErrorMessage(std::io::Error),
    Disconnected,
}

struct ChannelMessage {
    address: SocketAddr,
    message_type: ChannelMessageType,
    content: Option<[u8; MESSAGE_SIZE]>,
}

impl ChannelMessage {
    fn new(
        address: SocketAddr,
        content: Option<[u8; MESSAGE_SIZE]>,
        message_type: ChannelMessageType,
    ) -> Self {
        Self {
            address,
            message_type,
            content,
        }
    }
}

impl fmt::Display for ChannelMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} {:?} {:?}",
            self.address, self.message_type, self.content
        )
    }
}

fn client_handler(
    stream: Arc<TcpStream>,
    sender: Sender<ChannelMessage>,
) -> Result<(), std::io::Error> {
    let mut buffer = [0; MESSAGE_SIZE];

    let address = stream.peer_addr().unwrap();

    match sender.send(ChannelMessage::new(
        address,
        None,
        ChannelMessageType::Connected,
    )) {
        Ok(_) => {}
        Err(e) => eprintln!("{e}"),
    }

    'handle: loop {
        match stream.as_ref().read(&mut buffer) {
            Ok(0) => {
                match sender.send(ChannelMessage::new(
                    address,
                    None,
                    ChannelMessageType::Disconnected,
                )) {
                    Ok(_) => {}
                    Err(e) => eprintln!("{e}"),
                }
                break 'handle;
            }
            Ok(_) => {
                match sender.send(ChannelMessage::new(
                    address,
                    Some(buffer),
                    ChannelMessageType::IncommingMessage,
                )) {
                    Ok(_) => {}
                    Err(e) => eprintln!("{e}"),
                }

                stream.as_ref().write_all(&buffer)?; // echo
                stream.as_ref().flush()?;
                buffer.fill(0);
            }
            Err(e) => match sender.send(ChannelMessage::new(
                address,
                None,
                ChannelMessageType::ErrorMessage(e),
            )) {
                Ok(_) => {}
                Err(e) => eprintln!("{e}"),
            },
        }
    }

    Ok(())
}

fn server_handler(receiver: Receiver<ChannelMessage>) {
    loop {
        match receiver.recv() {
            Ok(msg) => println!("{msg}"),
            Err(e) => eprintln!("{e}"),
        }
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8123")?;

    let (sender, receiver) = channel::<ChannelMessage>();

    thread::spawn(move || server_handler(receiver));

    for stream in listener.incoming() {
        match stream {
            Ok(st) => {
                let sender = sender.clone();
                thread::spawn(move || client_handler(Arc::new(st), sender));
            }
            Err(e) => {
                eprintln!("Could not connect client to listener 127.0.0.1:8123, err: {e}")
            }
        }
    }
    Ok(())
}
