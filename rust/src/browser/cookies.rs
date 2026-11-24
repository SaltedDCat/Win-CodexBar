//! Cookie values and request-host filtering.

use thiserror::Error;

use super::detection::BrowserType;
use super::session_scan::BrowserCookieScan;

/// Errors that can occur during cookie extraction
#[derive(Debug, Error)]
pub enum CookieError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("No encryption key found")]
    NoEncryptionKey,

    #[error("Cookie not found for domain: {0}")]
    NotFound(String),

    #[error("Browser not installed")]
    BrowserNotInstalled,

    #[error("DPAPI error: {0}")]
    Dpapi(String),

    /// Chromium App-Bound Encryption (ABE) is protecting cookie values.
    /// The user-level DPAPI key in Local State can no longer decrypt cookies encrypted
    /// after the ABE migration. Modern Chromium-based browsers can write these cookies
    /// with a `v20` prefix; older migrated profiles can also fail every AES-GCM decrypt
    /// while exposing `app_bound_encrypted_key` in Local State.
    ///
    /// `browsers` names the browsers ABE blocked, in scan order without repeats.
    /// A single browser read reports its own browser; a scan that finds nothing
    /// merges every blocked browser it saw. An empty list means the browser was
    /// not attributed (only the raw per-cookie decrypt reports that).
    #[error("{}", app_bound_encryption_message(.browsers))]
    AppBoundEncryption { browsers: Vec<BrowserType> },
}

/// Display text for [`CookieError::AppBoundEncryption`]. Names the blocked
/// browsers so a multi-browser failure says which imports to work around.
fn app_bound_encryption_message(browsers: &[BrowserType]) -> String {
    if browsers.is_empty() {
        return "Chromium App-Bound Encryption is blocking automatic browser import. \
                Paste the Cookie header manually, or use Firefox if that browser has the same login."
            .to_string();
    }
    let names: Vec<&str> = browsers
        .iter()
        .map(|browser| browser.display_name())
        .collect();
    let list = join_names(&names);
    if browsers.len() == 1 {
        format!(
            "Chromium App-Bound Encryption blocked automatic import from {list}. \
             Paste the Cookie header manually, or use Firefox if that browser has the same login."
        )
    } else {
        format!(
            "Chromium App-Bound Encryption blocked automatic import from {list}. \
             Paste the Cookie header manually, or sign in with Firefox instead."
        )
    }
}

/// Join display names as "A", "A and B", or "A, B and C".
fn join_names(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [only] => (*only).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// A browser cookie
#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<i64>,
    pub is_secure: bool,
    pub is_http_only: bool,
}

impl Cookie {
    /// Format as a cookie header value
    pub fn to_header_value(&self) -> String {
        format!("{}={}", self.name, self.value)
    }
}

/// The cookies a browser sends with a request to `host`, for sites whose
/// session cookies live on a parent domain shared with unrelated sites.
///
/// A plain domain read returns the domain and every subdomain, so reading the
/// parent (`google.com`) would also pick up cookies for unrelated sibling
/// hosts. The scope reads `scan_domain` and keeps only cookies stored for
/// `host` itself or for one of its parent domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CookieRequestScope {
    /// Domain whose cookie rows are read (the host or one of its parents).
    pub scan_domain: &'static str,
    /// Host the request goes to.
    pub host: &'static str,
}

impl CookieRequestScope {
    /// Whether a cookie stored under `host_key` is sent to the scope's host.
    ///
    /// A leading dot marks a domain cookie, which a browser sends to the
    /// domain and its subdomains. Without the dot the cookie is host-only and
    /// is sent only on an exact host match, so a cookie stored as
    /// `google.com` must not leak to `gemini.google.com`.
    pub fn applies_to(&self, host_key: &str) -> bool {
        let host = self.host.to_ascii_lowercase();
        let trimmed = host_key.trim();
        if let Some(dotted) = trimmed.strip_prefix('.') {
            let domain = dotted.trim_end_matches('.').to_ascii_lowercase();
            !domain.is_empty() && (host == domain || host.ends_with(&format!(".{domain}")))
        } else {
            let stored = trimmed.trim_end_matches('.').to_ascii_lowercase();
            !stored.is_empty() && host == stored
        }
    }

