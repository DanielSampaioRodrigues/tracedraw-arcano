//! How the studio's adapters reach the services: JSON or multipart
//! requests, picture downloads and the pause between looks at a task
//! that runs in the background. The desktop uses [`Net`]; tests use
//! [`Fake`], which answers from a function and never sleeps.

use crate::net::{HttpRequest, HttpResult};

/// Requests, downloads and waiting.
pub trait Transport: Send + Sync {
    /// One request; the answer's body as text.
    fn call(&self, req: &HttpRequest) -> HttpResult;
    /// A file at `url` as bytes (pictures the service links to).
    fn download(&self, url: &str, headers: &[(String, String)]) -> Result<Vec<u8>, String>;
    /// Wait `secs` seconds before looking at a task again.
    fn wait(&self, secs: f64);
}

/// The largest answer read: pictures in base64 up to 4K.
pub const MAX_REPLY: u64 = 96 * 1024 * 1024;

/// The network (desktop).
pub struct Net;

impl Transport for Net {
    fn call(&self, req: &HttpRequest) -> HttpResult {
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::net::send(req, MAX_REPLY)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = req;
            Err(crate::i18n::tr("studio.desktop_only"))
        }
    }

    fn download(&self, url: &str, headers: &[(String, String)]) -> Result<Vec<u8>, String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use ureq::tls::{RootCerts, TlsConfig};
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .timeout_global(Some(std::time::Duration::from_secs(300)))
                .http_status_as_error(false)
                .tls_config(
                    TlsConfig::builder()
                        .root_certs(RootCerts::PlatformVerifier)
                        .build(),
                )
                .build()
                .into();
            let mut r = agent.get(url);
            for (name, value) in headers {
                r = r.header(name, value);
            }
            let mut resp = r.call().map_err(|e| e.to_string())?;
            let status = resp.status().as_u16();
            if status != 200 {
                return Err(crate::i18n::trf(
                    "studio.download_failed",
                    &[("status", &status.to_string())],
                ));
            }
            resp.body_mut()
                .with_config()
                .limit(MAX_REPLY)
                .read_to_vec()
                .map_err(|e| e.to_string())
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (url, headers);
            Err(crate::i18n::tr("studio.desktop_only"))
        }
    }

    fn wait(&self, secs: f64) {
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::sleep(std::time::Duration::from_secs_f64(secs.clamp(0.0, 30.0)));
        #[cfg(target_arch = "wasm32")]
        let _ = secs;
    }
}

/// Tests: answers every request with `answer`, downloads with
/// `download`, and records what was asked.
#[cfg(test)]
pub struct Fake {
    pub answer: fn(&HttpRequest) -> HttpResult,
    pub download: fn(&str) -> Result<Vec<u8>, String>,
    pub sent: std::sync::Mutex<Vec<HttpRequest>>,
}

