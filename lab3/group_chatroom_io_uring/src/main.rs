use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::os::unix::io::AsRawFd;
use std::os::unix::prelude::FromRawFd;
use std::{env, io, process, ptr};

use io_uring::{opcode, types, IoUring};

static PROMPT: &[u8] = "Message: ".as_bytes();

fn main() -> io::Result<()> {
    // bind to the port and start listening
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let port = &args[1];
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
    println!("Server listening on port {}", port);

    let mut client_id = 1;
    let mut tcp_streams = HashMap::new();
    let mut stream_buffers = HashMap::new();

    let mut ring = IoUring::new(256)?;
    let (submitter, mut sq, mut cq) = ring.split();

    // tcp listener accept event
    let accept_e = opcode::Accept::new(
        types::Fd(listener.as_raw_fd()),
        ptr::null_mut(),
        ptr::null_mut(),
    )
    .build()
    .user_data(0);
    unsafe {
        sq.push(&accept_e).unwrap();
    }
    sq.sync();

    // global message read buffer
    let mut data_recv = [0_u8; 1024];

    loop {
        submitter.submit_and_wait(1)?;
        cq.sync();

        for cqe in &mut cq {
            match cqe.user_data() {
                // new client connection
                0 => {
                    let fd = cqe.result();
                    let poll_token = client_id;
                    client_id += 1;
                    let stream = unsafe { TcpStream::from_raw_fd(fd) };
                    stream.set_nonblocking(true)?;
                    println!("New connection: {}", stream.peer_addr()?);
                    tcp_streams.insert(poll_token, stream);
                    stream_buffers.insert(poll_token, Vec::from(PROMPT));

                    let poll_e = opcode::PollAdd::new(types::Fd(fd), libc::POLLIN as _)
                        .build()
                        .user_data(poll_token as _);
                    let accept_e = opcode::Accept::new(
                        types::Fd(listener.as_raw_fd()),
                        ptr::null_mut(),
                        ptr::null_mut(),
                    )
                    .build()
                    .user_data(0);
                    unsafe {
                        sq.push(&poll_e).unwrap();
                        sq.push(&accept_e).unwrap();
                    }
                }
                // ready to receive message from a client
                id => {
                    let stream = tcp_streams.get(&id).unwrap();
                    let stream_fd = stream.as_raw_fd();
                    let mut stream = stream.try_clone()?;

                    // receive while not empty
                    let mut exit_flag = false;
                    while match stream.read(&mut data_recv) {
                        Ok(size) if size > 0 => {
                            let buffer = stream_buffers.get_mut(&id).unwrap();
                            for x in data_recv[0..size].iter() {
                                buffer.push(*x);
                                if *x == b'\n' {
                                    for (stream_id, mut stream) in &tcp_streams {
                                        if *stream_id != id {
                                            stream.write_all(buffer.as_slice())?;
                                        }
                                    }
                                    *buffer = Vec::from(PROMPT);
                                }
                            }
                            true
                        }
                        Ok(0) => {
                            println!("Terminating connection with {}", stream.peer_addr()?);
                            tcp_streams.remove(&id);
                            stream_buffers.remove(&id);
                            stream.shutdown(Shutdown::Both)?;
                            exit_flag = true;
                            false
                        }
                        _ => false,
                    } {}
                    if !exit_flag {
                        let poll_e = opcode::PollAdd::new(types::Fd(stream_fd), libc::POLLIN as _)
                            .build()
                            .user_data(id);
                        unsafe {
                            sq.push(&poll_e).unwrap();
                        }
                    }
                }
            }
            sq.sync();
        }
    }
}
