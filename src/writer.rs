use crate::piece::PieceData;
use anyhow::{Ok, Result};
use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt, BufWriter};
use tokio::sync::mpsc::{Receiver, Sender};

/// Structure that represents writer, which stores the downloaded pieces during download to final file.
pub struct PieceFileWriter {
    file: BufWriter<File>,
    total_pieces: usize,
    piece_length: usize,
    total_file_size: u64,
    piece_channel: Receiver<PieceData>,
    downloaded_sender: Sender<usize>,
}

impl PieceFileWriter {
    /// Creates new `PieceFileWriter`,
    /// `piece_channel` is used to receive data of already downloaded pieces,
    /// `downloaded_sender` is used to notifie TUI about pieces that were already writen to file.
    pub async fn new(
        file_path: PathBuf,
        total_pieces: usize,
        piece_length: usize,
        total_file_size: u64,
        piece_channel: Receiver<PieceData>,
        downloaded_sender: Sender<usize>,
    ) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        // Open file with write and create options
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_path)
            .await?;

        // Pre-allocate file space
        file.set_len(total_file_size).await?;

        let file_writer = BufWriter::new(file);

        Ok(PieceFileWriter {
            file: file_writer,
            total_pieces,
            piece_length,
            total_file_size,
            piece_channel,
            downloaded_sender,
        })
    }

    /// Write all the received pieces to the correct position in final file.
    pub async fn write_file(&mut self) -> Result<()> {
        let mut saved = 0;
        while let Some(piece_data) = self.piece_channel.recv().await {
            let piece_idx = piece_data.piece_idx;
            self.write_piece(piece_data).await?;
            self.downloaded_sender.send(piece_idx).await?;
            saved += 1;
            if saved == self.total_pieces {
                break;
            }
        }
        self.piece_channel.close();

        Ok(())
    }

    /// Writes the given piece to the final file, on correct position.
    async fn write_piece(&mut self, piece_data: PieceData) -> Result<()> {
        let piece_index = piece_data.piece_idx;
        let piece_data = &piece_data.data;
        // Validate piece index and data
        if piece_index >= self.total_pieces {
            anyhow::bail!("Invalid piece index");
        }

        // Calculate file offset for this piece
        let offset = (piece_index * self.piece_length) as u64;

        // Determine actual bytes to write (handle last piece potentially being smaller)
        let bytes_to_write = if piece_index == self.total_pieces - 1 {
            let remaining = self.total_file_size as usize - (piece_index * self.piece_length);
            &piece_data[..remaining.min(piece_data.len())]
        } else {
            &piece_data[..]
        };

        // Acquire file lock and write piece
        self.file.seek(std::io::SeekFrom::Start(offset)).await?;
        self.file.write_all(bytes_to_write).await?;
        self.file.flush().await?;

        Ok(())
    }
}
