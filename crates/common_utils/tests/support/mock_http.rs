#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct CapturedRequest {
    pub method: String,
    pub target: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl CapturedRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

pub struct MockResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// One bounded localhost exchange. Captures bytes, including non-UTF-8 bodies.
pub fn serve(
    handler: impl FnOnce(&CapturedRequest) -> MockResponse + Send + 'static,
) -> (String, thread::JoinHandle<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!(
        "http://{}/connector?merchant=fixture",
        listener.local_addr().unwrap()
    );
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(10))
                }
                Err(error) => panic!("Mock accept failed: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut received = Vec::new();
        let (split, header_length) = loop {
            if let Some(split) = received.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                break (split, split + 4);
            }
            let mut buffer = [0; 4096];
            let length = stream.read(&mut buffer).unwrap();
            assert_ne!(length, 0, "Request ended before headers");
            received.extend_from_slice(&buffer[..length]);
        };
        let raw_headers = std::str::from_utf8(&received[..split]).unwrap();
        let mut lines = raw_headers.split("\r\n");
        let request_line = lines.next().unwrap();
        let mut request_parts = request_line.split_whitespace();
        let method = request_parts.next().unwrap().to_owned();
        let target = request_parts.next().unwrap().to_owned();
        let headers: Vec<_> = lines
            .map(|line| {
                let (name, value) = line.split_once(':').unwrap();
                (name.to_owned(), value.trim().to_owned())
            })
            .collect();
        let length = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .map(|(_, value)| value.parse::<usize>().unwrap())
            .unwrap_or(0);
        assert!(
            !headers
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case("transfer-encoding")),
            "Expected fixed-size byte bodies"
        );
        while received.len() < header_length + length {
            let mut buffer = [0; 4096];
            let count = stream.read(&mut buffer).unwrap();
            assert_ne!(count, 0, "Request ended before body");
            received.extend_from_slice(&buffer[..count]);
        }
        let captured = CapturedRequest {
            method,
            target,
            headers,
            body: received[header_length..header_length + length].to_vec(),
        };
        let response = handler(&captured);
        write!(
            stream,
            "HTTP/1.1 {} Mock\r\nContent-Length: {}\r\nConnection: close\r\n",
            response.status,
            response.body.len()
        )
        .unwrap();
        for (name, value) in response.headers {
            write!(stream, "{name}: {value}\r\n").unwrap();
        }
        stream.write_all(b"\r\n").unwrap();
        stream.write_all(&response.body).unwrap();
        captured
    });
    (url, handle)
}
