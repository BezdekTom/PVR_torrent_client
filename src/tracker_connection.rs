use std::time::Duration;

use anyhow::{Error, Ok};
use lava_torrent::tracker::Peer;
use lava_torrent::{torrent::v1::Torrent, tracker::TrackerResponse};

use reqwest;
use url::Url;

// pub trait TorrentFile {
//     fn build_tracker_url(&self, peer_id: [u8; 20], port: u16) -> anyhow::Result<Url>;

//     async fn request_peers(&self, peer_id: [u8; 20], port: u16) -> anyhow::Result<Vec<Peer>>;
// }

// impl TorrentFile for Torrent {
fn build_tracker_url(torrent_file: &Torrent, peer_id: String, port: u16) -> anyhow::Result<Url> {
    let announce_url = torrent_file.announce.clone();
    let announce_url = match announce_url {
        Some(announce_url) => announce_url,
        None => {
            return Err(Error::msg("No announce in torrent file"));
        }
    };

    let left = torrent_file.piece_length * torrent_file.pieces.len() as i64;

    let mut url = Url::parse(&announce_url)?;
    url.query_pairs_mut()
        .append_pair("info_hash", &torrent_file.info_hash())
        .append_pair("peer_id", &peer_id)
        .append_pair("port", &port.to_string())
        .append_pair("uploaded", "0")
        .append_pair("downloaded", "0")
        .append_pair("left", &left.to_string())
        .append_pair("compact", "1");

    Ok(url)
}
pub async fn request_peers(
    torrent_file: &Torrent,
    peer_id: String,
    port: u16,
) -> anyhow::Result<Vec<Peer>> {
    let url = build_tracker_url(torrent_file, peer_id, port)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(5))
        .build()?;
    eprintln!("Client created, sending request");
    eprintln!("URL: {:?}", url.as_str());
    let response = client.get(url.as_str()).send().await?;
    eprintln!("Received response");

    // let body = response.text().await?;

    let response = lava_torrent::tracker::TrackerResponse::from_bytes(response.bytes().await?)?;
    eprintln!("Response transformed to bytes");

    match response {
        TrackerResponse::Success { peers, .. } => Ok(peers),
        TrackerResponse::Failure { reason } => Err(Error::msg(format!(
            "Invalid responcse from tracker, because: {reason}"
        ))),
    }
}
// }

fn generate_peer_id() -> String {
    format!("-RU0001-{:20}", rand::random::<u64>())
}
