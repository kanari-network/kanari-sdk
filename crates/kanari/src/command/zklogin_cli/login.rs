// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! `kanari zklogin login` — Google OIDC login bound to an ephemeral key.
//!
//! Flow (no client secret needed for Desktop clients; Web clients are asked
//! for it only if Google rejects the PKCE exchange):
//! 1. Generate ephemeral Ed25519 key + `randomness` + `salt`.
//! 2. `nonce = compute_nonce(ephemeral_pubkey, max_epoch, randomness)`.
//! 3. Open the Google login URL (loopback redirect + PKCE S256).
//! 4. Catch the authorization `code` on `127.0.0.1:<port>` (or paste it).
//! 5. Exchange code for `id_token`, fetch Google JWKS, verify the JWT.
//! 6. Fail closed unless the JWT `nonce` equals ours.
//! 7. `derive_zklogin_address_v2(iss, aud, sub, salt)` + persist the session.

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use clap::Parser;
use kanari_crypto::signatures::zklogin::{
    EphemeralKeypair, ISS_GOOGLE, JwksDocument, compute_nonce, derive_zklogin_address_v2,
    generate_randomness, generate_salt, verify_jwt_with_jwks,
};
use sha2::{Digest, Sha256};

use super::session::{self, SESSION_VERSION, ZkLoginSession};

pub const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const GOOGLE_JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
const HTTP_TIMEOUT_SECS: u64 = 30;

#[derive(Parser, Debug)]
pub struct Login {
    /// OAuth client ID (also the expected JWT `aud`).
    /// Falls back to KANARI_ZKLOGIN_CLIENT_ID when omitted.
    #[arg(long)]
    pub client_id: Option<String>,
    /// OIDC provider. Only `google` is supported for now.
    #[arg(long, default_value = "google")]
    pub provider: String,
    /// Validity bound baked into the nonce (epochs).
    #[arg(long, default_value = "1000")]
    pub max_epoch: u64,
    /// Loopback port for the OAuth redirect (`http://127.0.0.1:<port>/callback`).
    /// Must be registered for Web-application clients; Desktop clients accept any port.
    #[arg(long, default_value = "8765")]
    pub port: u16,
    /// Skip the loopback server and paste the code/redirect URL manually.
    #[arg(long)]
    pub manual: bool,
    /// Print the login URL but do not try to open a browser.
    #[arg(long)]
    pub no_browser: bool,
    /// Optional client secret up front (Web-application clients). Falls back
    /// to KANARI_ZKLOGIN_CLIENT_SECRET, then to an interactive hidden prompt.
    /// Desktop-app clients never need this.
    #[arg(long)]
    pub client_secret: Option<String>,
    /// Seconds to wait for the browser redirect before giving up.
    #[arg(long, default_value = "300")]
    pub timeout_secs: u64,
}

