use std::env;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::process;
use std::thread;

fn handle_client(mut stream: TcpStream, stream_write: &mut TcpStream) {
    let prompt = "Message: ".as_bytes();
    let mut buffer = Vec::from(prompt);
    let mut data_recv = [0_u8; 1024];
    while match stream.read(&mut data_recv) {
        Ok(size) => {
            for x in data_recv[0..size].iter() {
                buffer.push(*x);
                if *x == b'\n' {
                    stream_write.write_all(buffer.as_slice()).unwrap();
                    buffer = Vec::from(prompt);
                }
            }
            true
        }
        _ => {
            println!(
                "An error occurred, terminating connection with {}",
                stream.peer_addr().unwrap()
            );
            stream.shutdown(Shutdown::Both).unwrap();
            false
        }
    } {}
}

fn main() {
    // get port
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let port = &args[1];

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
    println!("Server listening on port {}", port);

    let mut tcp1 = listener.accept().expect("Error handling first client!").0;
    let mut tcp2 = listener.accept().expect("Error handling second client!").0;

    let tcp1_read = tcp1.try_clone().expect("clone failed!");
    let tcp2_read = tcp2.try_clone().expect("clone failed!");

    let a = thread::spawn(move || handle_client(tcp1_read, &mut tcp2));
    let b = thread::spawn(move || handle_client(tcp2_read, &mut tcp1));

    a.join().expect("thread panics!");
    b.join().expect("thread panics!");

    drop(listener);
}
