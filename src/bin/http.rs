// use bencode::Bencode;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use http::{Request, Uri};
use http_body_util::Full;
use hyper::{body::Bytes, client::Client};
use hyper_util::rt::TokioExecutor;
use lava_torrent::torrent_file::TorrentFile;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::io::{self, stdout};
use std::net::SocketAddr;
use std::path::Path;
use urlencoding::encode;

#[derive(Debug)]
struct Peer {
    ip: String,
    port: u16,
}

struct App {
    torrent_files: Vec<String>,
    selected_file: Option<String>,
    peers: Vec<Peer>,
    error: Option<String>,
}

impl App {
    fn new() -> Self {
        let torrent_files = std::fs::read_dir(".")
            .unwrap()
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.extension()?.to_str()? == "torrent" {
                    Some(path.to_str()?.to_owned())
                } else {
                    None
                }
            })
            .collect();

        Self {
            torrent_files,
            selected_file: None,
            peers: Vec::new(),
            error: None,
        }
    }

    fn discover_peers(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(torrent_path) = &self.selected_file {
            let torrent = TorrentFile::read_from_file(torrent_path)?;

            let info_hash = encode(&torrent.info_hash.to_string());
            let peer_id = generate_peer_id();
            let port = 6881;
            let uploaded = 0;
            let downloaded = 0;
            let left = torrent.info.piece_length * torrent.info.pieces.len() as u64;

            let url_str = format!(
                "{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact=1",
                torrent.announce, info_hash, peer_id, port, uploaded, downloaded, left
            );
            let uri: Uri = url_str.parse()?;

            let client = Client::builder(TokioExecutor::new()).build_http();

            let request = Request::builder()
                .method("GET")
                .uri(uri)
                .body(Full::<Bytes>::new(Bytes::new()))?;

            let response = client.request(request).await?;
            let body_bytes = hyper::body::to_bytes(response.into_body()).await?;

            let bencode = Bencode::decode(&body_bytes)?;
            if let Some(peers_data) = bencode.dict_get("peers") {
                if let Bencode::ByteString(peers_bytes) = peers_data {
                    self.parse_compact_peers(peers_bytes);
                }
            }
        }
        Ok(())
    }

    fn parse_compact_peers(&mut self, peers_bytes: &[u8]) {
        for chunk in peers_bytes.chunks(6) {
            if chunk.len() == 6 {
                let ip = format!("{}.{}.{}.{}", chunk[0], chunk[1], chunk[2], chunk[3]);
                let port = u16::from_be_bytes([chunk[4], chunk[5]]);
                self.peers.push(Peer { ip, port });
            }
        }
    }
}

fn generate_peer_id() -> String {
    format!("-RU0001-{:020}", rand::random::<u64>())
}

fn run_app(app: &mut App) -> io::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Down => {
                    if !app.torrent_files.is_empty() {
                        let current_index = app
                            .torrent_files
                            .iter()
                            .position(|f| Some(f) == app.selected_file.as_ref());
                        app.selected_file = current_index
                            .map(|i| app.torrent_files[(i + 1) % app.torrent_files.len()].clone())
                            .or_else(|| app.torrent_files.first().cloned());
                    }
                }
                KeyCode::Up => {
                    if !app.torrent_files.is_empty() {
                        let current_index = app
                            .torrent_files
                            .iter()
                            .position(|f| Some(f) == app.selected_file.as_ref());
                        app.selected_file = current_index
                            .map(|i| {
                                app.torrent_files
                                    [(i + app.torrent_files.len() - 1) % app.torrent_files.len()]
                                .clone()
                            })
                            .or_else(|| app.torrent_files.first().cloned());
                    }
                }
                KeyCode::Enter => {
                    app.peers.clear();
                    match app.discover_peers() {
                        Ok(_) => {}
                        Err(e) => app.error = Some(e.to_string()),
                    }
                }
                KeyCode::Esc => break,
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(f.size());

    // Torrent files list
    let torrent_items: Vec<ListItem> = app
        .torrent_files
        .iter()
        .map(|file| {
            let content = Path::new(file).file_name().unwrap().to_str().unwrap();
            let is_selected = Some(file) == app.selected_file.as_ref();
            ListItem::new(content).style(if is_selected {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
        })
        .collect();

    let torrent_list = List::new(torrent_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Torrent Files"),
    );
    f.render_widget(torrent_list, chunks[0]);

    // Peers list
    let peer_items: Vec<ListItem> = app
        .peers
        .iter()
        .map(|peer| ListItem::new(format!("{}:{}", peer.ip, peer.port)))
        .collect();

    let peer_list = List::new(peer_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Discovered Peers"),
    );
    f.render_widget(peer_list, chunks[1]);

    // Error display
    if let Some(error) = &app.error {
        let error_block = Paragraph::new(error.clone())
            .block(Block::default().borders(Borders::ALL).title("Error"));
        f.render_widget(error_block, chunks[1]);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    run_app(&mut app)?;
    Ok(())
}