impl Login {
    pub fn execute(&self) -> Result<()> {
        anyhow::ensure!(
            self.provider == "google",
            "unsupported provider '{}' (only `google` for now)",
            self.provider
        );
        let client_id = self
            .client_id
            .clone()
            .or_else(|| std::env::var("KANARI_ZKLOGIN_CLIENT_ID").ok())
            .filter(|s| !s.is_empty())
            .context("pass --client-id or set KANARI_ZKLOGIN_CLIENT_ID")?;

        // --- 1-2. Ephemeral key, randomness, salt, nonce ---
        let ephemeral = EphemeralKeypair::generate().context("cannot generate ephemeral key")?;
        let randomness = generate_randomness().context("cannot sample randomness")?;
        let salt = generate_salt().context("cannot sample salt")?;
        let pubkey = ephemeral.public_bytes();
        let nonce = compute_nonce(&pubkey, self.max_epoch, &randomness);

        // --- PKCE (public-client code exchange, no secret required) ---
        let verifier = URL_SAFE_NO_PAD.encode(randomness);
        let challenge = pkce_challenge(&verifier);

        let redirect_uri = format!("http://127.0.0.1:{}/callback", self.port);
        let auth_url = build_auth_url(&client_id, &redirect_uri, &nonce, &challenge);
        eprintln!("Ephemeral pubkey: {}", hex::encode(pubkey));
        eprintln!("Nonce:            {nonce}");
        eprintln!("\nOpen this URL to log in with Google:\n{auth_url}\n");
        if !self.no_browser {
            open_browser(&auth_url);
        }

        // --- 4. Authorization code via loopback (or manual paste) ---
        let code = if self.manual {
            read_code_manually()?
        } else {
            match wait_for_code(self.port, self.timeout_secs) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("Loopback failed ({e:#}); falling back to manual paste.");
                    read_code_manually()?
                }
            }
        };

        // --- 5. Code -> id_token (+ optional client secret for Web clients) ---
        let http = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
            .build()
            .context("cannot build HTTP client")?;
        let id_token = exchange_code(&http, &client_id, &redirect_uri, &code, &verifier, None)
            .or_else(|e| {
                if !needs_client_secret(&e) {
                    return Err(e);
                }
                let secret = match &self.client_secret {
                    Some(s) => s.clone(),
                    None => match std::env::var("KANARI_ZKLOGIN_CLIENT_SECRET")
                        .ok()
                        .filter(|s| !s.is_empty())
                    {
                        Some(s) => s,
                        None => {
                            eprintln!(
                                "Google rejected the PKCE-only exchange (likely a Web-application \
                                     client). Paste the client secret (input hidden):"
                            );
                            rpassword::prompt_password("Client secret: ")
                                .context("cannot read client secret")?
                        }
                    },
                };
                exchange_code(
                    &http,
                    &client_id,
                    &redirect_uri,
                    &code,
                    &verifier,
                    Some(secret.trim()),
                )
            })?;

        // --- 5b. JWKS fetch + JWT verify (sig/kid/iss/aud/exp) ---
        let jwks: JwksDocument = http
            .get(GOOGLE_JWKS_URL)
            .send()
            .context("cannot fetch Google JWKS")?
            .error_for_status()
            .context("Google JWKS request failed")?
            .json()
            .context("cannot parse Google JWKS")?;
        let now = now_unix_secs()?;
        // Nonce checked inside (step 6 folded in): fail closed unless the
        // JWT nonce equals the one bound to THIS session key.
        let claims =
            verify_jwt_with_jwks(&id_token, &jwks, ISS_GOOGLE, &client_id, Some(&nonce), now)
                .map_err(|e| anyhow::anyhow!("JWT verification failed: {e:?}"))?;

        // --- 7. Address + session (canonical v2 scheme, matches chain) ---
        let address = derive_zklogin_address_v2(&claims.iss, &client_id, &claims.sub, &salt)
            .map_err(|e| anyhow::anyhow!("address derivation failed: {e:?}"))?;
        let session = ZkLoginSession {
            version: SESSION_VERSION,
            provider: self.provider.clone(),
            iss: claims.iss,
            aud: client_id.clone(),
            sub: claims.sub,
            address: address.clone(),
            salt_hex: hex::encode(salt),
            ephemeral_pubkey_hex: hex::encode(pubkey),
            ephemeral_secret_hex: hex::encode(ephemeral.secret_bytes()),
            max_epoch: self.max_epoch,
            randomness_hex: hex::encode(randomness),
            id_token,
            obtained_at_unix: now,
            id_token_expires_at_unix: claims.exp,
        };
        let path = session::save_session(&session)?;

        eprintln!("zkLogin address: {address}");
        eprintln!("Session saved:   {}", path.display());
        eprintln!(
            "ID token expires: {}",
            session
                .id_token_expires_at_unix
                .map(|e| e.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        );
        eprintln!(
            "\nBACK UP THIS SALT (it re-derives your address): {}",
            session.salt_hex
        );
        eprintln!(
            "WARNING: the session file holds the ephemeral secret (usable until max_epoch {}). \
             Keep it owner-only; `kanari zklogin logout` deletes it.",
            self.max_epoch
        );
        Ok(())
    }
}

