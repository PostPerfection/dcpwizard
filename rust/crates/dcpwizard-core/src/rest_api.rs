//! REST API server for DCP operations.
//!
//! Kept separate from [`postkit::rest_api::RestServer`]: postkit's route handler
//! signature is `Fn(method, path)` and never passes the request body, so it
//! cannot express the body-consuming `POST /create` and `POST /verify` routes
//! this server needs. The GET routes (/health, /jobs, /metrics) also depend on
//! dcpwizard's own job queue.

use std::io::{Read, Write};
use std::net::TcpListener;

/// Start a minimal REST API server for DCP operations.
///
/// Endpoints:
/// - POST /create — Create a DCP from JSON config
/// - POST /verify — Verify a DCP
/// - GET /jobs — List all jobs
/// - GET /daemon-status — Check if job daemon is running
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
                ("GET", "/daemon-status") => {
                    let running = crate::job_queue::is_daemon_running();
                    let response = serde_json::json!({"daemon_running": running}).to_string();
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
                ("GET", "/metrics") => {
                    let metrics = build_prometheus_metrics(&queue);
                    let _ = send_plain(&mut stream, 200, &metrics);
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

fn send_plain(stream: &mut std::net::TcpStream, status: u16, body: &str) -> std::io::Result<()> {
    let status_text = match status {
        200 => "OK",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: text/plain; version=0.0.4; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

/// Build Prometheus-compatible metrics text from the job queue state.
fn build_prometheus_metrics(queue: &crate::job_queue::JobQueue) -> String {
    use crate::job_queue::JobState;
    use std::fmt::Write;

    let jobs = queue.list();
    let total = jobs.len();
    let pending = jobs.iter().filter(|j| j.state == JobState::Pending).count();
    let running = jobs.iter().filter(|j| j.state == JobState::Running).count();
    let completed = jobs
        .iter()
        .filter(|j| j.state == JobState::Completed)
        .count();
    let failed = jobs.iter().filter(|j| j.state == JobState::Failed).count();

    let mut out = String::new();

    let _ = writeln!(
        out,
        "# HELP dcpwizard_jobs_total Total number of jobs submitted."
    );
    let _ = writeln!(out, "# TYPE dcpwizard_jobs_total gauge");
    let _ = writeln!(out, "dcpwizard_jobs_total {total}");
    let _ = writeln!(out);
    let _ = writeln!(out, "# HELP dcpwizard_jobs_pending Number of pending jobs.");
    let _ = writeln!(out, "# TYPE dcpwizard_jobs_pending gauge");
    let _ = writeln!(out, "dcpwizard_jobs_pending {pending}");
    let _ = writeln!(out);
    let _ = writeln!(out, "# HELP dcpwizard_jobs_running Number of running jobs.");
    let _ = writeln!(out, "# TYPE dcpwizard_jobs_running gauge");
    let _ = writeln!(out, "dcpwizard_jobs_running {running}");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "# HELP dcpwizard_jobs_completed Number of completed jobs."
    );
    let _ = writeln!(out, "# TYPE dcpwizard_jobs_completed gauge");
    let _ = writeln!(out, "dcpwizard_jobs_completed {completed}");
    let _ = writeln!(out);
    let _ = writeln!(out, "# HELP dcpwizard_jobs_failed Number of failed jobs.");
    let _ = writeln!(out, "# TYPE dcpwizard_jobs_failed gauge");
    let _ = writeln!(out, "dcpwizard_jobs_failed {failed}");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "# HELP dcpwizard_daemon_running Whether the job daemon is running."
    );
    let _ = writeln!(out, "# TYPE dcpwizard_daemon_running gauge");
    let daemon_up = if crate::job_queue::is_daemon_running() {
        1
    } else {
        0
    };
    let _ = writeln!(out, "dcpwizard_daemon_running {daemon_up}");

    out
}
