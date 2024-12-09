use anyhow::{Context, Error, Ok};
use lava_torrent::torrent::v1::Torrent;
use lava_torrent::tracker::Peer;
use reqwest::Url;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

use bytes::{Buf, BytesMut};

use crate::peer_id::PeerId;
use crate::tracker_response::TrackerResponse;

pub fn discover_peers(
    torrent_file: &Torrent,
    peer_id: &PeerId,
    port: u16,
) -> anyhow::Result<TrackerResponse> {
    eprintln!("Discover peers");
    let url = torrent_file.announce.clone();
    let url = match url {
        Some(url) => url,
        None => {
            return Err(Error::msg("No announce in torrent file"));
        }
    };

    let announce_url = Url::parse(&url)?;

    let tracker_response = match announce_url.scheme() {
        "http" | "https" => TrackerResponse::get_from_http(torrent_file, peer_id, port),
        "udp" => discover_udp_peers(torrent_file, &url, peer_id),
        _ => Err(Error::msg(format!(
            "Unsupported tracker protocol: {}",
            announce_url.scheme()
        ))),
    };

    tracker_response
}

fn discover_udp_peers(
    torrent: &Torrent,
    announce: &str,
    peer_id: &PeerId,
) -> anyhow::Result<TrackerResponse> {
    eprintln!("Discover udp peers");
    let announce_url = Url::parse(announce)?;
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(announce_url.socket_addrs(|| Some(6969))?[0])?;

    let transaction_id = rand::random::<u32>();
    let connection_id = 0x41727101980u64; // Default UDP tracker connection ID

    // Prepare connection request
    let mut connection_req = Vec::new();
    connection_req.extend_from_slice(&connection_id.to_be_bytes());
    connection_req.extend_from_slice(&(0u32.to_be_bytes())); // Connect action
    connection_req.extend_from_slice(&transaction_id.to_be_bytes());

    socket.send(&connection_req)?;

    // Read connection response
    let mut connection_resp = [0u8; 16];
    socket.recv(&mut connection_resp)?;

    // Extract connection response details
    let resp_transaction_id = u32::from_be_bytes(connection_resp[4..8].try_into()?);
    let resp_connection_id = u64::from_be_bytes(connection_resp[8..16].try_into()?);

    if resp_transaction_id != transaction_id {
        return Err(Error::msg("Invalid transaction ID"));
    }

    // Prepare announce request
    let mut announce_req = Vec::new();
    announce_req.extend_from_slice(&resp_connection_id.to_be_bytes());
    announce_req.extend_from_slice(&(1u32.to_be_bytes())); // Announce action
    announce_req.extend_from_slice(&transaction_id.to_be_bytes());
    announce_req.extend_from_slice(&torrent.info_hash_bytes());
    announce_req.extend_from_slice(&peer_id.as_bytes());
    announce_req.extend_from_slice(&0u64.to_be_bytes()); // downloaded
    announce_req.extend_from_slice(&(torrent.length as u64).to_be_bytes()); // left
    announce_req.extend_from_slice(&0u64.to_be_bytes()); // uploaded
    announce_req.extend_from_slice(&(2u32.to_be_bytes())); // event: started
    announce_req.extend_from_slice(&0u32.to_be_bytes()); // IP address
    announce_req.extend_from_slice(&0u32.to_be_bytes()); // key
    announce_req.extend_from_slice(&(-1i32).to_be_bytes()); // num_want: default

    socket.send(&announce_req)?;
    eprintln!("Request sended");

    // Read announce response
    let mut announce_resp = [0u8; 1024];
    let resp_size = socket.recv(&mut announce_resp)?;
    eprintln!("Response size: {}", &resp_size);

    // Parse peers from response
    // parse_udp_response(&announce_resp[..resp_size])
    todo!()
}

fn parse_udp_peers(response: &[u8]) -> anyhow::Result<Vec<Peer>> {
    let mut peers = Vec::new();

    // Skip first 12 bytes (action, transaction ID, interval, leechers, seeders)
    let peer_bytes = &response[12..];

    for chunk in peer_bytes.chunks(6) {
        if chunk.len() == 6 {
            let ip = Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
            let port = u16::from_be_bytes([chunk[4], chunk[5]]);

            peers.push(Peer {
                id: None,
                addr: SocketAddr::new(IpAddr::V4(ip), port),
                extra_fields: None,
            });
        }
    }

    Ok(peers)
}

fn parse_udp_response(response: &[u8]) -> anyhow::Result<Vec<Peer>> {
    let mut buf = BytesMut::from(response);

    // Read the connection ID
    let connection_id = buf.get_u64();

    // Read the action
    let action = buf.get_u32();

    // Read the transaction ID
    let transaction_id = buf.get_u32();

    // Read the interval
    let interval = buf.get_u32();

    // Read the number of leechers and seeders
    let leechers = buf.get_u32();
    let seeders = buf.get_u32();

    // Read peers
    let mut peers = Vec::new();
    while buf.has_remaining() {
        let ip = Ipv4Addr::new(buf.get_u8(), buf.get_u8(), buf.get_u8(), buf.get_u8());
        let port = buf.get_u16();
        peers.push(Peer {
            id: None,
            addr: SocketAddr::new(IpAddr::V4(ip), port),
            extra_fields: None,
        });
    }

    eprintln!("ConnectionID: {connection_id}, Action: {action}, transaction_id: {transaction_id}, interval: {interval}, leechers: {leechers}, seeders: {seeders}");
    Ok(peers)
}