#[cfg(test)]
impl Fake {
    pub fn new(
        answer: fn(&HttpRequest) -> HttpResult,
        download: fn(&str) -> Result<Vec<u8>, String>,
    ) -> Self {
        Fake {
            answer,
            download,
            sent: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// The requests made so far.
    pub fn sent(&self) -> Vec<HttpRequest> {
        self.sent.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

#[cfg(test)]
impl Transport for Fake {
    fn call(&self, req: &HttpRequest) -> HttpResult {
        if let Ok(mut s) = self.sent.lock() {
            s.push(req.clone());
        }
        (self.answer)(req)
    }

    fn download(&self, url: &str, _headers: &[(String, String)]) -> Result<Vec<u8>, String> {
        (self.download)(url)
    }

    fn wait(&self, _secs: f64) {}
}

/// An answer with `status` and a JSON `body` (tests).
#[cfg(test)]
pub fn reply(status: u16, body: serde_json::Value) -> HttpResult {
    Ok(crate::net::HttpResponse {
        status,
        body: body.to_string(),
    })
}

/// A multipart/form-data body: text fields, then files
/// (`(field, file name, content type, bytes)`).
pub fn multipart(
    boundary: &str,
    fields: &[(&str, String)],
    files: &[(&str, &str, &str, &[u8])],
) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    for (name, file, ctype, bytes) in files {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"{file}\"\r\nContent-Type: {ctype}\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

/// The boundary the studio's multipart bodies use.
pub const BOUNDARY: &str = "tracedraw-studio-boundary-5d1e8a";

/// A JSON POST.
pub fn post_json(url: String, headers: Vec<(String, String)>, body: &serde_json::Value) -> HttpRequest {
    let mut h = headers;
    h.push(("content-type".into(), "application/json".into()));
    HttpRequest {
        method: "POST",
        url,
        headers: h,
        body: body.to_string(),
        bytes: Vec::new(),
    }
}

/// A multipart POST.
pub fn post_multipart(url: String, headers: Vec<(String, String)>, body: Vec<u8>) -> HttpRequest {
    let mut h = headers;
    h.push((
        "content-type".into(),
        format!("multipart/form-data; boundary={BOUNDARY}"),
    ));
    HttpRequest {
        method: "POST",
        url,
        headers: h,
        body: String::new(),
        bytes: body,
    }
}

/// A GET.
pub fn get(url: String, headers: Vec<(String, String)>) -> HttpRequest {
    HttpRequest {
        method: "GET",
        url,
        headers,
        body: String::new(),
        bytes: Vec::new(),
    }
}

/// The answer as JSON when it is a success, else the service's message
/// with the status.
pub fn json_ok(r: HttpResult) -> Result<serde_json::Value, String> {
    let r = r?;
    let v: Option<serde_json::Value> = serde_json::from_str(&r.body).ok();
    if !(200..300).contains(&r.status) {
        return Err(service_error(r.status, v.as_ref(), &r.body));
    }
    v.ok_or_else(|| crate::i18n::tr("studio.bad_reply"))
}

/// A service's error as one line: its own message when the answer has
/// one (`error.message`, `message`, `detail`, `errors[0]`...), and the
/// HTTP status.
pub fn service_error(status: u16, v: Option<&serde_json::Value>, body: &str) -> String {
    let msg = v
        .and_then(|v| {
            [
                "/error/message",
                "/error",
                "/message",
                "/detail",
                "/details",
                "/errors/0/message",
                "/errors/0",
                "/name",
            ]
            .iter()
            .find_map(|p| match v.pointer(p) {
                Some(serde_json::Value::String(s)) if !s.is_empty() => Some(s.clone()),
                Some(serde_json::Value::Array(a)) if !a.is_empty() => {
                    Some(a.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("; "))
                }
                _ => None,
            })
        })
        .unwrap_or_else(|| body.chars().take(200).collect());
    crate::i18n::trf(
        "studio.service_error",
        &[("status", &status.to_string()), ("message", msg.trim())],
    )
}

/// Bytes from base64 (with or without a `data:` prefix).
pub fn unbase64(s: &str) -> Option<Vec<u8>> {
    use base64::Engine as _;
    let s = s.trim();
    let s = s.split_once(";base64,").map(|(_, b)| b).unwrap_or(s);
    base64::engine::general_purpose::STANDARD.decode(s).ok()
}

/// Base64 of `bytes`.
pub fn base64(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multipart_bodies_carry_fields_and_files() {
        let b = multipart(BOUNDARY, &[("prompt", "a cat".into())], &[("image", "i.png", "image/png", b"PNG")]);
        let s = String::from_utf8_lossy(&b);
        assert!(s.contains("name=\"prompt\"\r\n\r\na cat\r\n"));
        assert!(s.contains("name=\"image\"; filename=\"i.png\"\r\nContent-Type: image/png\r\n\r\nPNG\r\n"));
        assert!(s.ends_with(&format!("--{BOUNDARY}--\r\n")));
    }

    #[test]
    fn errors_read_the_services_own_message() {
        let v = serde_json::json!({"error": {"message": "bad key"}});
        assert!(service_error(401, Some(&v), "").contains("bad key"));
        let v = serde_json::json!({"errors": ["too big"], "name": "bad_request"});
        assert!(service_error(400, Some(&v), "").contains("too big"));
        assert!(service_error(500, None, "oops").contains("oops"));
        assert_eq!(unbase64("data:image/png;base64,UE5H"), Some(b"PNG".to_vec()));
    }
}
