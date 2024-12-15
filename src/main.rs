use std::{
    env::{self},
    path::Path,
};

use anyhow::Ok;
use torrent_client::tui::run_tui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <torrent_file_path>", args[0]);
        anyhow::bail!("Invalid params");
    }
    let torrent_file_path = &args[1];

    let download_folder_path;
    if args.len() == 3 {
        download_folder_path = args[2].clone();
    } else {
        let file_path = Path::new(torrent_file_path).parent();
        if let Some(parent_path) = file_path {
            download_folder_path = parent_path.to_str().unwrap().to_string();
        } else {
            eprintln!(
                "Usage: {} <torrent_file_path> <download folder path>",
                args[0]
            );
            anyhow::bail!("Invalid params");
        }
    }

    run_tui(torrent_file_path, download_folder_path).await?;
    Ok(())

    // let torrent_path="/home/tom/VSB/ing/3-semestr/pvr/torrent_client/data/linuxmint-22-cinnamon-64bit.iso.torrent";
    // let torrent_path ="/home/tom/VSB/ing/3-semestr/pvr/torrent_client/data/debian-12.8.0-amd64-netinst.iso.torrent";
    // let torrent_path ="/home/tom/VSB/ing/3-semestr/pvr/torrent_client/data/ubuntu-24.04.1-desktop-amd64.iso.torrent";
    // let torrent_path = "/home/tom/VSB/ing/3-semestr/pvr/torrent_client/data/music.torrent";
}
