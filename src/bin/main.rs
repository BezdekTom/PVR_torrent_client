use anyhow::Ok;
use torrent_client::{
    peer_id::PeerId, tracker_connection::discover_peers, tracker_response::TrackerResponse,
};

fn main() -> anyhow::Result<()> {
    // let torrent_path="/home/tom/VSB/ing/3-semestr/pvr/torrent_client/data/linuxmint-22-cinnamon-64bit.iso.torrent";
    // let torrent_path ="/home/tom/VSB/ing/3-semestr/pvr/torrent_client/data/debian-12.8.0-amd64-netinst.iso.torrent";
    let torrent_path ="/home/tom/VSB/ing/3-semestr/pvr/torrent_client/data/ubuntu-24.04.1-desktop-amd64.iso.torrent";

    let torrent_file = lava_torrent::torrent::v1::Torrent::read_from_file(torrent_path)?;

    let peer_id = PeerId::generate();
    eprintln!("Peer ID: {:?}", peer_id.to_string());
    let port: u16 = 6881;

    let TrackerResponse { interval, peers } = discover_peers(&torrent_file, &peer_id, port)?;

    for (i, p) in peers.iter().enumerate() {
        println!("Pear: {}  address: {}", i + 1, p.addr)
    }

    Ok(())
}
