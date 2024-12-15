use crate::peer_id::PeerId;
use crate::tracker_connection::tracker_response::TrackerResponse;
use anyhow::Context;
use lava_torrent::torrent::v1::Torrent;
use lava_torrent::tracker::TrackerResponse as LavaTrackerResponse;
use serde::Serialize;

/// Structure representing HTTP request to torrent tracker
#[derive(Debug, Clone, Serialize)]
pub struct HttpTrackerRequest {
    /// 20 character long string
    peer_id: String,

    /// Port client listening on
    port: u16,

    /// The total amount downloaded
    uploaded: usize,

    /// The total amount yet downloaded
    downloaded: usize,

    /// The number left to download
    left: usize,

    /// set to one if the peer list should bee compact
    /// set to 1 if compact should be used
    /// compact is more common, therefor I will use compact
    compact: u8,
}

impl HttpTrackerRequest {
    /// Creates new HTTP request based on information from `torrent file`, given `peer id` and `port`.
    pub fn new(torrent: &Torrent, peer_id: &PeerId, port: u16) -> Self {
        HttpTrackerRequest {
            peer_id: peer_id.to_string(),
            port,
            uploaded: 0,
            downloaded: 0,
            left: torrent.length as usize,
            compact: 1,
        }
    }
}

/// Send request to torrent tracker and accept response
async fn tracker_request(
    torrent: &Torrent,
    peer_id: &PeerId,
    port: u16,
) -> anyhow::Result<TrackerResponse> {
    let request = HttpTrackerRequest::new(torrent, peer_id, port);
    let url_params =
        serde_urlencoded::to_string(&request).context("Failed to urlencode parameters")?;
    let tracker_url = format!(
        "{}?{}&info_hash={}",
        torrent
            .announce
            .clone()
            .context("No announced in torrent file")?,
        url_params,
        &urlencode(&torrent.info_hash_bytes())
    );

    let response = reqwest::get(tracker_url)
        .await
        .context("Sending get request")?;
    let response = response
        .bytes()
        .await
        .context("Getting bytes from response")?;

    match LavaTrackerResponse::from_bytes(response) {
        Ok(response) => match response {
            LavaTrackerResponse::Success {
                interval, peers, ..
            } => Ok(TrackerResponse {
                interval: interval as usize,
                peers,
            }),
            LavaTrackerResponse::Failure { reason } => Err(anyhow::Error::msg(format!(
                "Failed to parse tracker response, reason: {:?}",
                reason
            ))),
        },
        Err(e) => Err(e.into()),
    }
}

impl TrackerResponse {
    /// Get tracker response from http torrent tracker
    pub async fn get_from_http(
        torrent: &Torrent,
        peer_id: &PeerId,
        port: u16,
    ) -> anyhow::Result<Self> {
        tracker_request(torrent, peer_id, port).await
    }
}

/// Function that encode byte array to string correctly for URL request.
/// Neccesary for sending `info_hash`.
fn urlencode(t: &Vec<u8>) -> String {
    let mut encoded = String::with_capacity(3 * t.len());
    for &byte in t {
        encoded.push('%');
        encoded.push_str(&hex::encode([byte]));
    }
    encoded
}
