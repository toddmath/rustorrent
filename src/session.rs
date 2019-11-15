use crate::metadata::Torrent;
use crate::http_client::{self, AnnounceQuery, AnnounceResponse};

// enum State {
//     Handshaking,
//     Downloading
// }

use crate::http_client::{Peers,Peers6};
use std::io::Cursor;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6, Ipv4Addr, Ipv6Addr, ToSocketAddrs};

// struct Peers {
//     addrs: Vec<SocketAddr>
// }

fn get_peers_addrs(response: &AnnounceResponse) -> Vec<SocketAddr> {
    let mut addrs = Vec::new();
    
    match response.peers6 {
        Some(Peers6::Binary(ref bin)) => {
            addrs.reserve(bin.len() / 18);
            
            for chunk in bin.chunks_exact(18) {
                let mut cursor = Cursor::new(chunk);
                let mut addr: [u16; 9] = [0; 9];
                for i in 0..9 {
                    match cursor.read_u16::<BigEndian>() {
                        Ok(value) => addr[i] = value,
                        _ => continue
                    }                
                }
                let ipv6 = Ipv6Addr::new(addr[0], addr[1], addr[2], addr[3], addr[4], addr[5], addr[6], addr[7]);
                let ipv6 = SocketAddrV6::new(ipv6, addr[8], 0, 0);

                addrs.push(ipv6.into());
            }
        }
        Some(Peers6::Dict(ref peers)) => {
            addrs.reserve(peers.len());
            
            for peer in peers {
                match (peer.ip.as_str(), peer.port).to_socket_addrs() {
                    Ok(socks) => {
                        for addr in socks {
                            addrs.push(addr);
                        }
                    }
                    Err(e) => {
                        // TODO: report to the user
                    }
                }
            }
        }
        _ => {}
    }
    
    match response.peers {
        Some(Peers::Binary(ref bin)) => {
            addrs.reserve(bin.len() / 6);
            
            for chunk in bin.chunks_exact(6) {
                let mut cursor = Cursor::new(&chunk[..]);

                let ipv4 = match cursor.read_u32::<BigEndian>() {
                    Ok(ipv4) => ipv4,
                    _ => continue
                };
                
                let port = match cursor.read_u16::<BigEndian>() {
                    Ok(port) => port,
                    _ => continue
                };
                
                let ipv4 = SocketAddrV4::new(Ipv4Addr::from(ipv4), port);

                addrs.push(ipv4.into());
            }
        }
        Some(Peers::Dict(ref peers)) => {
            addrs.reserve(peers.len());
            
            for peer in peers {
                match (peer.ip.as_str(), peer.port).to_socket_addrs() {
                    Ok(socks) => {
                        for addr in socks {
                            addrs.push(addr);
                        }
                    }
                    Err(e) => {
                        // TODO: report to the user
                    }
                }
            }
        }
        _ => {}
    }

    println!("ADDRS: {:#?}", addrs);
    addrs
}

use std::io::prelude::*;
use std::net::TcpStream;
use smallvec::SmallVec;
use std::io::{BufReader, BufWriter};

