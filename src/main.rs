use rand::distributions::Alphanumeric;
use rand::Rng;
use std::collections::HashMap;
use std::fmt;
use std::io::Read;
use std::io::Write;
use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

const MESSAGE_SIZE: usize = 32;
const RANDOM_ID_LENGTH: usize = 8;

#[derive(Debug)]
enum ChannelMessageType {
    Connected,
    IncommingMessage,
    Disconnected,
}

enum ChannelMessageError<T> {
    ErrorMessage(T),
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
            "[{}] {:?} {:?}",
            self.address, self.message_type, self.content
        )
    }
}

struct Client {
    name: String,
    address: SocketAddr,
    messages: Vec<[u8; MESSAGE_SIZE]>,
}

impl Client {
    fn new(name: String, address: SocketAddr) -> Self {
        Client {
            name,
            address,
            messages: Vec::new(),
        }
    }
}

impl fmt::Display for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} sent {:?}",
            self.address,
            self.name,
            self.messages.last()
        )
    }
}

fn generate_client_id(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

fn client_handler(
    stream: Arc<TcpStream>,
    sender: Sender<ChannelMessage>,
    error_sender: Sender<ChannelMessageError<std::io::Error>>,
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
            Err(e) => match error_sender.send(ChannelMessageError::ErrorMessage(e)) {
                Ok(_) => {}
                Err(e) => eprintln!("{e}"),
            },
        }
    }

    Ok(())
}

fn server_handler(receiver: Receiver<ChannelMessage>) {
    let mut clients = HashMap::<String, Client>::new();
    loop {
        match receiver.recv() {
            Ok(msg) => match msg.message_type {
                ChannelMessageType::Connected => {
                    clients.insert(
                        msg.address.to_string(),
                        Client::new(generate_client_id(RANDOM_ID_LENGTH), msg.address),
                    );
                }
                ChannelMessageType::IncommingMessage => {
                    if let Some(user) = clients.get_mut(&msg.address.to_string()) {
                        user.messages.push(msg.content.unwrap());
                        println!("INFO: {user}");
                    }
                }
                ChannelMessageType::Disconnected => {
                    if let Some(user) = clients.remove(&msg.address.to_string()) {
                        println!("INFO: {msg} {}", user.name);
                    }
                }
            },
            Err(e) => eprintln!("{e}"),
        }
    }
}

fn server_error_handler<T: fmt::Display>(receiver: Receiver<ChannelMessageError<T>>) {
    loop {
        match receiver.recv() {
            Ok(ChannelMessageError::ErrorMessage(err)) => eprintln!("{err}"),
            Err(e) => eprintln!("{e}"),
        }
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6969")?;

    let (sender, receiver) = channel::<ChannelMessage>();
    let (sender_error, receiver_error) = channel::<ChannelMessageError<std::io::Error>>();

    thread::spawn(move || server_handler(receiver));
    thread::spawn(move || server_error_handler(receiver_error));

    for stream in listener.incoming() {
        match stream {
            Ok(st) => {
                let sender = sender.clone();
                let sender_error = sender_error.clone();
                thread::spawn(move || client_handler(Arc::new(st), sender, sender_error));
            }
            Err(e) => {
                eprintln!("Could not connect client to listener 127.0.0.1:8123, err: {e}")
            }
        }
    }
    Ok(())
}
