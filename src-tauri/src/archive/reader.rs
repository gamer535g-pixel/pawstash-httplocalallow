use crate::server::axum_server::UpstreamGateway;
use axum::http::{header, Method, StatusCode};
use bytes::{Bytes, BytesMut};
use std::collections::{HashMap, VecDeque};
use std::io::{self, Read, Seek, SeekFrom};
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

const BLOCK_SIZE: u64 = 1 << 20;
const TAIL_SIZE: u64 = 256 << 10;
const CACHE_BUDGET: usize = 6 << 20;
const MAX_FORWARD_SKIP: u64 = 256 << 10;
const FETCH_ATTEMPTS: u32 = 3;
// Pawchive asks clients to stay at or below one request per second.
pub const HOST_REQUEST_INTERVAL: Duration = Duration::from_secs(1);

pub struct HostPacer {
    interval: Duration,
    hosts: Mutex<HashMap<String, Arc<tokio::sync::Mutex<Option<Instant>>>>>,
}

impl HostPacer {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            hosts: Mutex::new(HashMap::new()),
        }
    }

    pub async fn slot(&self, host: &str) -> tokio::sync::OwnedMutexGuard<Option<Instant>> {
        let cell = self
            .hosts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(host.to_string())
            .or_default()
            .clone();
        let mut last = cell.lock_owned().await;
        if let Some(previous) = *last {
            tokio::time::sleep_until(tokio::time::Instant::from_std(previous + self.interval))
                .await;
        }
        *last = Some(Instant::now());
        last
    }
}

pub fn parse_content_range(value: &str) -> Option<(u64, u64, u64)> {
    let (range, total) = value.trim().strip_prefix("bytes ")?.split_once('/')?;
    let (start, end) = range.split_once('-')?;
    let (start, end, total) = (
        start.trim().parse().ok()?,
        end.trim().parse().ok()?,
        total.trim().parse().ok()?,
    );
    (start <= end && end < total).then_some((start, end, total))
}

fn fatal(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn retryable(error: &io::Error) -> bool {
    error.kind() != io::ErrorKind::InvalidData
}

fn gateway_error((status, message): (StatusCode, String)) -> io::Error {
    if status.is_server_error() {
        io::Error::other(message)
    } else {
        fatal(message)
    }
}

async fn read_body(
    mut response: reqwest::Response,
    expected: u64,
    counter: &AtomicU64,
) -> io::Result<Bytes> {
    let mut body = BytesMut::with_capacity(expected as usize);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| io::Error::other(format!("Archive download failed: {e}")))?
    {
        counter.fetch_add(chunk.len() as u64, Ordering::Relaxed);
        if (body.len() + chunk.len()) as u64 > expected {
            return Err(fatal("The archive host sent more data than requested"));
        }
        body.extend_from_slice(&chunk);
    }
    if body.len() as u64 != expected {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "The archive host closed the connection early",
        ));
    }
    Ok(body.freeze())
}

#[derive(Default)]
struct ExtentCache {
    extents: VecDeque<(u64, Bytes)>,
    bytes: usize,
}

impl ExtentCache {
    fn read(&mut self, position: u64, buf: &mut [u8]) -> Option<usize> {
        let index = self
            .extents
            .iter()
            .position(|(start, data)| *start <= position && position < start + data.len() as u64)?;
        let (start, data) = self.extents.remove(index)?;
        let offset = (position - start) as usize;
        let count = buf.len().min(data.len() - offset);
        buf[..count].copy_from_slice(&data[offset..offset + count]);
        self.extents.push_back((start, data));
        Some(count)
    }

    fn insert(&mut self, start: u64, data: Bytes) {
        self.bytes += data.len();
        self.extents.push_back((start, data));
        while self.bytes > CACHE_BUDGET && self.extents.len() > 1 {
            if let Some((_, evicted)) = self.extents.pop_front() {
                self.bytes -= evicted.len();
            }
        }
    }
}

pub enum Transport {
    Upstream(Arc<UpstreamGateway>),
    Loopback(reqwest::Client),
}