fn read_messages(mut stream: TcpStream) -> std::result::Result<(), std::io::Error> {
    let mut buffer = Vec::with_capacity(32_768);

    //let mut stream = BufReader::with_capacity(32_768, stream);

    let mut i = 0;
    loop {
        let stream = std::io::Read::by_ref(&mut stream);

        println!("READING LENGTH", );
        
        buffer.clear();
        match stream.take(4).read_to_end(&mut buffer) {
            Ok(0) => return Ok(()),
            Err(e) => {
                println!("ERROR: {:?}", e);
                return Ok(());
            }
            _ => {}
        }

        let length = {
            // println!("LEN BUF = {:?}", buffer.len());
            let mut cursor = Cursor::new(&buffer[..]);
            cursor.read_u32::<BigEndian>()? as u64
        };
        
        println!("LENGTH={} {:?}", length, &buffer[..]);

        if length == 0 {
            continue;
        } // else if length >= buffer.capacity() {
        //     buffer.reserve(buffer.capacity() - length);
        // }

        buffer.clear();

        stream.take(length).read_to_end(&mut buffer)?;
        //stream.read_exact(&mut buffer[..length]);

        println!("ICIIII", );

        let mut last_have = 0;

        match buffer[0] {
            0 => {
                println!("CHOKE {:?} {:?}", String::from_utf8_lossy(&buffer[1..]), &buffer[..]);
                // let mut aa: [u8; 5] = [0; 5];
                // let mut cursor = Cursor::new(&mut aa[..]);
                // cursor.write_u32::<BigEndian>(1)?;
                // cursor.write_u8(2)?;
                                
                // stream.write_all(&aa)?;
                // stream.flush()?;

                // println!("INTERESTED SENT");

                // // request: <len=0013><id=6><index><begin><length>
                
                // let mut aa: [u8; 13] = [0; 13];
                // let mut cursor = Cursor::new(&mut aa[..]);
                // cursor.write_u32::<BigEndian>(13)?;
                // cursor.write_u8(6)?;
                // cursor.write_u32::<BigEndian>(0)?;
                // cursor.write_u32::<BigEndian>(256)?;
                                
                // stream.write_all(&aa)?;
                // stream.flush()?;

                // println!("REQUEST SENT");
            }
            1 => {
                println!("UNCHOKE", );
                
                let mut aa: [u8; 17] = [0; 17];
                let mut cursor = Cursor::new(&mut aa[..]);
                cursor.write_u32::<BigEndian>(13)?;
                cursor.write_u8(6)?;
                cursor.write_u32::<BigEndian>(last_have)?;
                cursor.write_u32::<BigEndian>(0)?;
                cursor.write_u32::<BigEndian>(16384)?;
                                
                stream.write_all(&aa)?;
                stream.flush()?;

                println!("REQUEST SENT");
            }
            2 => println!("INTERESTED", ),
            3 => println!("NOT INTERESTED", ),
            4 => {
                //cursor.set_position(1);
                let mut cursor = Cursor::new(&buffer[1..]);
                last_have = cursor.read_u32::<BigEndian>()?;
                println!("HAVE {:?}", last_have);
            }
            5 => {
                println!("BITFIELD {:?}", &buffer[1..]);
                
                let mut aa: [u8; 5] = [0; 5];
                let mut cursor = Cursor::new(&mut aa[..]);
                cursor.write_u32::<BigEndian>(1)?;
                cursor.write_u8(2)?;
                                
                stream.write_all(&aa)?;
                stream.flush()?;

                println!("INTERESTED SENT");
            }
            6 => {
                println!("REQUEST {:?}", &buffer[1..]);
            }
            7 => {
                // piece: <len=0009+X><id=7><index><begin><block>
                
                let mut cursor = Cursor::new(&buffer[1..]);

                let index = cursor.read_u32::<BigEndian>()?;
                let begin = cursor.read_u32::<BigEndian>()?;
                
                println!("PIECE ! {:?} {:?}", index, begin);
            }
            x => { println!("UNKNOWN {} {:?}", x, &buffer[1..]); }
        }
        i += 1;
        // if i >= 6 {
        //     return Ok(())
        // }
    }
}

// fn read_messages(mut stream: TcpStream) {
//     //let mut buffer = [0; 4096];
//     let mut buffer = Vec::with_capacity(4096);
//     //let mut buffer = BufReader::new(stream);
//     //let mut cursor = Cursor::new(&buffer[..]);
    
//     loop {
// //        buffer.read_exact();
//         stream.read_exact(&mut buffer[..4]);

//         let length = {
//             let mut cursor = Cursor::new(&buffer[..]);
//             cursor.read_u32::<BigEndian>().unwrap() as usize
//         };

//         if length == 0 {
//             continue;
//         } else if length >= buffer.capacity() {
//             buffer.reserve(buffer.capacity() - length);
//         }
        
//         stream.read_exact(&mut buffer[..length]);

//         match buffer[0] {
//             0 => println!("CHOKE", ),
//             1 => println!("UNCHOKE", ),
//             2 => println!("INTERESTED", ),
//             3 => println!("NOT INTERESTED", ),
//             4 => {
//                 //cursor.set_position(1);
//                 let mut cursor = Cursor::new(&buffer[1..]);
//                 println!("HAVE {:?}", cursor.read_u32::<BigEndian>());
//             }
//             5 => {
//                 println!("BITFIELD", );
//             }
//             x => { println!("UNKNOWN {}", x); }
//         }
//     }
// }

fn do_handshake(addr: &SocketAddr, torrent: &Torrent) -> std::result::Result<(), std::io::Error> {
    let mut stream = TcpStream::connect_timeout(addr, std::time::Duration::from_secs(5))?;

    let mut handshake: [u8; 68] = [0; 68];

    {
        let mut cursor = Cursor::new(&mut handshake[..]);

        cursor.write(&[19]);
        cursor.write(b"BitTorrent protocol");
        cursor.write(&[0,0,0,0,0,0,0,0]);
        cursor.write(torrent.info_hash.as_ref());
        cursor.write(b"-RT1220sJ1Nna5rzWLd8");
    }

    stream.set_write_timeout(Some(std::time::Duration::from_secs(30)));
    
    stream.write_all(&handshake)?;
    stream.flush()?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30)));

    // TODO: Use SmallVec here
    let mut buffer = [0; 128];

    stream.read_exact(&mut buffer[..1]);

    let len = buffer[0] as usize;

    stream.read_exact(&mut buffer[..len + 48]);

    if &buffer[len + 8..len + 28] == torrent.info_hash.as_slice() {
        //println!("HASH MATCHED !", );
    }

    read_messages(stream)
}

