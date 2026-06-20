#[cfg(test)]
mod security_tests {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    #[test]
    fn test_redis_rejects_unauthenticated_requests() {
        // Invariant: Protected endpoints must reject unauthenticated requests
        // Redis should require authentication before executing commands
        
        let redis_host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let redis_port = std::env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
        let addr = format!("{}:{}", redis_host, redis_port);

        // Test payloads: unauthenticated commands that should be rejected
        let payloads = vec![
            "*1\r\n$4\r\nPING\r\n",           // Simple PING without auth
            "*3\r\n$3\r\nSET\r\n$4\r\ntest\r\n$5\r\nvalue\r\n", // SET command without auth
            "*2\r\n$4\r\nAUTH\r\n$0\r\n\r\n", // AUTH with empty password (malformed)
        ];

        for payload in &payloads {
            if let Ok(mut stream) = TcpStream::connect(&addr) {
                stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
                stream.set_write_timeout(Some(Duration::from_secs(2))).ok();

                if stream.write_all(payload.as_bytes()).is_ok() {
                    let mut response = vec![0u8; 256];
                    if let Ok(n) = stream.read(&mut response) {
                        let response_str = String::from_utf8_lossy(&response[..n]);
                        // Security invariant: unauthenticated requests should be rejected
                        // Response should be -NOAUTH or -ERR, not +OK or +PONG
                        assert!(
                            response_str.starts_with("-NOAUTH") || 
                            response_str.starts_with("-ERR") ||
                            response_str.contains("authentication"),
                            "SECURITY VIOLATION: Redis accepted unauthenticated command. \
                             Response: {}. Payload: {:?}",
                            response_str.trim(),
                            payload
                        );
                    }
                }
            }
        }
    }
}