impl Transport {
    async fn get(&self, url: &str, range: &str) -> Result<reqwest::Response, (StatusCode, String)> {
        match self {
            Self::Upstream(gateway) => gateway
                .send(Method::GET, url, Some(range))
                .await
                .map(|(response, _)| response),
            Self::Loopback(client) => client
                .get(url)
                .header(header::RANGE, range)
                .send()
                .await
                .map_err(|e| {
                    (
                        StatusCode::BAD_GATEWAY,
                        format!("Local media server request failed: {e}"),
                    )
                }),
        }
    }
}

pub struct RemoteSource {
    url: String,
    host: String,
    len: u64,
    transport: Transport,
    pacer: Arc<HostPacer>,
    runtime: tokio::runtime::Handle,
    cache: Mutex<ExtentCache>,
    fetched: AtomicU64,
}

impl RemoteSource {
    pub async fn open(
        transport: Transport,
        pacer: Arc<HostPacer>,
        url: reqwest::Url,
    ) -> Result<Arc<Self>, String> {
        let host = url
            .host_str()
            .ok_or_else(|| "The archive URL has no host".to_string())?
            .to_ascii_lowercase();
        if matches!(transport, Transport::Loopback(_))
            && (!super::is_own_stream_route(&url) || url.path().starts_with(super::ARCHIVE_ROUTE))
        {
            return Err(
                "Only the app's own media streams can be read without the cloud proxy".to_string(),
            );
        }
        let fetched = AtomicU64::new(0);
        let slot = pacer.slot(&host).await;
        let response = transport
            .get(url.as_str(), &format!("bytes=-{TAIL_SIZE}"))
            .await
            .map_err(|(_, message)| message)?;
        let status = response.status();
        let (tail_start, len) = match status {
            StatusCode::PARTIAL_CONTENT => {
                let (start, _, total) = response
                    .headers()
                    .get(header::CONTENT_RANGE)
                    .and_then(|value| value.to_str().ok())
                    .and_then(parse_content_range)
                    .ok_or_else(|| {
                        "The archive host sent an invalid Content-Range header".to_string()
                    })?;
                (start, total)
            }
            StatusCode::OK => match response.content_length() {
                Some(total) if total <= TAIL_SIZE => (0, total),
                _ => {
                    return Err(
                        "The archive host does not support byte ranges; download the archive to open it"
                            .to_string(),
                    )
                }
            },
            other => return Err(format!("The archive host answered HTTP {other}")),
        };
        let tail = read_body(response, len - tail_start, &fetched)
            .await
            .map_err(|e| e.to_string())?;
        drop(slot);

        let mut cache = ExtentCache::default();
        cache.insert(tail_start, tail);
        Ok(Arc::new(Self {
            url: url.to_string(),
            host,
            len,
            transport,
            pacer,
            runtime: tokio::runtime::Handle::current(),
            cache: Mutex::new(cache),
            fetched,
        }))
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    #[cfg(test)]
    pub fn fetched_bytes(&self) -> u64 {
        self.fetched.load(Ordering::Relaxed)
    }

    pub async fn request_range(&self, start: u64, end: u64) -> io::Result<reqwest::Response> {
        let response = self
            .transport
            .get(&self.url, &format!("bytes={start}-{}", end - 1))
            .await
            .map_err(gateway_error)?;
        let status = response.status();
        if status == StatusCode::OK {
            return Err(fatal(
                "The archive host ignored the byte range request; download the archive instead",
            ));
        }
        if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
            return Err(io::Error::other(format!(
                "The archive host answered HTTP {status}"
            )));
        }
        if status != StatusCode::PARTIAL_CONTENT {
            return Err(fatal(format!("The archive host answered HTTP {status}")));
        }
        let range = response
            .headers()
            .get(header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .and_then(parse_content_range);
        if range != Some((start, end - 1, self.len)) {
            return Err(fatal(format!(
                "The archive host returned a different byte range ({range:?}) than requested ({start}-{})",
                end - 1
            )));
        }
        Ok(response)
    }

    async fn fetch_block(&self, start: u64, end: u64) -> io::Result<Bytes> {
        let _slot = self.pacer.slot(&self.host).await;
        let response = self.request_range(start, end).await?;
        read_body(response, end - start, &self.fetched).await
    }

    pub async fn open_stream(&self, start: u64, end: u64) -> io::Result<reqwest::Response> {
        let _slot = self.pacer.slot(&self.host).await;
        self.request_range(start, end).await
    }

    fn fetch_block_blocking(&self, start: u64, end: u64) -> io::Result<Bytes> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            match self.runtime.block_on(self.fetch_block(start, end)) {
                Ok(data) => return Ok(data),
                Err(error) if retryable(&error) && attempt < FETCH_ATTEMPTS => {
                    tracing::warn!(url = %self.url, start, end, attempt, %error, "Archive block fetch failed; retrying");
                }
                Err(error) => return Err(error),
            }
        }
    }
}