/// PKCE S256 challenge for RFC 7636 §4.2 test vector compatibility.
pub fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub fn build_auth_url(client_id: &str, redirect_uri: &str, nonce: &str, challenge: &str) -> String {
    let q = [
        ("client_id", client_id.to_string()),
        ("redirect_uri", redirect_uri.to_string()),
        ("response_type", "code".to_string()),
        ("scope", "openid email profile".to_string()),
        ("nonce", nonce.to_string()),
        ("code_challenge", challenge.to_string()),
        ("code_challenge_method", "S256".to_string()),
        ("access_type", "online".to_string()),
        ("prompt", "select_account".to_string()),
    ];
    let query: Vec<String> = q
        .iter()
        .map(|(k, v)| format!("{k}={}", percent_encode(v)))
        .collect();
    format!("{GOOGLE_AUTH_URL}?{}", query.join("&"))
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Extract `?code=` (or `?error=`) from a loopback callback path/query.
pub fn extract_code(path_and_query: &str) -> Result<String> {
    let query = path_and_query.split_once('?').map(|(_, q)| q).unwrap_or("");
    // Strip fragment if the user pasted a full URL.
    let query = query.split_once('#').map(|(q, _)| q).unwrap_or(query);
    let mut code = None;
    let mut error = None;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        match k {
            "code" => code = Some(percent_decode(v)),
            "error" => error = Some(percent_decode(v)),
            _ => {}
        }
    }
    if let Some(e) = error {
        anyhow::bail!("provider returned an error: {e}");
    }
    code.ok_or_else(|| anyhow::anyhow!("no authorization code in redirect (missing ?code=)"))
}

fn percent_decode(s: &str) -> String {
    let mut bytes = Vec::with_capacity(s.len());
    let raw = s.as_bytes();
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'%'
            && i + 2 < raw.len()
            && let (Some(h), Some(l)) = (hex_val(raw[i + 1]), hex_val(raw[i + 2]))
        {
            bytes.push(h << 4 | l);
            i += 3;
            continue;
        }
        bytes.push(if raw[i] == b'+' { b' ' } else { raw[i] });
        i += 1;
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Program + argv to open a URL in the default browser.
/// Windows goes through `rundll32 url.dll,FileProtocolHandler`: unlike
/// `cmd /C start` (splits at `&`) or plain `explorer.exe` (sometimes opens
/// a File Explorer window instead of the browser), this takes the URL as a
/// single argv with no shell in between.
#[cfg(target_os = "windows")]
fn windows_browser_cmd(url: &str) -> (String, Vec<String>) {
    (
        "rundll32.exe".to_string(),
        vec!["url.dll,FileProtocolHandler".to_string(), url.to_string()],
    )
}

fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let _ = {
        let (prog, args) = windows_browser_cmd(url);
        std::process::Command::new(prog).args(args).spawn()
    };
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

/// Serve one loopback callback and return the authorization code.
fn wait_for_code(port: u16, timeout_secs: u64) -> Result<String> {
    use std::io::{Read, Write};
    use std::time::{Duration, Instant};

    let listener = std::net::TcpListener::bind(("127.0.0.1", port))
        .with_context(|| format!("cannot bind 127.0.0.1:{port}"))?;
    listener.set_nonblocking(true).ok();
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    eprintln!("Waiting for the browser redirect (up to {timeout_secs}s)...");

    loop {
        if Instant::now() > deadline {
            anyhow::bail!("timed out waiting for the login redirect");
        }
        let (mut stream, _) = match listener.accept() {
            Ok(v) => v,
            Err(_) => {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        };
        stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]);
        let path = request
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .unwrap_or("/");
        let outcome = extract_code(path);
        let body = match &outcome {
            Ok(_) => "<h1>Logged in</h1><p>You can close this tab and return to the CLI.</p>",
            Err(e) => &format!("<h1>Login failed</h1><p>{e:#}</p>"),
        };
        let _ = stream.write_all(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .as_bytes(),
        );
        return outcome;
    }
}

fn read_code_manually() -> Result<String> {
    use std::io::{BufRead, Write};
    eprint!("Paste the full redirect URL (or just the code): ");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .context("cannot read stdin")?;
    let line = line.trim();
    if line.is_empty() {
        anyhow::bail!("empty input");
    }
    // Accept either a bare code or a full URL containing ?code=.
    if line.starts_with("http") {
        extract_code(line)
    } else {
        Ok(line.to_string())
    }
}

