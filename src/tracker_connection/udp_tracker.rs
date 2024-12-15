use super::tracker_response::TrackerResponse;
use crate::peer_id::PeerId;
use anyhow::Result;
use lava_torrent::{torrent::v1::Torrent, tracker::Peer};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};
use tokio::{net::UdpSocket, time::timeout};
use url::Url;

/// TODO: finish work on this, not working for now
impl TrackerResponse {
    #[allow(dead_code, unused)]
    pub async fn get_from_udp(torrent: &Torrent, peer_id: &PeerId) -> Result<Self> {
        let announce = torrent.announce.clone();
        let announce = match announce {
            Some(url) => url,
            None => {
                anyhow::bail!("No announce in torrent file");
            }
        };
        todo!()
        // discover_udp_peers(torrent, &announce, peer_id).await
    }
}

#[allow(dead_code)]
async fn discover_udp_peers(
    torrent: &Torrent,
    announce: &str,
    peer_id: &PeerId,
) -> anyhow::Result<TrackerResponse> {
    eprintln!("Discover udp peers");
    let announce_url = Url::parse(announce)?;
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    timeout(
        Duration::from_secs(5),
        socket.connect(announce_url.socket_addrs(|| Some(6969))?[0]),
    )
    .await??;

    let transaction_id = rand::random::<u32>();
    let connection_id = 0x41727101980u64; // Default UDP tracker connection ID

    // Prepare connection request
    let mut connection_req = Vec::new();
    connection_req.extend_from_slice(&connection_id.to_be_bytes());
    connection_req.extend_from_slice(&(0u32.to_be_bytes())); // Connect action
    connection_req.extend_from_slice(&transaction_id.to_be_bytes());

    eprintln!("Sending request");
    socket.send(&connection_req).await?;

    eprintln!("Receiving request answer");
    // Read connection response
    let mut connection_resp = [0u8; 16];
    socket.recv(&mut connection_resp).await?;

    eprintln!("Response received");
    // Extract connection response details
    let resp_transaction_id = u32::from_be_bytes(connection_resp[4..8].try_into()?);
    let resp_connection_id = u64::from_be_bytes(connection_resp[8..16].try_into()?);

    if resp_transaction_id != transaction_id {
        anyhow::bail!("Invalid transaction ID");
    }

    // Prepare announce request
    let mut announce_req = Vec::new();
    announce_req.extend_from_slice(&resp_connection_id.to_be_bytes());
    announce_req.extend_from_slice(&(1u32.to_be_bytes())); // Announce action
    announce_req.extend_from_slice(&transaction_id.to_be_bytes());
    announce_req.extend_from_slice(&torrent.info_hash_bytes());
    announce_req.extend_from_slice(peer_id.as_ref());
    announce_req.extend_from_slice(&0u64.to_be_bytes()); // downloaded
    announce_req.extend_from_slice(&(torrent.length as u64).to_be_bytes()); // left
    announce_req.extend_from_slice(&0u64.to_be_bytes()); // uploaded
    announce_req.extend_from_slice(&(2u32.to_be_bytes())); // event: started
    announce_req.extend_from_slice(&0u32.to_be_bytes()); // IP address
    announce_req.extend_from_slice(&0u32.to_be_bytes()); // key
    announce_req.extend_from_slice(&(-1i32).to_be_bytes()); // num_want: default

    socket.send(&announce_req).await?;
    eprintln!("Request sended");

    // Read announce response
    let mut announce_resp = [0u8; 1024];
    let resp_size = socket.recv(&mut announce_resp).await?;
    eprintln!("Response size: {}", &resp_size);

    // Parse peers from response
    parse_udp_response(&announce_resp[..resp_size])
}

#[allow(dead_code)]
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

#[allow(unused)]
fn parse_udp_response(response: &[u8]) -> anyhow::Result<TrackerResponse> {
    let action = u32::from_be_bytes(response[0..4].try_into()?);
    let transaction_id = u32::from_be_bytes(response[4..8].try_into()?);

    match action {
        1 => {
            // Announce response
            let interval = u32::from_be_bytes(response[8..12].try_into()?);
            let leechers = u32::from_be_bytes(response[12..16].try_into()?);
            let seeders = u32::from_be_bytes(response[16..20].try_into()?);
            let mut peers = Vec::new();

            let num_peers = (response.len() - 20) / 6;
            for i in 0..num_peers {
                let peer_offset = 20 + i * 6;
                let mut peer_ip = IpAddr::V4(Ipv4Addr::new(
                    response[peer_offset],
                    response[peer_offset + 1],
                    response[peer_offset + 2],
                    response[peer_offset + 3],
                ));
                let peer_port =
                    u16::from_be_bytes(response[peer_offset + 4..peer_offset + 6].try_into()?);
                peers.push(Peer {
                    id: None,
                    addr: SocketAddr::new(peer_ip, peer_port),
                    extra_fields: None,
                });
            }

            Ok(TrackerResponse {
                interval: interval as usize,
                peers,
            })
        }
        _ => {
            anyhow::bail!("Unexpected action in UDP tracker response: {}", action);
        }
    }
}