struct SequentialStream {
    next: u64,
    response: reqwest::Response,
    pending: Bytes,
}

impl SequentialStream {
    fn read_at(
        &mut self,
        position: u64,
        buf: &mut [u8],
        source: &RemoteSource,
    ) -> io::Result<usize> {
        loop {
            if self.pending.is_empty() {
                let chunk = source
                    .runtime
                    .block_on(async {
                        self.response
                            .chunk()
                            .await
                            .map_err(|e| io::Error::other(format!("Archive download failed: {e}")))
                    })?
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "The archive host closed the connection early",
                        )
                    })?;
                source
                    .fetched
                    .fetch_add(chunk.len() as u64, Ordering::Relaxed);
                self.pending = chunk;
            }
            if self.next < position {
                let skip = ((position - self.next) as usize).min(self.pending.len());
                self.pending = self.pending.slice(skip..);
                self.next += skip as u64;
                continue;
            }
            let count = buf.len().min(self.pending.len());
            buf[..count].copy_from_slice(&self.pending[..count]);
            self.pending = self.pending.slice(count..);
            self.next += count as u64;
            return Ok(count);
        }
    }
}

pub struct RemoteReader {
    source: Arc<RemoteSource>,
    position: u64,
    sequential: Option<Range<u64>>,
    stream: Option<SequentialStream>,
}

impl RemoteReader {
    pub fn new(source: Arc<RemoteSource>, sequential: Option<Range<u64>>) -> Self {
        Self {
            source,
            position: 0,
            sequential,
            stream: None,
        }
    }

    fn read_block(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let start = self.position - self.position % BLOCK_SIZE;
        let end = (start + BLOCK_SIZE).min(self.source.len);
        let data = self.source.fetch_block_blocking(start, end)?;
        let offset = (self.position - start) as usize;
        let count = buf.len().min(data.len() - offset);
        buf[..count].copy_from_slice(&data[offset..offset + count]);
        self.source
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(start, data);
        self.position += count as u64;
        Ok(count)
    }

    fn read_streamed(&mut self, buf: &mut [u8], end: u64) -> io::Result<usize> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            let position = self.position;
            if self.stream.as_ref().is_some_and(|stream| {
                position < stream.next || position - stream.next > MAX_FORWARD_SKIP
            }) {
                self.stream = None;
            }
            let stream = match self.stream.as_mut() {
                Some(stream) => stream,
                None => match self
                    .source
                    .runtime
                    .block_on(self.source.open_stream(position, end))
                {
                    Ok(response) => self.stream.insert(SequentialStream {
                        next: position,
                        response,
                        pending: Bytes::new(),
                    }),
                    Err(error) if retryable(&error) && attempt < FETCH_ATTEMPTS => {
                        tracing::warn!(url = %self.source.url, position, attempt, %error, "Archive stream failed to open; retrying");
                        continue;
                    }
                    Err(error) => return Err(error),
                },
            };
            match stream.read_at(position, buf, &self.source) {
                Ok(count) => {
                    self.position += count as u64;
                    return Ok(count);
                }
                Err(error) => {
                    self.stream = None;
                    if !(retryable(&error) && attempt < FETCH_ATTEMPTS) {
                        return Err(error);
                    }
                    tracing::warn!(url = %self.source.url, position, attempt, %error, "Archive stream broke; reconnecting");
                }
            }
        }
    }
}

