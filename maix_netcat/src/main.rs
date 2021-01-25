#[macro_use]
extern crate argh;
extern crate bstr;
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Protocol {
    TCP,
    UDP,
}

impl Protocol {
    fn connect_to(&self, socket_addr: std::net::SocketAddr) -> std::io::Result<Endpoint> {
        match self {
            Self::TCP => std::net::TcpStream::connect(socket_addr).map(Endpoint::TCP),
            Self::UDP => std::net::UdpSocket::bind(socket_addr)
                .map(|socket| Endpoint::UDP(socket, (0, 0, [0; 65507]))),
        }
    }
}

impl argh::FromArgValue for Protocol {
    fn from_arg_value(value: &str) -> Result<Self, String> {
        match value.trim().to_lowercase().as_str() {
            "u" | "udp" => Ok(Self::UDP),
            "t" | "tcp" => Ok(Self::TCP),
            _ => Err(String::from(
                "The argument must be either 'tcp'('t') or 'udp'('u')",
            )),
        }
    }
}

#[derive(FromArgs, Debug, Eq, PartialEq, Clone)]
/// A simple netcat clone written in rust
struct Options {
    #[argh(option, short = 'p', default = "Protocol::TCP")]
    /// choose between udp and tcp
    pub protocol: Protocol,

    #[argh(positional)]
    /// the socket address to reach
    socket_addr: String,
    #[argh(positional)]
    /// the socket port
    port: u16,
}

fn main() {
    let option: Options = argh::from_env();
    use std::net::ToSocketAddrs;
    let mut socket_addr = match (option.socket_addr, option.port).to_socket_addrs() {
        Ok(i) => i,
        Err(e) => {
            println!("{}", e);
            std::process::exit(1)
        }
    };
    let socket_addr: std::net::SocketAddr = match socket_addr.next() {
        Some(ip) => ip,
        None => {
            println!("No Ip Adress where given");
            std::process::exit(1)
        }
    };
    let mut endpoint = match option.protocol.connect_to(socket_addr) {
        Ok(e) => e,
        Err(e) => {
            println!("Error while connecting to endpoint: {}", e);
            std::process::exit(1)
        }
    };
    use std::io::prelude::*;
    let (send, recv) = std::sync::mpsc::channel::<([u8; 150], usize)>();
    let _input_handler = std::thread::spawn(move || {
        let send = send;
        let mut internal_buffer = String::with_capacity(150);
        let mut remaining;
        let mut start;
        let stdin = std::io::stdin();
        loop {
            start = 0;
            remaining = stdin
                .read_line(&mut internal_buffer)
                .expect("Error while reading STDIN");
            while remaining > 0 {
                let mut new_buffer = [0u8; 150];
                let written = (&mut new_buffer[..])
                    .write(&internal_buffer.as_bytes()[start..])
                    .expect("Error while writting to internal buffer");
                remaining -= written;
                start += written;
                send.send((new_buffer, written))
                    .expect("Error while sending input");
            }
        }
    });
    let mut bstring_buffer: bstr::BString = Default::default();
    loop {
        bstring_buffer.clear();
        let _ = endpoint
            .read(bstring_buffer.as_mut_slice())
            .expect("Error while reading data from socket");
        print!("{:?}", bstring_buffer);
        let data = recv.try_recv();
        if let Err(std::sync::mpsc::TryRecvError::Empty) = data {
            continue;
        }
        let (data, len) = data.expect("Error while manipulating input");
        let real_data = &data[..=len];
        endpoint
            .write(real_data)
            .expect("Error while sending input");
    }
}

#[derive(Debug)]
enum Endpoint {
    TCP(std::net::TcpStream),
    UDP(
        std::net::UdpSocket,
        (
            usize,       /*start*/
            usize,       /*end*/
            [u8; 65507], /*buffer*/
        ),
    ),
}

impl std::io::Write for Endpoint {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::TCP(t) => t.write(buf),
            Self::UDP(u, ..) => u.send(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::TCP(t) => t.flush(),
            Self::UDP(_, _) => Ok(()),
        }
    }
}

impl std::io::Read for Endpoint {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        use std::io::prelude::*;
        let mut buf = buf;
        match self {
            Self::TCP(t) => t.read(buf),
            Self::UDP(u, ref mut inner_buffer) => {
                if inner_buffer.0 >= inner_buffer.1 {
                    inner_buffer.0 = 0;
                    let written = u.recv(&mut inner_buffer.2)?;
                    inner_buffer.1 = written;
                    println!("\nread {} from udp socket", written);
                }
                let written = buf.write(&inner_buffer.2[(inner_buffer.0)..(inner_buffer.1)])?;
                inner_buffer.0 += written;
                Ok(written)
            }
        }
    }
}
