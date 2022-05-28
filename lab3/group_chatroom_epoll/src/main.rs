use libc::epoll_event;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::os::unix::prelude::AsRawFd;
use std::process;
use std::{env, io};

// see: https://github.com/tokio-rs/mio/
macro_rules! syscall {
    ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
        let res = unsafe { libc::$fn($($arg, )*) };
        if res == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }
    }};
}

static PROMPT: &[u8] = "Message: ".as_bytes();

fn main() -> io::Result<()> {
    // bind to the port and start listening
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let port = &args[1];
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
    listener.set_nonblocking(true)?;
    println!("Server listening on port {}", port);
    let listener_fd = listener.as_raw_fd();

    // Initialize epoll. Listen for new tcpstreams coming
    let epoll_fd = syscall!(epoll_create1(0))?;
    syscall!(epoll_ctl(
        epoll_fd,
        libc::EPOLL_CTL_ADD,
        listener_fd,
        &mut libc::epoll_event {
            events: libc::EPOLLIN as u32,
            u64: 0,
        }
    ))?;

    // current client id, clients, and buffers
    let mut stream_id = 1;
    let mut tcp_streams = HashMap::new();
    let mut stream_buffers = HashMap::new();

    // global read buffer
    let mut data_recv = [0_u8; 1024];

    // epoll events
    let mut events: Vec<libc::epoll_event> = Vec::with_capacity(32);
    loop {
        // receive epoll events
        events.clear();
        let res = syscall!(epoll_wait(
            epoll_fd,
            events.as_mut_ptr() as *mut libc::epoll_event,
            32,
            -1,
        ))?;
        unsafe { events.set_len(res as usize) };

        for event in &events {
            match event.u64 {
                // new tcp stream(client)
                0 => match listener.accept() {
                    Ok((stream, addr)) => {
                        stream.set_nonblocking(true)?;
                        println!("New connection: {}", addr);
                        let stream_fd = stream.as_raw_fd();
                        tcp_streams.insert(stream_id, stream);
                        stream_buffers.insert(stream_id, Vec::from(PROMPT));
                        syscall!(epoll_ctl(
                            epoll_fd,
                            libc::EPOLL_CTL_ADD,
                            stream_fd,
                            &mut libc::epoll_event {
                                events: libc::EPOLLIN as u32,
                                u64: stream_id,
                            }
                        ))?;
                        stream_id += 1;
                    }
                    Err(e) => eprintln!("couldn't accept: {}", e),
                },
                // receive chat message
                id => {
                    let stream = tcp_streams.get(&id).unwrap();
                    let stream_fd = stream.as_raw_fd();
                    let mut stream = stream.try_clone().unwrap();

                    // receive while not empty
                    let mut first_recv = false;
                    while match stream.read(&mut data_recv) {
                        Ok(size) if size > 0 => {
                            first_recv = true;
                            let buffer = stream_buffers.get_mut(&id).unwrap();
                            for x in data_recv[0..size].iter() {
                                buffer.push(*x);
                                if *x == b'\n' {
                                    for (stream_id, mut stream) in &tcp_streams {
                                        if *stream_id != id {
                                            stream.write_all(buffer.as_slice()).unwrap();
                                        }
                                    }
                                    *buffer = Vec::from(PROMPT);
                                }
                            }
                            true
                        }
                        _ => {
                            if !first_recv {
                                println!(
                                    "Terminating connection with {}",
                                    stream.peer_addr().unwrap()
                                );
                                syscall!(epoll_ctl(
                                    epoll_fd,
                                    libc::EPOLL_CTL_DEL,
                                    stream_fd,
                                    std::ptr::null_mut::<epoll_event>()
                                ))?;
                                tcp_streams.remove(&id);
                                stream_buffers.remove(&id);
                                stream.shutdown(Shutdown::Both).unwrap();
                            }
                            false
                        }
                    } {}
                }
            }
        }
    }
}
