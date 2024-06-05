use std::{io, net::SocketAddr};

use tokio::{
    fs::OpenOptions, io::{AsyncReadExt, AsyncWriteExt}, sync::broadcast
};

#[derive(Debug)]
struct Logger {
    file: tokio::fs::File,
}

impl Logger {
    pub fn new(file: tokio::fs::File) -> Self {
        Self { file }
    }

    pub async fn log(&mut self, content: &str) {
        let log = format!("[{}] {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), content);
        self.file.write_all(log.as_bytes()).await.unwrap();
        print!("{}", log);
    }
}

#[derive(Debug, Clone)]
struct Message {
    text: String,
    name: String,
    sender: SocketAddr,
}

impl Message {
    fn new(text: String, name: String, sender: SocketAddr) -> Self {
        Self { text, name, sender }
    }
}

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    let (tx, _rx) = broadcast::channel::<Message>(100);

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            let mut buff = Vec::new();
            tokio::io::stdin().read_buf(&mut buff).await.unwrap();
            println!("Server sent: {}", String::from_utf8(buff.clone()).unwrap());
            tx_clone
                .send(Message::new(
                    String::from_utf8(buff).unwrap(),
                    String::from("Server"),
                    "0.0.0.0:0".parse().unwrap(),
                ))
                .unwrap();
        }
    });

    loop {
        let tx_clone = tx.clone();
        let log_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open("log.txt")
            .await
            .unwrap();
        let mut logger = Logger::new(log_file);
        let (socket, addr) = listener.accept().await.unwrap();
        println!("Accepted connection from: {}", addr);

        tokio::spawn(async move {
            let (mut read, mut write) = socket.into_split();
            let mut rx = tx_clone.subscribe();

            write.write_all(b"enter your name: ").await.unwrap();
            let mut name = Vec::new();
            read.read_buf(&mut name).await.unwrap();
            let name = String::from_utf8(name).unwrap().trim_end().to_string();
            // println!("{} connected", name);
            logger.log(&format!("{} connected", name)).await;

            loop {
                let mut buff = Vec::new();
                tokio::select! {
                    msg = read.read_buf(&mut buff) => {
                        if let Err(err) = msg {
                            match err.kind() {
                                io::ErrorKind::ConnectionReset | io::ErrorKind::BrokenPipe => {
                                    logger.log(&format!("{} disconnected", name)).await;
                                    return;
                                }
                                _ => panic!("failed to read from socket; err = {:?}", err),
                            }
                        }
                        let buff = String::from_utf8(buff.clone()).unwrap();
                        let buff = buff.trim_end().to_string();
                        logger.log(&format!("{}: {}", name, buff)).await;
                        tx_clone.send(Message::new(buff.clone(), name.clone(), addr)).unwrap();
                    }

                    msg = rx.recv() => {
                        let msg = msg.unwrap();
                        let Message { text, name: client_name, sender } = msg;
                        let name =  if sender == addr {
                            String::from("You")
                        } else {
                            name.clone()
                        };

                        if let Err(err) = write.write_all(format!("[{}] {}\n", name, text).as_bytes()).await {
                            match err.kind() {
                                io::ErrorKind::ConnectionReset | io::ErrorKind::BrokenPipe => {
                                    logger.log(&format!("{} disconnected", client_name)).await;
                                    return;
                                }
                                _ => panic!("failed to read from socket; err = {:?}", err),
                            }
                        }
                    }
                }
            }
        });
    }
}
