use std::io::{Read, Write};
use std::net::TcpListener;

/// Start a minimal REST API server for DCP operations.
///
/// Endpoints:
/// - POST /create — Create a DCP from JSON config
/// - POST /verify — Verify a DCP
/// - GET /jobs — List all jobs
/// - GET /health — Health check
pub fn start_rest_api(bind_addr: &str) -> i32 {
    let listener = match TcpListener::bind(bind_addr) {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to bind to {bind_addr}: {e}");
            return -1;
        }
    };

    let queue = crate::job_queue::JobQueue::new();
    crate::job_queue::start_job_queue(&queue);

    tracing::info!("REST API listening on {bind_addr}");

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to accept connection: {e}");
                continue;
            }
        };

        let queue = queue.clone();

        std::thread::spawn(move || {
            let mut buf = vec![0u8; 8192];
            let n = match stream.read(&mut buf) {
                Ok(n) => n,
                Err(_) => return,
            };

            let request = String::from_utf8_lossy(&buf[..n]);
            let first_line = request.lines().next().unwrap_or("");
            let parts: Vec<&str> = first_line.split_whitespace().collect();

            if parts.len() < 2 {
                let _ = send_response(&mut stream, 400, "Bad Request");
                return;
            }

            let method = parts[0];
            let path = parts[1];

            // Extract body (after \r\n\r\n)
            let body = request
                .find("\r\n\r\n")
                .map(|pos| &request[pos + 4..])
                .unwrap_or("")
                .trim()
                .to_string();

            match (method, path) {
                ("GET", "/health") => {
                    let response = serde_json::json!({"status": "ok"}).to_string();
                    let _ = send_json(&mut stream, 200, &response);
                }
                ("GET", "/jobs") => {
                    let jobs = queue.list();
                    let response = serde_json::to_string(&jobs).unwrap_or_else(|_| "[]".into());
                    let _ = send_json(&mut stream, 200, &response);
                }
                ("POST", "/create") => match serde_json::from_str::<crate::dcp::DcpConfig>(&body) {
                    Ok(_config) => {
                        let job_id = queue.submit(crate::job_queue::JobType::CreateDcp, &body);
                        let response = serde_json::json!({"job_id": job_id}).to_string();
                        let _ = send_json(&mut stream, 202, &response);
                    }
                    Err(e) => {
                        let response = serde_json::json!({"error": format!("Invalid config: {e}")})
                            .to_string();
                        let _ = send_json(&mut stream, 400, &response);
                    }
                },
                ("POST", "/verify") => {
                    let path = body.trim().trim_matches('"');
                    if path.is_empty() {
                        let response =
                            serde_json::json!({"error": "Missing DCP path in body"}).to_string();
                        let _ = send_json(&mut stream, 400, &response);
                    } else {
                        let job_id = queue.submit(crate::job_queue::JobType::VerifyDcp, path);
                        let response = serde_json::json!({"job_id": job_id}).to_string();
                        let _ = send_json(&mut stream, 202, &response);
                    }
                }
                _ => {
                    let _ = send_response(&mut stream, 404, "Not Found");
                }
            }
        });
    }

    0
}

fn send_response(
    stream: &mut std::net::TcpStream,
    status: u16,
    message: &str,
) -> std::io::Result<()> {
    let status_text = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };

    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{message}",
        message.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

fn send_json(stream: &mut std::net::TcpStream, status: u16, json: &str) -> std::io::Result<()> {
    let status_text = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };

    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}",
        json.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}
