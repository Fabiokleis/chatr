use chatr::Message;
use std::io::{BufRead, BufReader};
use std::io::{Read, Write};
use std::net::{IpAddr, TcpStream};
use std::process::exit;
use std::string::ToString;
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

fn server_message_handler(conn: Arc<TcpStream>) {
    let mut buffer = String::new();
    let mut reader = BufReader::new(conn.as_ref());
    loop {
        match reader.read_line(&mut buffer) {
            Ok(0) => {
                println!("server down");
                break;
            }
            Ok(_) => {
                let bytes = buffer.as_bytes();
                match serde_json::from_slice::<Message>(bytes) {
                    Ok(message) => {
                        print!(
                            "{}@{}# {}",
                            message.author.name, message.author.address, message.content
                        );
                    }
                    Err(e) => {
                        eprintln!("{e}")
                    }
                }
                reader.consume(buffer.len());
                buffer.clear();
            }
            Err(e) => {
                eprintln!("{e}");
                exit(1);
            }
        }
    }
}

fn handle_connection(conn: TcpStream) {
    let mut buff = String::new();

    let stream = Arc::new(conn);
    let stream1 = Arc::clone(&stream);

    std::thread::spawn(move || server_message_handler(stream));

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
