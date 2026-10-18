use axum::body::Body;
use axum::http::{header, StatusCode};
use axum::response::Response;
use bytes::Bytes;
use std::io::{self, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::watch;

const STREAM_CHUNK: usize = 256 * 1024;
const ABANDON_AFTER: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, PartialEq)]
pub enum ExtractState {
    Running(u64),
    Done,
    Failed(String),
}

pub struct Extraction {
    path: PathBuf,
    total: u64,
    state: watch::Sender<ExtractState>,
    readers: AtomicUsize,
    idle_since: Mutex<Option<Instant>>,
    cancelled: AtomicBool,
}

pub struct ReaderGuard(Arc<Extraction>);

impl Drop for ReaderGuard {
    fn drop(&mut self) {
        if self.0.readers.fetch_sub(1, Ordering::AcqRel) == 1 {
            *self
                .0
                .idle_since
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = Some(Instant::now());
        }
    }
}

impl Extraction {
    pub fn new(path: PathBuf, total: u64) -> Arc<Self> {
        Arc::new(Self {
            path,
            total,
            state: watch::Sender::new(ExtractState::Running(0)),
            readers: AtomicUsize::new(0),
            idle_since: Mutex::new(None),
            cancelled: AtomicBool::new(false),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_running(&self) -> bool {
        matches!(*self.state.borrow(), ExtractState::Running(_))
    }

    pub fn is_reusable(&self) -> bool {
        match &*self.state.borrow() {
            ExtractState::Running(_) => true,
            ExtractState::Done => std::fs::metadata(&self.path)
                .map(|metadata| metadata.len() == self.total)
                .unwrap_or(false),
            ExtractState::Failed(_) => false,
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn attach(self: &Arc<Self>) -> ReaderGuard {
        self.readers.fetch_add(1, Ordering::AcqRel);
        *self
            .idle_since
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = None;
        ReaderGuard(self.clone())
    }

    pub fn progress(&self, written: u64) -> Result<(), String> {
        let abandoned = self.readers.load(Ordering::Acquire) == 0
            && self
                .idle_since
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .is_some_and(|since| since.elapsed() > ABANDON_AFTER);
        if abandoned {
            self.cancel();
        }
        if self.is_cancelled() {
            return Err("Archive extraction was stopped".to_string());
        }
        self.state.send_replace(ExtractState::Running(written));
        Ok(())
    }

    pub fn finish(&self, result: Result<(), String>) {
        self.state.send_replace(match result {
            Ok(()) => ExtractState::Done,
            Err(message) => ExtractState::Failed(message),
        });
    }

    pub async fn wait_started(&self) -> Result<(), String> {
        let mut state = self.state.subscribe();
        loop {
            match state.borrow_and_update().clone() {
                ExtractState::Running(written) if written > 0 => return Ok(()),
                ExtractState::Running(_) => {}
                ExtractState::Done => return Ok(()),
                ExtractState::Failed(message) => return Err(message),
            }
            state
                .changed()
                .await
                .map_err(|_| "Archive extraction stopped".to_string())?;
        }
    }

    pub fn body(self: Arc<Self>, reader: ReaderGuard, start: u64, end: u64) -> Body {
        let stream = async_stream::try_stream! {
            let _reader = reader;
            let mut state = self.state.subscribe();
            let mut file: Option<tokio::fs::File> = None;
            let mut position = start;
            while position <= end {
                let available = loop {
                    let current = state.borrow_and_update().clone();
                    match current {
                        ExtractState::Running(written) if written > position => break written,
                        ExtractState::Running(_) => {}
                        ExtractState::Done => break self.total,
                        ExtractState::Failed(message) => Err(io::Error::other(message))?,
                    }
                    if state.changed().await.is_err() {
                        Err(io::Error::other("Archive extraction stopped"))?;
                    }
                };
                if file.is_none() {
                    let mut opened = tokio::fs::File::open(&self.path).await?;
                    opened.seek(SeekFrom::Start(position)).await?;
                    file = Some(opened);
                }
                let length = (available.min(end + 1) - position).min(STREAM_CHUNK as u64) as usize;
                let mut chunk = vec![0u8; length];
                if let Some(handle) = file.as_mut() {
                    handle.read_exact(&mut chunk).await?;
                }
                position += length as u64;
                yield Bytes::from(chunk);
            }
        };
        Body::from_stream(io_stream(stream))
    }
}

// Pins the error type that `?` converts into inside `try_stream!`.
fn io_stream<S: futures_util::Stream<Item = io::Result<Bytes>>>(stream: S) -> S {
    stream
}

pub fn entry_response(
    partial: bool,
    start: u64,
    end: u64,
    total: u64,
    mime: &str,
    body: Body,
) -> Result<Response, (StatusCode, String)> {
    let length = if total == 0 { 0 } else { end - start + 1 };
    let mut builder = Response::builder()
        .status(if partial {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        })
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, length);
    if partial {
        builder = builder.header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{total}"),
        );
    }
    builder
        .body(body)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub fn stored_entry_range(
    data_start: u64,
    entry_size: u64,
    start: u64,
    end: u64,
) -> Option<(u64, u64)> {
    if start > end || end >= entry_size {
        return None;
    }
    Some((data_start.checked_add(start)?, data_start.checked_add(end)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_entry_ranges_shift_by_data_start() {
        assert_eq!(stored_entry_range(100, 50, 0, 49), Some((100, 149)));
        assert_eq!(stored_entry_range(100, 50, 10, 10), Some((110, 110)));
        assert_eq!(stored_entry_range(0, 1, 0, 0), Some((0, 0)));
        assert_eq!(stored_entry_range(100, 50, 0, 50), None);
        assert_eq!(stored_entry_range(100, 50, 20, 10), None);
        assert_eq!(stored_entry_range(u64::MAX - 5, 50, 0, 10), None);
    }

    #[tokio::test]
    async fn body_streams_bytes_as_extraction_progresses() {
        use futures_util::StreamExt;

        let dir = std::env::temp_dir().join(format!("pawstash-extract-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("0.bin");
        std::fs::write(&path, b"hello").unwrap();
        let extraction = Extraction::new(path.clone(), 10);
        extraction.progress(5).unwrap();

        let guard = extraction.attach();
        let mut body = extraction.clone().body(guard, 2, 8).into_data_stream();
        assert_eq!(
            body.next().await.unwrap().unwrap(),
            Bytes::from_static(b"llo")
        );

        let writer = extraction.clone();
        let finisher = tokio::spawn(async move {
            std::fs::write(&path, b"hello worl").unwrap();
            writer.progress(10).unwrap();
            writer.finish(Ok(()));
        });
        assert_eq!(
            body.next().await.unwrap().unwrap(),
            Bytes::from_static(b" wor")
        );
        assert!(body.next().await.is_none());
        finisher.await.unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn abandoned_extraction_stops() {
        let extraction = Extraction::new(PathBuf::from("unused"), 10);
        assert!(extraction.progress(1).is_ok());
        let guard = extraction.attach();
        drop(guard);
        *extraction.idle_since.lock().unwrap() =
            Some(Instant::now() - ABANDON_AFTER - Duration::from_secs(1));
        assert!(extraction.progress(2).is_err());
        let extraction = Extraction::new(PathBuf::from("unused"), 10);
        extraction.cancel();
        assert!(extraction.progress(1).is_err());
    }
}
