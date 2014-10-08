extern crate csv;
extern crate serialize;

use std::os;
use std::path::Path;
use std::io::{File,BufferedReader};
use std::io::net::udp::UdpSocket;
use std::io::net::ip::{Ipv4Addr, SocketAddr};
use std::str::from_utf8;

static PORT: u16 = 8888;
static UDP_PAYLOAD: uint = 900;

#[deriving(Show,Encodable)]
struct Record {
    variance: Option<f32>,
    frequency: Option<f32>,
}


fn main() {
    let args = os::args();
    if args.len() != 2 {
        fail!("You need to specify filename.");
    }
    let filename = args[1].as_slice();
    let fp = &Path::new(filename);
    let mut reader = BufferedReader::new(File::open(fp));
    let header = match reader.read_exact(512) {
        Err(_) => fail!("Couldn't read from file"),
        Ok(s)  => match String::from_utf8(s) {
            Ok(s)  => s,
            Err(_) => fail!("Incorrectly encoded data"),
        },
    };
    let mut values: Vec<i16> = Vec::new();
    loop {
        let res = match reader.read_le_i16() {
            Ok(v)  => v,
            Err(_) => break,
        };
        values.push(res);
    }
    let mut ys = values
                .as_slice()
                .chunks(3)
                .filter_map(|xs| if xs.len() == 3 {Some(xs[1])} else {None});

    println!("Opening UDP socket...");
    let local = SocketAddr { ip: Ipv4Addr(10, 1, 1, 1), port: PORT };
    let remote = SocketAddr { ip: Ipv4Addr(10, 1, 1, 2), port: PORT };
    let mut socket = match UdpSocket::bind(local) {
        Ok(s) => s,
        Err(e) => fail!("Couldn't bind socket: {}", e),
    };

    println!("Sending data...");
    let mut output_buffer: [u8, ..UDP_PAYLOAD] = [0, ..UDP_PAYLOAD];
    let mut input_buffer: [u8, ..UDP_PAYLOAD] = [0, ..UDP_PAYLOAD];
    let mut results = Vec::new();
    let mut i = 0u;
    let mut counter = 0u;

    for y in ys {
        let h = (y >> 8) as u8;
        let l = (y & 0xff) as u8;
        output_buffer[i] = h;
        output_buffer[i+1] = l;
        i += 2;
        i %= UDP_PAYLOAD;

        if i == 0 {
            match socket.send_to(output_buffer, remote) {
                Ok(_) => (),
                Err(e) => fail!("Couldn't send packet: {}", e),
            };
            counter += 1;
            counter %= 2;
            
            if counter == 0 {
                let resp = match socket.recv_from(input_buffer) {
                    Ok(_) => from_utf8(input_buffer).unwrap(),
                    Err(e) => fail!("Couldn't receive a packet: {}", e),
                };
                let parsed: Vec<&str> = resp
                                        .as_slice()
                                        .lines()
                                        .next()
                                        .unwrap()
                                        .split(',')
                                        .map(|s| s.trim())
                                        .collect();
                results.push(Record {
                    variance: from_str(parsed[0]),
                    frequency: from_str(parsed[1]),
                });
            }
        }
    }

    drop(socket);
    
    println!("Writing results to file...");
    let out_fp = &Path::new("features.csv");
    let mut writer = csv::Writer::from_file(out_fp);
    for r in results.iter() {
        let _ = writer.encode(r);
    }
}
