//! Serve the embedded web app locally: `cargo run -p bagz-web --bin serve` -> http://127.0.0.1:43123

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

fn main() -> std::io::Result<()> {
    let addr = std::env::var("BAGZ_ADDR").unwrap_or_else(|_| "127.0.0.1:43123".into());
    let listener = TcpListener::bind(&addr)?;
    println!("BAGZ demo on http://{addr}");
    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut line = String::new();
        BufReader::new(&stream).read_line(&mut line)?;
        let path = line.split_whitespace().nth(1).unwrap_or("/");
        let a = bagz_web::resolve(path);
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            a.content_type,
            a.body.len()
        );
        stream.write_all(head.as_bytes())?;
        stream.write_all(a.body.as_bytes())?;
    }
    Ok(())
}
