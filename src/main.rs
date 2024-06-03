use std::net::SocketAddr;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::broadcast,
};

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

    // let tx_clone = tx.clone();
    // tokio::spawn(async move {
    //     let mut rx = tx_clone.subscribe();
    //     loop {
    //         let msg = rx.recv().await;
    //         print!("{}", msg.unwrap().text);
    //     }
    // });

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
        let (socket, addr) = listener.accept().await.unwrap();
        println!("Accepted connection from: {}", addr);

        tokio::spawn(async move {
            let (mut read, mut write) = socket.into_split();
            let mut rx = tx_clone.subscribe();

            write.write_all(b"enter your name: ").await.unwrap();
            let mut name = Vec::new();
            read.read_buf(&mut name).await.unwrap();
            let name = String::from_utf8(name).unwrap().trim_end().to_string();
            println!("{} connected", name);
            loop {
                let mut buff = Vec::new();
                tokio::select! {
                    msg = read.read_buf(&mut buff) => {
                        msg.unwrap();
                        let buff = String::from_utf8(buff.clone()).unwrap();
                        tx_clone.send(Message::new(buff, name.clone(), addr)).unwrap();
                    }

                    msg = rx.recv() => {
                        let msg = msg.unwrap();
                        let Message { text, mut name, sender } = msg;
                        if sender == addr {
                            name = String::from("You");
                        }
                        write.write_all(format!("[{}] {}", name, text).as_bytes()).await.unwrap();
                    }
                }
            }
        });
    }
}
