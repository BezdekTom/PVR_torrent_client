use anyhow::Ok;
use torrent_client::tracker_connection::request_peers;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let torrent_path="/home/tom/VSB/ing/3-semestr/pvr/torrent_client/data/debian-12.8.0-amd64-netinst.iso.torrent";

    let torrent_file = lava_torrent::torrent::v1::Torrent::read_from_file(torrent_path)?;

    let peer_id = "1234567890asdfghjklq".to_string();
    let port: u16 = 6881;

    let peers = request_peers(&torrent_file, peer_id, port).await?;

    for (i, p) in peers.iter().enumerate() {
        println!("Pear: {}  address: {}", i, p.addr)
    }

    Ok(())
}
