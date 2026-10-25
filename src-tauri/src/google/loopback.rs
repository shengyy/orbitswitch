//! One-shot loopback HTTP server that receives the OAuth redirect.
//!
//! Google accepts any `127.0.0.1` port for installed-app clients, so the
//! server binds an ephemeral port. Requests whose `state` does not match are
//! answered and ignored, so a stray page cannot abort a sign-in.

use std::net::Ipv4Addr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::error::{Error, Result};

const CALLBACK_PATH: &str = "/oauth-callback";
const MAX_REQUEST: usize = 16 * 1024;

pub struct Loopback {
    listener: TcpListener,
    pub redirect_uri: String,
}

enum Outcome {
    Ignore(u16),
    Code(String),
    Denied(String),
}

impl Loopback {
    pub async fn bind() -> Result<Loopback> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let port = listener.local_addr()?.port();
        Ok(Loopback {
            listener,
            redirect_uri: format!("http://127.0.0.1:{port}{CALLBACK_PATH}"),
        })
    }

    /// Serves requests until the matching callback arrives and returns its
    /// authorization code. Timeouts and cancellation belong to the caller.
    pub async fn wait_for_code(self, state: &str) -> Result<String> {
        loop {
            let (mut stream, _) = self.listener.accept().await?;
            let Some(head) = read_head(&mut stream).await else {
                continue;
            };
            let chinese = prefers_chinese(&head);
            match outcome(&head, state) {
                Outcome::Ignore(status) => respond(&mut stream, status, None).await,
                Outcome::Code(code) => {
                    respond(&mut stream, 200, Some(page(true, chinese))).await;
                    return Ok(code);
                }
                Outcome::Denied(reason) => {
                    respond(&mut stream, 200, Some(page(false, chinese))).await;
                    return Err(Error::OAuthDenied(reason));
                }
            }
        }
    }
}

async fn read_head(stream: &mut TcpStream) -> Option<String> {
    let mut buf = Vec::with_capacity(2048);
    let mut chunk = [0u8; 2048];
    while buf.len() < MAX_REQUEST {
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..read]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8(buf).ok()
}

fn outcome(head: &str, state: &str) -> Outcome {
    let target = head.lines().next().and_then(|line| {
        let mut parts = line.split_whitespace();
        (parts.next() == Some("GET"))
            .then(|| parts.next())
            .flatten()
    });
    let Some(target) = target else {
        return Outcome::Ignore(400);
    };
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path != CALLBACK_PATH {
        return Outcome::Ignore(404);
    }
    let params: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();
    let get = |key: &str| {
        params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    };
    if get("state").as_deref() != Some(state) {
        return Outcome::Ignore(400);
    }
    if let Some(error) = get("error") {
        return Outcome::Denied(error);
    }
    match get("code") {
        Some(code) if !code.is_empty() => Outcome::Code(code),
        _ => Outcome::Ignore(400),
    }
}

fn prefers_chinese(head: &str) -> bool {
    head.lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("accept-language")
                .then(|| value.trim().to_ascii_lowercase())
        })
        .is_some_and(|value| value.starts_with("zh"))
}