    /// Keep only the cookies sent to the scope's host.
    pub fn retain(&self, cookies: &mut Vec<Cookie>) {
        cookies.retain(|cookie| self.applies_to(&cookie.domain));
    }
}

/// Cookie extractor for browsers
pub struct CookieExtractor;

impl CookieExtractor {
    /// Build a cookie header string for HTTP requests
    pub fn build_cookie_header(cookies: &[Cookie]) -> String {
        cookies
            .iter()
            .map(|c| c.to_header_value())
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Helper to get cookies for a specific domain from any available browser.
///
/// Stops at the first browser with any matching cookie and reads no further
/// browser. Providers that validate the session and must move on after an
/// auth failure iterate a [`BrowserCookieScan`] instead.
pub fn get_cookies_for_domain(domain: &str) -> Result<Vec<Cookie>, CookieError> {
    let mut scan = BrowserCookieScan::new(&[domain]);
    match scan.next() {
        Some(candidate) => Ok(candidate.cookies),
        None => Err(scan.exhaustion_error()),
    }
}

/// Get cookie-header candidates from every detected browser that has readable
/// cookies for a domain.
///
/// Reads every browser up front; prefer iterating a [`BrowserCookieScan`] so
/// later browsers are read only when an earlier session is rejected.
pub fn get_cookie_headers_for_domain(
    domain: &str,
) -> Result<Vec<(BrowserType, String)>, CookieError> {
    let mut scan = BrowserCookieScan::new(&[domain]);
    let headers = scan
        .by_ref()
        .map(|candidate| (candidate.browser, candidate.header()))
        .collect::<Vec<_>>();
    if headers.is_empty() {
        return Err(scan.exhaustion_error());
    }
    Ok(headers)
}

/// Get cookie candidates from every detected browser that has readable cookies
/// for a domain, preserving browser order and cookie metadata for provider-side
/// host filtering. Reports App-Bound Encryption when it is the only blocker.
pub fn get_cookies_by_browser_for_domain(
    domain: &str,
) -> Result<Vec<(BrowserType, Vec<Cookie>)>, CookieError> {
    let mut scan = BrowserCookieScan::new(&[domain]);
    let candidates = scan
        .by_ref()
        .map(|candidate| (candidate.browser, candidate.cookies))
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Err(scan.exhaustion_error());
    }
    Ok(candidates)
}

/// Get a cookie header string for a domain
pub fn get_cookie_header(domain: &str) -> Result<String, CookieError> {
    let cookies = get_cookies_for_domain(domain)?;
    Ok(CookieExtractor::build_cookie_header(&cookies))
}

/// Get a cookie header string from the first domain that has readable cookies.
pub fn get_cookie_header_for_domains(domains: &[&str]) -> Result<String, CookieError> {
    let mut scan = BrowserCookieScan::new(domains);
    match scan.next() {
        Some(candidate) => Ok(candidate.header()),
        None => Err(scan.exhaustion_error()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_scope_keeps_host_and_dotted_parent_cookies_only() {
        let scope = CookieRequestScope {
            scan_domain: "example.com",
            host: "app.example.com",
        };
        assert!(scope.applies_to("app.example.com"));
        assert!(scope.applies_to(".app.example.com"));
        assert!(scope.applies_to(".example.com"));
        assert!(!scope.applies_to("example.com"));
        assert!(!scope.applies_to("EXAMPLE.COM."));
        assert!(!scope.applies_to("mail.example.com"));
        assert!(!scope.applies_to("deep.app.example.com"));
        assert!(!scope.applies_to("badexample.com"));
        assert!(!scope.applies_to(""));
    }

    #[test]
    fn host_only_cookie_is_not_sent_to_subdomains() {
        let subdomain = CookieRequestScope {
            scan_domain: "example.com",
            host: "app.example.com",
        };
        // A host-only cookie (stored without a leading dot) is sent to the
        // exact host only; a browser would not send it to a subdomain.
        assert!(!subdomain.applies_to("example.com"));
        assert!(subdomain.applies_to(".example.com"));

        let exact = CookieRequestScope {
            scan_domain: "example.com",
            host: "example.com",
        };
        assert!(exact.applies_to("example.com"));
        assert!(exact.applies_to(".example.com"));
    }
}
