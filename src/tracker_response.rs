pub struct TrackerResponse {
    pub interval: usize,
    pub peers: Vec<lava_torrent::tracker::Peer>,
}