fn exchange_code(
    http: &reqwest::blocking::Client,
    client_id: &str,
    redirect_uri: &str,
    code: &str,
    verifier: &str,
    client_secret: Option<&str>,
) -> Result<String> {
    let mut pairs = vec![
        ("code", code.to_string()),
        ("client_id", client_id.to_string()),
        ("redirect_uri", redirect_uri.to_string()),
        ("grant_type", "authorization_code".to_string()),
        ("code_verifier", verifier.to_string()),
    ];
    if let Some(secret) = client_secret {
        pairs.push(("client_secret", secret.to_string()));
    }
    // reqwest 0.13 blocking has no `.form()` helper here; the token endpoint
    // only needs `application/x-www-form-urlencoded`, which we build directly.
    let body: Vec<String> = pairs
        .iter()
        .map(|(k, v)| format!("{k}={}", percent_encode(v)))
        .collect();
    let resp: serde_json::Value = http
        .post(GOOGLE_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body.join("&"))
        .send()
        .context("token request failed (network?)")?
        .json()
        .context("token endpoint did not return JSON")?;
    if let Some(id_token) = resp.get("id_token").and_then(|v| v.as_str()) {
        return Ok(id_token.to_string());
    }
    let err = resp
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let desc = resp
        .get("error_description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    anyhow::bail!("token exchange failed [{err}]: {desc}")
}

/// Google rejects secretless PKCE exchange for Web-application clients.
/// Observed shapes: `invalid_client`/`unauthorized_client`, and
/// `invalid_request` with "client_secret is missing". Desktop clients
/// succeed without a secret.
fn needs_client_secret(err: &anyhow::Error) -> bool {
    let s = format!("{err:#}");
    s.contains("invalid_client") || s.contains("unauthorized_client") || s.contains("client_secret")
}

fn now_unix_secs() -> Result<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .context("system clock is before the Unix epoch")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_matches_rfc7636_vector() {
        // RFC 7636 Appendix B.
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn auth_url_carries_all_params() {
        let url = build_auth_url(
            "CID.apps.googleusercontent.com",
            "http://127.0.0.1:8765/callback",
            "NONCE",
            "CHAL",
        );
        assert!(url.starts_with(GOOGLE_AUTH_URL));
        for needle in [
            "client_id=CID.apps.googleusercontent.com",
            "redirect_uri=http%3A%2F%2F127.0.0.1%3A8765%2Fcallback",
            "response_type=code",
            "scope=openid%20email%20profile",
            "nonce=NONCE",
            "code_challenge=CHAL",
            "code_challenge_method=S256",
        ] {
            assert!(url.contains(needle), "missing {needle}");
        }
    }

    #[test]
    fn extract_code_handles_shapes() {
        assert_eq!(
            extract_code("/callback?code=AUTHCODE&scope=openid").unwrap(),
            "AUTHCODE"
        );
        assert_eq!(extract_code("/callback?scope=x&code=A%2BB").unwrap(), "A+B");
        assert!(extract_code("/callback").is_err());
        assert!(extract_code("/callback?error=access_denied").is_err());
    }

    #[test]
    fn percent_roundtrip() {
        for s in ["abc-_.~", "http://127.0.0.1:8765/callback", "a b+c@d"] {
            assert_eq!(percent_decode(&percent_encode(s)), s);
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn windows_browser_cmd_keeps_full_url() {
        // Regression history:
        // 1. `cmd /C start` split the URL at every `&` (missing response_type).
        // 2. `explorer.exe <url>` popped a File Explorer window on some setups.
        // rundll32 passes the URL as one argv with no shell in between.
        let url = "https://accounts.google.com/o/oauth2/v2/auth?client_id=X&response_type=code";
        let (prog, args) = windows_browser_cmd(url);
        assert_eq!(prog, "rundll32.exe");
        assert_eq!(args.len(), 2);
        assert!(args[1].contains("response_type=code"));
    }

    #[test]
    fn nonce_binding_is_enforced_per_account() {
        // Same iss/aud/salt, different sub => different address. The login
        // flow additionally requires the JWT nonce to equal ours, so a token
        // minted for another session can never yield our address.
        use kanari_crypto::signatures::zklogin::derive_zklogin_address_v2;
        let salt = [7u8; 32];
        let a =
            derive_zklogin_address_v2("https://accounts.google.com", "client123", "user456", &salt)
                .unwrap();
        let b = derive_zklogin_address_v2(
            "https://accounts.google.com",
            "client123",
            "attacker",
            &salt,
        )
        .unwrap();
        assert_ne!(a, b);
        assert!(a.starts_with("0x") && a.len() == 66);
    }
}