impl Read for RemoteReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() || self.position >= self.source.len {
            return Ok(0);
        }
        let cached = self
            .source
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .read(self.position, buf);
        if let Some(count) = cached {
            self.position += count as u64;
            return Ok(count);
        }
        match self.sequential.clone() {
            Some(window) if window.contains(&self.position) => self.read_streamed(buf, window.end),
            _ => self.read_block(buf),
        }
    }
}

fn seek_position(current: u64, len: u64, target: SeekFrom) -> io::Result<u64> {
    let next = match target {
        SeekFrom::Start(offset) => i128::from(offset),
        SeekFrom::End(offset) => i128::from(len) + i128::from(offset),
        SeekFrom::Current(offset) => i128::from(current) + i128::from(offset),
    };
    u64::try_from(next).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Seek before the start of the archive",
        )
    })
}

impl Seek for RemoteReader {
    fn seek(&mut self, target: SeekFrom) -> io::Result<u64> {
        self.position = seek_position(self.position, self.source.len, target)?;
        Ok(self.position)
    }
}

pub struct LocalReader {
    file: Arc<std::fs::File>,
    len: u64,
    position: u64,
}

impl LocalReader {
    pub fn new(file: Arc<std::fs::File>, len: u64) -> Self {
        Self {
            file,
            len,
            position: 0,
        }
    }
}

#[cfg(unix)]
fn read_file_at(file: &std::fs::File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(file, buf, offset)
}

#[cfg(windows)]
fn read_file_at(file: &std::fs::File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::windows::fs::FileExt::seek_read(file, buf, offset)
}

impl Read for LocalReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.position >= self.len {
            return Ok(0);
        }
        let limit = buf.len().min((self.len - self.position) as usize);
        let count = read_file_at(&self.file, &mut buf[..limit], self.position)?;
        self.position += count as u64;
        Ok(count)
    }
}

impl Seek for LocalReader {
    fn seek(&mut self, target: SeekFrom) -> io::Result<u64> {
        self.position = seek_position(self.position, self.len, target)?;
        Ok(self.position)
    }
}

pub enum SourceReader {
    Remote(RemoteReader),
    Local(LocalReader),
}

impl Read for SourceReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Remote(reader) => reader.read(buf),
            Self::Local(reader) => reader.read(buf),
        }
    }
}

impl Seek for SourceReader {
    fn seek(&mut self, target: SeekFrom) -> io::Result<u64> {
        match self {
            Self::Remote(reader) => reader.seek(target),
            Self::Local(reader) => reader.seek(target),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_content_range() {
        assert_eq!(
            parse_content_range("bytes 239423803-239489338/239489339"),
            Some((239423803, 239489338, 239489339))
        );
        assert_eq!(parse_content_range("bytes 0-0/1"), Some((0, 0, 1)));
        assert_eq!(parse_content_range("bytes */1000"), None);
        assert_eq!(parse_content_range("bytes 10-5/1000"), None);
        assert_eq!(parse_content_range("bytes 0-1000/1000"), None);
    }

    #[test]
    fn extent_cache_serves_hits_and_evicts_oldest() {
        let mut cache = ExtentCache::default();
        cache.insert(100, Bytes::from_static(b"abcdef"));
        let mut buf = [0u8; 4];
        assert_eq!(cache.read(102, &mut buf), Some(4));
        assert_eq!(&buf, b"cdef");
        assert_eq!(cache.read(106, &mut buf), None);
        assert_eq!(cache.read(99, &mut buf), None);

        let block = Bytes::from(vec![7u8; CACHE_BUDGET]);
        cache.insert(1_000, block);
        assert_eq!(cache.read(100, &mut buf), None);
        assert_eq!(cache.read(1_000, &mut buf), Some(4));
    }

    #[test]
    fn seeks_relative_to_both_ends() {
        assert_eq!(seek_position(5, 100, SeekFrom::End(-22)).unwrap(), 78);
        assert_eq!(seek_position(5, 100, SeekFrom::Current(10)).unwrap(), 15);
        assert!(seek_position(5, 100, SeekFrom::Current(-6)).is_err());
    }
}
