use std::io::Write;
use std::net::{TcpListener, TcpStream};

fn handle_connection(mut stream: TcpStream) -> Result<(), std::io::Error> {
    println!("{:?}", stream);

    let message = String::from("Hello, World\n");

    write!(stream, "{message}")
}

fn main() -> std::io::Result<()> {
    let address = "127.0.0.1:8124";
    let listener = TcpListener::bind(address)?;

    for stream in listener.incoming() {
        match stream {
            Ok(st) => handle_connection(st)?,
            Err(e) => eprintln!("Could not connect client to listener {address}, err: {e}"),
        }
    }
    Ok(())
}
