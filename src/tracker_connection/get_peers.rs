use anyhow::Error;
use lava_torrent::torrent::v1::Torrent;
use reqwest::Url;

use crate::peer_id::PeerId;
use crate::tracker_connection::tracker_response::TrackerResponse;

/// Discover available peers from tracker.
/// Done based on informations from `torrent_file`.
/// User `peer_id` and `port` is needed.
pub async fn discover_peers(
    torrent_file: &Torrent,
    peer_id: &PeerId,
    port: u16,
) -> anyhow::Result<TrackerResponse> {
    let url = torrent_file.announce.clone();
    let url = match url {
        Some(url) => url,
        None => {
            anyhow::bail!("No announce in torrent file");
        }
    };

    let announce_url = Url::parse(&url)?;

    let tracker_response = match announce_url.scheme() {
        "http" | "https" => TrackerResponse::get_from_http(torrent_file, peer_id, port).await,
        // UDP is not working for now, will fail on todo!()
        "udp" => TrackerResponse::get_from_udp(torrent_file, peer_id).await,
        _ => Err(Error::msg(format!(
            "Unsupported tracker protocol: {}",
            announce_url.scheme()
        ))),
    };

    tracker_response
}
