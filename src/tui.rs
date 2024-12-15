use crate::{
    download::TorrentDownloader,
    peer_id::PeerId,
    tracker_connection::{get_peers::discover_peers, tracker_response::TrackerResponse},
};
use anyhow::Result;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Gauge, List, Paragraph},
    Terminal,
};
use std::io;
use tokio::{sync::mpsc, task::JoinHandle};

/// TUI that display information about current downloading in "nicer" format, than just print
/// No interactions from user are supported.
pub async fn run_tui(torrent_file_path: &str, download_folder_path: String) -> Result<()> {
    let backend = ratatui::backend::CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    let torrent_file = lava_torrent::torrent::v1::Torrent::read_from_file(torrent_file_path)?;
    let peer_id = PeerId::generate();
    let port: u16 = 6881;
    let TrackerResponse { interval: _, peers } =
        discover_peers(&torrent_file, &peer_id, port).await?;

    let (tx, mut rx) = mpsc::channel::<usize>(100);

    let tracker_announce = torrent_file.announce.clone().unwrap();
    let info_hash = torrent_file.info_hash();
    let tui_peers = peers.clone();
    let num_pieces = torrent_file.pieces.len();
    let target_name = torrent_file.name.clone();
    let mut downloaded_pieces = Vec::new();

    let download_folder_path = download_folder_path.to_string();

    let download_task: JoinHandle<Result<()>> = tokio::spawn(async move {
        let downloader = TorrentDownloader::new(torrent_file)?;
        downloader
            .download_torrent(peers, &peer_id, download_folder_path, tx)
            .await?;
        Ok(())
    });

    loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(5),
                        Constraint::Percentage(10),
                        Constraint::Percentage(10),
                        Constraint::Percentage(10),
                        Constraint::Percentage(25),
                        Constraint::Percentage(15),
                        Constraint::Percentage(25),
                    ]
                    .as_ref(),
                )
                .split(size);

            // PVR - Torrent client
            let pvr_block = Paragraph::new("PVR - Torrent client")
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::new().blue().bold());
            f.render_widget(pvr_block, chunks[0]);

            // Downloading
            let downloading_block = Block::default().title("Downloading").borders(Borders::ALL);
            let downloading_paragraph =
                Paragraph::new(format!("{torrent_file_path} -> {target_name}"))
                    .alignment(ratatui::layout::Alignment::Left)
                    .block(downloading_block);
            f.render_widget(downloading_paragraph, chunks[1]);

            // Tracker Announce
            let tracker_announce_block = Block::default()
                .title("Tracker Announce")
                .borders(Borders::ALL);
            let tracker_announce_paragraf =
                Paragraph::new(tracker_announce.as_str()).block(tracker_announce_block);
            f.render_widget(tracker_announce_paragraf, chunks[2]);

            // Info hash
            let tracker_announce_block = Block::default().title("Info Hash").borders(Borders::ALL);
            let tracker_announce_paragraf =
                Paragraph::new(info_hash.as_str()).block(tracker_announce_block);
            f.render_widget(tracker_announce_paragraf, chunks[3]);

            // Peers
            let peers_block = Block::default()
                .title(format!("Peers ({})", tui_peers.len()))
                .borders(Borders::ALL);
            let peers_list = List::new(
                tui_peers
                    .iter()
                    .map(|peer| format!("{}", peer.addr))
                    .collect::<Vec<_>>(),
            )
            .block(peers_block);
            f.render_widget(peers_list, chunks[4]);

            let downloaded_gauge = Gauge::default()
                .gauge_style(Style::default().fg(Color::Green))
                .ratio(downloaded_pieces.len() as f64 / num_pieces as f64);

            // Downloaded File
            let downloaded_block = Block::default().title("Downloaded").borders(Borders::ALL);

            f.render_widget(downloaded_gauge.block(downloaded_block), chunks[5]);

            let downloaded_pieces_block = Block::default()
                .title(format!(
                    "Downloaded pieces (downloaded: {}, to download: {}, totaly {})",
                    downloaded_pieces.len(),
                    num_pieces - downloaded_pieces.len(),
                    num_pieces
                ))
                .borders(Borders::ALL);
            let downloaded_pieces_list = List::new(
                downloaded_pieces
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(i, piece)| format!("Piece {} ({}/{})", piece, i + 1, num_pieces))
                    .collect::<Vec<_>>(),
            )
            .block(downloaded_pieces_block);
            f.render_widget(downloaded_pieces_list, chunks[6]);
        })?;

        // Update the UI with new data
        while let Ok(piece) = rx.try_recv() {
            downloaded_pieces.push(piece);
        }

        // End the app when download ends
        if let Err(mpsc::error::TryRecvError::Disconnected) = rx.try_recv() {
            download_task.await??;
            return Ok(());
        }
    }
}
