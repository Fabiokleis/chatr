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
    Connected(Arc<TcpStream>),
    IncomingMessage,
    Disconnected,
    ErrorMessage,
}

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

    fn shell_format(&self, lf: bool) {
        let shell_format = if lf {
            format!("{}@{}$ \n", self.name, self.address)
        } else {
            format!("{}@{}$ ", self.name, self.address)
        };
        match self.stream.as_ref().write(shell_format.as_bytes()) {
            Ok(_) => {}
            Err(e) => eprintln!("ERROR: {e}"),
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
                    if let Some(user) = clients.get_mut(&msg.address.to_string()) {
                        user.messages.push(msg.content);
                        user.shell_format(false);
                        println!("INFO: {user}");
                    }
                    let user = clients.get(&msg.address.to_string()).unwrap();
                    broadcast_message(user, &clients)
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

fn broadcast_message(broadcaster: &Client, clients: &HashMap<String, Client>) {
    for (_, c) in clients.iter() {
        if c.address != broadcaster.address {
            if let Some(message) = broadcaster.messages.last() {
                c.shell_format(true);
                match c.stream.as_ref().write_all(message) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("{e}")
                    }
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
