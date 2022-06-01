use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::process;
use std::thread;
use std::{env, io};

fn handle_client(mut stream: TcpStream, stream_write: &mut TcpStream) -> io::Result<()> {
    let prompt = "Message: ".as_bytes();
    let mut buffer = Vec::from(prompt);
    let mut data_recv = [0_u8; 1024];
    while match stream.read(&mut data_recv) {
        Ok(size) if size > 0 => {
            for x in data_recv[0..size].iter() {
                buffer.push(*x);
                if *x == b'\n' {
                    stream_write.write_all(buffer.as_slice())?;
                    buffer = Vec::from(prompt);
                }
            }
            true
        }
        _ => false,
    } {}
    println!("Terminating connection with {}", stream.peer_addr()?);
    stream.shutdown(Shutdown::Both)?;
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let port = &args[1];

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
    println!("Server listening on port {}", port);

    let mut tcp1 = listener.accept().expect("Error handling first client!").0;
    let mut tcp2 = listener.accept().expect("Error handling second client!").0;

    let tcp1_read = tcp1.try_clone().expect("Clone failed!");
    let tcp2_read = tcp2.try_clone().expect("Clone failed!");

    let tcp1_handler = thread::spawn(move || handle_client(tcp1_read, &mut tcp2));
    let tcp2_handler = thread::spawn(move || handle_client(tcp2_read, &mut tcp1));

    tcp1_handler.join().expect("Thread panics!")?;
    tcp2_handler.join().expect("Thread panics!")?;
    Ok(())
}
