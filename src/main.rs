use std::io::Read;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::str;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

const MESSAGE_SIZE: usize = 64;

struct ChannelMessage {
    content: String,
}

fn client_handler(mut stream: TcpStream, sender: Sender<String>) -> Result<(), std::io::Error> {
    let mut buffer = [0; MESSAGE_SIZE];

    'handle: loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                println!("{:?} closed connection", stream);
                break 'handle;
            }
            Ok(n) => {
                let msg_str = str::from_utf8(&buffer).expect("Could not parse message as str utf8");

                write!(stream, "{msg_str}")?; // echo
                print!("{:?} <{n}> {msg_str}", stream);

                stream.flush()?;
                buffer.fill(0);
            }
            Err(e) => {
                sender
                    .send(format!("Could not read from {:?}, err: {e}", stream))
                    .unwrap();
            }
        }
    }

    Ok(())
}

fn server_handler(receiver: Receiver<String>) {
    match receiver.recv() {
        Ok(msg) => println!("{msg}"),
        Err(e) => eprintln!("{e}"),
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8123")?;

    let (sender, receiver) = channel::<String>();

    thread::spawn(move || server_handler(receiver));

    for stream in listener.incoming() {
        match stream {
            Ok(st) => {
                let sender = sender.clone();
                thread::spawn(move || client_handler(st, sender));
            }
            Err(e) => {
                eprintln!("Could not connect client to listener 127.0.0.1:8123, err: {e}")
            }
        }
    }
    Ok(())
}
