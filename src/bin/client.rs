use std::io::{Read, Write};
use std::net::{IpAddr, TcpStream};
use std::process::exit;
use std::string::ToString;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

fn usage() {
    eprintln!("Usage: client [host] [port]");
    exit(1);
}

#[derive(Debug, Default)]
struct Server {
    host: String,
    port: u16,
}

impl ToString for Server {
    fn to_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

fn stdout_receiver(rec: Receiver<String>, mut stdout: std::io::Stdout) {
    loop {
        match rec.recv() {
            Ok(msg) => {
                if msg == "closed" {
                    break;
                }
                let _ = stdout.write_all(msg.as_bytes());
            }
            Err(e) => {
                eprintln!("{e}");
                exit(1);
            }
        }
    }
}

fn server_message_handler(conn: Arc<TcpStream>, sender: Sender<String>) {
    let mut bytes = [0; 32];
    loop {
        match conn.as_ref().read(&mut bytes) {
            Ok(0) => {
                let _ = sender.send("closed".to_string());
                break;
            }
            Ok(_) => {
                if let Ok(buff) = String::from_utf8(bytes.to_vec()) {
                    let _ = sender.send(buff);
                }
                bytes.fill(0);
            }
            Err(e) => {
                eprintln!("{e}");
                exit(1);
            }
        }
    }
}

fn handle_connection(conn: TcpStream) {
    let stdout = std::io::stdout();
    let mut buff = String::new();

    let stream = Arc::new(conn);
    let stream1 = Arc::clone(&stream);
    let (sender, receiver) = channel::<String>();

    std::thread::spawn(move || stdout_receiver(receiver, stdout));
    std::thread::spawn(move || server_message_handler(stream, sender));

    loop {
        match std::io::stdin().read_line(&mut buff) {
            Ok(_) => match buff.as_str().trim() {
                "close" => {
                    break;
                }
                _ => match stream1.as_ref().write_all(buff.as_bytes()) {
                    Ok(_) => {
                        buff.clear();
                    }
                    Err(e) => {
                        println!("failed to send message: {e}");
                        buff.clear();
                    }
                },
            },
            Err(e) => {
                eprintln!("{e}");
                exit(1);
            }
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    if args.len() != 2 {
        usage();
    }

    let mut server = Server::default();

    match args.next().unwrap().parse::<IpAddr>() {
        Ok(ip) => {
            server.host = ip.to_string();
        }
        Err(e) => {
            eprintln!("{e}");
            exit(1);
        }
    }

    match args.next().unwrap().parse::<u16>() {
        Ok(port) => {
            server.port = port;
        }
        Err(e) => {
            eprintln!("{e}");
            exit(1);
        }
    }
    println!("{server:?}");

    match std::net::TcpStream::connect(server.to_string()) {
        Ok(stream) => handle_connection(stream),
        Err(e) => {
            eprintln!("{e}");
            exit(1);
        }
    }
}
