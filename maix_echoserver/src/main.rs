#[macro_use]
extern crate log;
extern crate simplelog;
use std::net;
use std::thread;
enum Protocol {
    TCP,
    UDP,
}

fn main() {
    use simplelog::*;
    TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed).unwrap();

    let mut args_iter = std::env::args();
    args_iter.next();
    let protocol = args_iter
        .next()
        .map(|x| match x.trim().to_lowercase().as_str() {
            "tcp" => Ok(Protocol::TCP),
            "udp" => Ok(Protocol::UDP),
            _ => Err(x.to_string()),
        })
        .unwrap_or(Err(String::new()));
    if let Err(e) = protocol.as_ref() {
        error!("The given protocol needs to be 'TCP' or 'UDP' not {}", e);
        return;
    }

    let protocol = protocol.unwrap();
    let args = args_iter
        .map(|x| u16::from_str_radix(x.trim(), 10).map_err(|_| x.trim().to_string()))
        .collect::<Vec<Result<u16, String>>>();
    //dbg!(&args);
    let mut threads = Vec::new();
    for port in &args {
        if let Err(e) = port.as_ref() {
            warn!("Argument \"{}\" isn't a valid port, not testing", e);
            continue;
        }
        let port = port.clone().unwrap();
        threads.push(match protocol {
            Protocol::TCP => thread::spawn(handle_tcp(port)),
            Protocol::UDP => thread::spawn(handle_udp(port)),
        });
    }

    loop {}
}

fn handle_udp(port: u16) -> impl Fn() -> () {
    move || {
        let mut buffer = [0u8;65507];
        let ip = ("0.0.0.0", port);
        let socket = net::UdpSocket::bind(ip);
        if let Err(e) = socket.as_ref() {
            error!("Error when binding port \x1b[31m{}\x1b[0m: {}", port, e);
            return;
        }
        let socket = socket.unwrap();
        info!("Started listing on \x1b[32m{}\x1b[0m", port);
        loop {
            // Copied from the docs of std::net::UdpSocket

            // Receives a single datagram message on the socket. If `buf` is too small to hold
            // the message, it will be cut off.
            let (amt, src) = {
                let res = socket.recv_from(&mut buffer);
                if let Err(e) = res.as_ref() {
                    error!("Error with port \x1b[31m{}\x1b[0m: {}", port, e);
                    return;
                }
                res.unwrap()
            };
            info!("Recieved a packet on port \x1b[32m{}\x1b[0m", port);
            // Redeclare `buf` as slice of the received data and
            // send reverse data back to origin.
            let buf_resized = &mut buffer[..amt];
            let res = socket.send_to(buf_resized, &src);
            if let Err(e) = res.as_ref() {
                error!("Error with port \x1b[31m{}\x1b[0m: {}", port, e);
                return;
            }
        }
    }
}
fn handle_tcp(port: u16) -> impl Fn() -> () {
    /*return || {
        return error!("TCP echo server isn't implemented yet!");
    };*/
    let ip = ("0.0.0.0", port);
    let (sender, recv) = std::sync::mpsc::channel::<net::TcpStream>();
    let _recv_thread = std::thread::spawn(move || {
        let socket = net::TcpListener::bind(ip);
        if let Err(e) = socket.as_ref() {
            return error!("Error when binding port \x1b[31m{}\x1b[0m: {}", port, e);
        }
        let socket = socket.unwrap();
        for stream in socket.incoming() {
            if stream.is_err() {
                return error!("Error with port \x1b[31m{}\x1b[0m", port);
            }
            let stream = stream.unwrap();
            sender
                .send(stream)
                .expect(&format!("\x1b[13m{}\x1b[0m: Sending error", port));
        }
    });
    return move || {
        let mut buffer = [0u8;10];
        let mut clients: Vec<net::TcpStream> = Vec::new();
        loop {
            for new_client in recv.try_iter() {
                clients.push(new_client);
                info!("Connection on port \x1b[31m{}\x1b[0m", port);
            }
            for client in &mut clients {
                use std::io::prelude::*;
                let size = client.read(&mut buffer).unwrap();
                let new_buffer = &buffer[..size];
                client.write(new_buffer).unwrap();
            }
        }
    };
}


















