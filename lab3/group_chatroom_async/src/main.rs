use std::collections::HashMap;
use std::env;
use std::io;
use std::process;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{self, Receiver, Sender};

#[derive(Debug)]
enum MessageType {
    ChatMessage,
    ClientStart,
    ClientEnd,
}

#[derive(Debug)]
struct Message {
    client_id: usize,
    client: Option<OwnedWriteHalf>,
    message_type: MessageType,
    content: Option<Vec<u8>>,
}

impl Message {
    pub fn new(
        client_id: usize,
        client: Option<OwnedWriteHalf>,
        message_type: MessageType,
        content: Option<Vec<u8>>,
    ) -> Self {
        Message {
            client_id,
            client,
            message_type,
            content,
        }
    }
}

async fn handle_client(mut stream: OwnedReadHalf, id: usize, sender: Sender<Message>) {
    let prompt = "Message: ".as_bytes();
    let mut buffer = Vec::from(prompt);
    let mut data_recv = [0_u8; 1024];

    // read from tcpstream
    while match stream.read(&mut data_recv).await {
        Ok(size) if size > 0 => {
            for x in data_recv[0..size].iter() {
                buffer.push(*x);
                if *x == b'\n' {
                    sender
                        .send(Message::new(
                            id,
                            None,
                            MessageType::ChatMessage,
                            Some(buffer),
                        ))
                        .await
                        .unwrap();
                    buffer = Vec::from(prompt);
                }
            }
            true
        }
        _ => false,
    } {}

    // stream exit normally. notice the receiver
    println!(
        "Terminating connection with {}",
        stream.peer_addr().unwrap()
    );
    sender
        .send(Message::new(id, None, MessageType::ClientEnd, None))
        .await
        .unwrap();
}

async fn handle_send(mut receiver: Receiver<Message>) {
    // HashMap is used to map client_id to specific client
    let mut clients: HashMap<usize, OwnedWriteHalf> = HashMap::new();

    loop {
        let received = receiver.recv().await.unwrap();
        match received.message_type {
            MessageType::ChatMessage => {
                // send to all other clients
                let content = received.content.unwrap_or_default();
                for (id, client) in clients.iter_mut() {
                    if *id != received.client_id {
                        client.write_all(content.as_slice()).await.unwrap();
                    }
                }
            }
            MessageType::ClientStart => {
                // add to hashmap
                clients.insert(received.client_id, received.client.unwrap());
            }
            MessageType::ClientEnd => {
                // remove from hashmap
                clients.remove(&received.client_id);
            }
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // bind to the port and start listening
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let port = &args[1];
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("Server listening on port {}", port);

    // id of clients
    let mut client_cnt = 0;

    // mpsc, consumer(receiver) is to maintain a hashmap of clients and send chat messages
    let (sender, receiver) = mpsc::channel(128);
    tokio::task::spawn(async move { handle_send(receiver).await });

    // wait for clients
    loop {
        let (stream, _) = listener.accept().await?;
        println!("New connection: {}", stream.peer_addr()?);

        // notice the receiver there is a new client
        let (stream, stream_for_write) = stream.into_split();
        sender
            .send(Message::new(
                client_cnt,
                Some(stream_for_write),
                MessageType::ClientStart,
                None,
            ))
            .await
            .unwrap();

        // handle new client
        let sender = sender.clone();
        tokio::task::spawn(async move { handle_client(stream, client_cnt, sender).await });
        client_cnt += 1;
    }
}