async fn respond(stream: &mut TcpStream, status: u16, body: Option<String>) {
    let body = body.unwrap_or_default();
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "Bad Request",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

fn page(success: bool, chinese: bool) -> String {
    let (lang, title, detail) = match (success, chinese) {
        (true, true) => (
            "zh-CN",
            "授权完成",
            "账号已添加到 OrbitSwitch，可以关闭此页面。",
        ),
        (true, false) => (
            "en",
            "You're all set",
            "The account was added to OrbitSwitch. You can close this tab.",
        ),
        (false, true) => (
            "zh-CN",
            "授权未完成",
            "Google 没有授予访问权限。回到 OrbitSwitch 重新添加即可。",
        ),
        (false, false) => (
            "en",
            "Sign-in not completed",
            "Google did not grant access. Return to OrbitSwitch to try again.",
        ),
    };
    let mark = if success { "#82aa98" } else { "#d97706" };
    format!(
        r#"<!doctype html><html lang="{lang}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{title} · OrbitSwitch</title>
<style>
:root{{color-scheme:light dark;--bg:#f5f6f8;--card:#fff;--text:#111827;--muted:#6b7280;--line:rgba(17,24,39,.08)}}
@media (prefers-color-scheme:dark){{:root{{--bg:#0d1117;--card:#161b22;--text:#e6edf3;--muted:#8b949e;--line:rgba(240,246,252,.1)}}}}
*{{box-sizing:border-box}}body{{margin:0;min-height:100vh;display:grid;place-items:center;background:var(--bg);color:var(--text);font:15px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",system-ui,sans-serif;padding:16px}}
.card{{width:min(380px,100%);background:var(--card);border:1px solid var(--line);border-radius:16px;padding:32px 28px;text-align:center;box-shadow:0 12px 40px rgba(0,0,0,.08)}}
svg{{width:56px;height:56px;margin-bottom:16px}}h1{{font-size:20px;margin:0 0 6px;font-weight:650;letter-spacing:-.01em}}p{{margin:0;color:var(--muted)}}
</style></head><body><main class="card">
<svg viewBox="0 0 36 36" aria-hidden="true"><path fill="{mark}" fill-rule="evenodd" d="M 20.97 2.28 A 16 16 0 0 1 33.1 23.29 A 1.24 1.24 0 0 1 31.31 23.95 L 28.86 22.53 A 1.24 1.24 0 0 1 28.29 21.11 A 10.75 10.75 0 0 0 20.46 7.53 A 1.24 1.24 0 0 1 19.5 6.33 L 19.5 3.5 A 1.24 1.24 0 0 1 20.97 2.28 Z M 30.13 28.43 A 16 16 0 0 1 5.87 28.43 A 1.24 1.24 0 0 1 6.19 26.55 L 8.64 25.14 A 1.24 1.24 0 0 1 10.16 25.36 A 10.75 10.75 0 0 0 25.84 25.36 A 1.24 1.24 0 0 1 27.36 25.14 L 29.81 26.55 A 1.24 1.24 0 0 1 30.13 28.43 Z M 2.9 23.29 A 16 16 0 0 1 15.03 2.28 A 1.24 1.24 0 0 1 16.5 3.5 L 16.5 6.33 A 1.24 1.24 0 0 1 15.54 7.53 A 10.75 10.75 0 0 0 7.71 21.11 A 1.24 1.24 0 0 1 7.14 22.53 L 4.69 23.95 A 1.24 1.24 0 0 1 2.9 23.29 Z M 17.28 12.77 L 23.15 11.95 A 2.5 2.5 0 0 1 25.61 15.75 L 22.48 20.79 A 5.28 5.28 0 1 1 17.28 12.77 Z M 20.18 18 A 2.18 2.18 0 1 0 15.82 18 A 2.18 2.18 0 1 0 20.18 18 Z"/></svg>
<h1>{title}</h1><p>{detail}</p></main></body></html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(target: &str) -> String {
        format!(
            "GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept-Language: zh-CN,zh;q=0.9\r\n\r\n"
        )
    }

    #[test]
    fn accepts_matching_state_only() {
        assert!(
            matches!(outcome(&head("/oauth-callback?code=c1&state=s"), "s"), Outcome::Code(c) if c == "c1")
        );
        assert!(matches!(
            outcome(&head("/oauth-callback?code=c1&state=x"), "s"),
            Outcome::Ignore(400)
        ));
        assert!(matches!(
            outcome(&head("/favicon.ico"), "s"),
            Outcome::Ignore(404)
        ));
        assert!(matches!(
            outcome(&head("/oauth-callback?error=access_denied&state=s"), "s"),
            Outcome::Denied(e) if e == "access_denied"
        ));
    }

    #[test]
    fn detects_chinese_browsers() {
        assert!(prefers_chinese(&head("/")));
        assert!(!prefers_chinese(
            "GET / HTTP/1.1\r\nAccept-Language: en-US\r\n\r\n"
        ));
    }
}
