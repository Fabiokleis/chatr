use std::io::Read;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::str;
use std::sync::mpsc::{channel, Sender};
use std::thread;

fn handle_connection(mut stream: TcpStream, sender: Sender<String>) -> Result<(), std::io::Error> {
    println!("{:?}", stream);

    let mut buffer: [u8; 1024] = [0; 1024];

    'handle: loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                println!("{:?} closed connection", stream);
                break 'handle;
            }
            Ok(n) => {
                let msg_str = str::from_utf8(&buffer).unwrap();

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

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8123")?;

    let (sender, receiver) = channel::<String>();

    for stream in listener.incoming() {
        match stream {
            Ok(st) => {
                let sender = sender.clone();
                thread::spawn(move || handle_connection(st, sender));
            }
            Err(e) => {
                eprintln!("Could not connect client to listener 127.0.0.1:8123, err: {e}")
            }
        }

        match receiver.recv() {
            Ok(msg) => println!("{msg}"),
            Err(e) => eprintln!("{e}"),
        }
    }
    Ok(())
}
