use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::process;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

enum MessageType {
    ChatMessage,
    ClientStart,
    ClientEnd,
}

struct Message {
    client_id: usize,
    client: Option<TcpStream>,
    message_type: MessageType,
    content: Option<Vec<u8>>,
}

impl Message {
    pub fn new(
        client_id: usize,
        client: Option<TcpStream>,
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

fn handle_client(mut stream: TcpStream, id: usize, sender: Sender<Message>) {
    let prompt = "Message: ".as_bytes();
    let mut buffer = Vec::from(prompt);
    let mut data_recv = [0_u8; 1024];

    // read from tcpstream
    while match stream.read(&mut data_recv) {
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
        .unwrap();
    stream.shutdown(Shutdown::Both).unwrap();
}

fn handle_send(receiver: Receiver<Message>) {
    // HashMap is used to map client_id to specific client
    let mut clients: HashMap<usize, TcpStream> = HashMap::new();

    for received in &receiver {
        match received.message_type {
            MessageType::ChatMessage => {
                // send to all other clients
                let content = received.content.unwrap_or_default();
                for mut client in &clients {
                    if *client.0 != received.client_id {
                        client.1.write_all(content.as_slice()).unwrap();
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

fn main() {
    // bind to the port and start listening
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let port = &args[1];
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
    println!("Server listening on port {}", port);

    // id of clients
    let mut client_cnt = 0;

    // mpsc, consumer(receiver) is to maintain a hashmap of clients and send chat messages
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        handle_send(receiver);
    });

    // wait for clients
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());

                // notice the receiver there is a new client
                let stream_for_write = stream.try_clone().unwrap();
                sender
                    .send(Message::new(
                        client_cnt,
                        Some(stream_for_write),
                        MessageType::ClientStart,
                        None,
                    ))
                    .unwrap();

                // handle new client
                let sender = sender.clone();
                thread::spawn(move || {
                    handle_client(stream, client_cnt, sender);
                });
                client_cnt += 1;
            }
            Err(e) => {
                // connection failed
                println!("Error: {}", e);
            }
        }
    }
}
