use chatr::Message;
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

#[derive(Debug, Clone)]
enum ChannelMessageType {
    Connected(Arc<TcpStream>),
    IncomingMessage,
    Disconnected,
    ErrorMessage,
}

#[derive(Clone)]
struct ChannelMessage {
    address: SocketAddr,
    message_type: ChannelMessageType,
    content: Vec<u8>,
}

impl ChannelMessage {
    fn new(address: SocketAddr, content: Vec<u8>, message_type: ChannelMessageType) -> Self {
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
    stream: Arc<TcpStream>,
    messages: Vec<Vec<u8>>,
}

impl Client {
    fn new(name: String, address: SocketAddr, stream: Arc<TcpStream>) -> Self {
        Client {
            name,
            address,
            messages: Vec::new(),
            stream,
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
            self.messages.last().unwrap()
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
) -> Result<(), std::io::Error> {
    let mut buffer = [0; MESSAGE_SIZE];

    let address = stream.peer_addr().unwrap();

    match sender.send(ChannelMessage::new(
        address,
        vec![],
        ChannelMessageType::Connected(Arc::clone(&stream)),
    )) {
        Ok(_) => {}
        Err(e) => eprintln!("{e}"),
    }

    'handle: loop {
        match stream.as_ref().read(&mut buffer) {
            Ok(0) => {
                match sender.send(ChannelMessage::new(
                    address,
                    vec![],
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
                    Vec::from(buffer),
                    ChannelMessageType::IncomingMessage,
                )) {
                    Ok(_) => {}
                    Err(e) => eprintln!("{e}"),
                }

                //stream.as_ref().write_all(&buffer)?; // echo
                stream.as_ref().flush()?;
                buffer.fill(0);
            }
            Err(err) => match sender.send(ChannelMessage::new(
                address,
                err.to_string().into_bytes(),
                ChannelMessageType::ErrorMessage,
            )) {
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
                ChannelMessageType::Connected(socket) => {
                    clients.insert(
                        msg.address.to_string(),
                        Client::new(generate_client_id(RANDOM_ID_LENGTH), msg.address, socket),
                    );
                }
                ChannelMessageType::IncomingMessage => {
                    broadcast_message(msg.clone(), &clients);
                    if let Some(user) = clients.get_mut(&msg.address.to_string()) {
                        user.messages.push(msg.content);
                        println!("INFO: {}", user);
                    };
                }
                ChannelMessageType::Disconnected => {
                    if let Some(user) = clients.remove(&msg.address.to_string()) {
                        println!("INFO: {msg} {}", user.name);
                    }
                }
                ChannelMessageType::ErrorMessage => {
                    eprintln!("ERROR: {msg}")
                }
            },
            Err(e) => eprintln!("{e}"),
        }
    }
}

fn broadcast_message(msg: ChannelMessage, clients: &HashMap<String, Client>) {
    let mut name: Option<String> = None;
    if let Some(client) = clients.get(&msg.address.to_string()) {
        name = Some(client.name.clone());
    }
    for (_, c) in clients.iter() {
        if c.address != msg.address {
            let message = Message::new(
                name.clone().unwrap_or("UNKNOWN".to_string()),
                msg.address.to_string(),
                String::from_utf8(msg.content.clone()).unwrap(),
            );

            let serialized = serde_json::to_vec(&message).unwrap();
            match c.stream.as_ref().write_all(&serialized[..]) {
                Ok(_) => {
                    let _ = c.stream.as_ref().write(&[b'\n']);
                }
                Err(e) => {
                    eprintln!("{e}")
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6969")?;

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
