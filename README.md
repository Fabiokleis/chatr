# chatr

Exploring Rust std::net crate https://doc.rust-lang.org/std/net/index.html

The project starts a tcp server listening to port 6969 and handle multiple client connections.

### Run
To start server
```
cargo run --bin server
```

To start client
```
cargo run --bin client -- 127.0.0.1 6969
```