use url::Url;

// struct TrackersList {
//     list: Vec<Tracker>
// }

// impl From<&Torrent> for TrackersList {
//     fn from(torrent: &Torrent) -> TrackersList {
//         TrackersList {
//             list: torrent.iter_urls().map(Tracker::new).collect()
//         }
//     }
// }

use crate::http_client::HttpError;
use crossbeam_channel::{unbounded, Receiver, Sender};

#[derive(Debug)]
enum TorrentError {
    Http(HttpError),
    IO(std::io::Error)
}

impl From<HttpError> for TorrentError {
    fn from(e: HttpError) -> TorrentError {
        match e {
            HttpError::IO(e) => TorrentError::IO(e),
            e => TorrentError::Http(e)
        }
    }
}

struct Tracker {
    url: Url,
    announce: Option<AnnounceResponse>
}

impl Tracker {
    fn new(url: Url) -> Tracker {
        Tracker { url, announce: None }
    }

    fn announce(&mut self, torrent: &Torrent) -> Result<Vec<SocketAddr>> {
        let query = AnnounceQuery::from(torrent);
        let response = http_client::get(&self.url, query)?;

        let peers = get_peers_addrs(&response);
        self.announce = Some(response);
        
        Ok(peers)
    }
}

enum PeerState {
    Connecting,
    Handshaking,
    Downloading {
        piece: usize,
        index: usize,
    },
    Dead
}

struct Peer {
    addr: SocketAddr,
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
    state: PeerState
}

enum MessageActor {
    AddPeer(PeerAddr),
    RemovePeer(PeerAddr),
}

type PeerAddr = Sender<MessageActor>;

struct TorrentData {
    torrent: Torrent
}

impl TorrentData {
    fn new(torrent: Torrent) -> TorrentData {
        TorrentData { torrent }
    }
}

struct TorrentActor {
    data: Arc<RwLock<TorrentData>>,
    peers: Vec<PeerAddr>,
    trackers: Vec<Tracker>,
    receiver: Receiver<MessageActor>,
    // We keep a Sender to not close the channel
    // in case there is no peer
    sender: Sender<MessageActor>,
}

type Result<T> = std::result::Result<T, TorrentError>;

use std::sync::{Arc, RwLock};

impl TorrentActor {
    fn new(torrent: Torrent) -> TorrentActor {
        let (sender, receiver) = unbounded();
        TorrentActor {
            data: Arc::new(RwLock::new(TorrentData::new(torrent))),
            receiver,
            sender,
            peers: vec![],
            trackers: vec![],            
        }
    }
    
    fn start(&mut self) {
        self.collect_trackers();
        self.connect();
    }

    fn collect_trackers(&mut self) {
        let data = self.data.read().unwrap();
        self.trackers = data.torrent.iter_urls().map(Tracker::new).collect();
    }
    
    fn connect(&mut self) -> Result<()> {
        let torrent = &self.torrent;

        for tracker in &mut self.trackers {
            let addrs = match tracker.announce(&torrent) {
                Ok(peers) => peers,
                Err(e) => {
                    eprintln!("[Tracker announce] {:?}", e);
                    continue;
                }
            };
            
            for addr in &addrs {
                println!("ADDR: {:?}", addr);
                
                std::thread::spawn(|| {
                    let res = do_handshake(addr, torrent);
                    println!("RES: {:?}", res);
                });
                
            }
           
        }

        Ok(())
        // for url in self.torrent.iter_urls().filter(|url| url.scheme() == "http") {
        //     // println!("URL={:?}", url);
            
        //     let query = AnnounceQuery::from(torrent);
        //     let res: Result<AnnounceResponse,_> = http_client::get(url, query);

        //     if let Ok(ref res) = res {
        //         let addrs = get_peers_addrs(res);

        //         for addr in &addrs {
        //             println!("ADDR: {:?}", addr);
        //             let res = do_handshake(addr, torrent);
        //             println!("RES: {:?}", res);
        //         }
        //     };
        // }
    }
}

pub struct Session {
    actors: Vec<TorrentActor>
}

impl Session {
    pub fn new() -> Session {
        Session { actors: vec![] }
    }
    
    pub fn add_torrent(&mut self, torrent: Torrent) {
        println!("TORRENT={:#?}", torrent);

        let mut actor = TorrentActor::new(torrent);

        actor.start();

        // let query = AnnounceQuery::from(&torrent);

        // let res: AnnounceResponse = http_client::get(&torrent.meta.announce, query).unwrap();

        // println!("RESPONSE: {:?}", res);
    }
}