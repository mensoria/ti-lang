//! Loopback-only fault proxy. Never logs headers, tokens or payloads.
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
pub struct Proxy {
    pub endpoint: String,
    pub mode: Arc<AtomicU8>,
    pub observed: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}
impl Drop for Proxy {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.mode.store(0, Ordering::SeqCst);
    }
}
impl Proxy {
    pub fn start(upstream: String) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = listener.local_addr().unwrap().to_string();
        let mode = Arc::new(AtomicU8::new(0));
        let observed = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let (m, o, s) = (mode.clone(), observed.clone(), stop.clone());
        thread::spawn(move || {
            while !s.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((client, _)) => {
                        let (u, m, o, s) = (upstream.clone(), m.clone(), o.clone(), s.clone());
                        thread::spawn(move || {
                            let _ = forward(client, &u, &m, &o, &s);
                        });
                    }
                    Err(_) => thread::sleep(Duration::from_millis(5)),
                }
            }
        });
        Self {
            endpoint,
            mode,
            observed,
            stop,
        }
    }
}
fn forward(
    mut client: TcpStream,
    upstream: &str,
    mode: &AtomicU8,
    observed: &AtomicBool,
    stop: &AtomicBool,
) -> std::io::Result<()> {
    client.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut request = vec![];
    let mut buf = [0; 8192];
    let header_end = loop {
        let n = client.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        request.extend_from_slice(&buf[..n]);
        if let Some(i) = request.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        if request.len() > 16384 {
            return Err(std::io::Error::other("header limit"));
        }
    };
    let header = String::from_utf8_lossy(&request[..header_end]).into_owned();
    let length = header
        .lines()
        .find_map(|l| {
            l.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .and_then(|v| v.trim().parse::<usize>().ok())
        })
        .unwrap_or(0);
    if length > 2 * 1024 * 1024 {
        return Err(std::io::Error::other("request limit"));
    }
    while request.len() < header_end + length {
        let n = client.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        request.extend_from_slice(&buf[..n]);
    }
    let transaction = request[header_end..]
        .windows(13)
        .any(|w| w == b"TI_TEST_ABORT");
    let rewritten = header
        .lines()
        .filter(|l| !l.to_ascii_lowercase().starts_with("connection:"))
        .collect::<Vec<_>>()
        .join("\r\n");
    let mut server = TcpStream::connect(upstream)?;
    server.set_read_timeout(Some(Duration::from_secs(5)))?;
    server.write_all(rewritten.trim_end().as_bytes())?;
    server.write_all(b"\r\nConnection: close\r\n\r\n")?;
    server.write_all(&request[header_end..header_end + length])?;
    let mut response = vec![];
    server.read_to_end(&mut response)?;
    let fault = if transaction {
        mode.swap(0, Ordering::SeqCst)
    } else {
        0
    };
    if fault != 0 {
        observed.store(true, Ordering::SeqCst);
        if fault == 1 {
            return Ok(());
        }
        mode.store(2, Ordering::SeqCst);
        while mode.load(Ordering::SeqCst) == 2 && !stop.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(5));
        }
    }
    client.write_all(&response)?;
    Ok(())
}
