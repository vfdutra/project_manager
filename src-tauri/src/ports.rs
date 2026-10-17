use regex::Regex;
use std::sync::LazyLock;

static ANSI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\x1b\[[0-9;?]*[A-Za-z]").unwrap());
static URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://(?:localhost|127\.0\.0\.1|0\.0\.0\.0|\[::1?\]):(\d{2,5})").unwrap()
});
static PORT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:listening|running|started|serving)\b.*?\bport\b\s*:?\s*(\d{2,5})").unwrap()
});

pub fn strip_ansi(s: &str) -> String {
    ANSI.replace_all(s, "").into_owned()
}

/// First local URL announced in a log line, normalised to http(s)://localhost:PORT.
// ponytail: log scraping, misses servers that never print their port; the per-project URL override covers those.
pub fn extract_url(line: &str) -> Option<String> {
    let caps = URL.captures(line).or_else(|| PORT.captures(line))?;
    let https = line[caps.get(0)?.start()..].starts_with("https");
    Some(format!("{}://localhost:{}", if https { "https" } else { "http" }, &caps[1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(line: &str) -> Option<String> {
        extract_url(&strip_ansi(line))
    }

    #[test]
    fn known_dev_servers() {
        assert_eq!(url("  ➜  Local:   http://localhost:5173/").as_deref(), Some("http://localhost:5173"));
        assert_eq!(url("Starting development server at http://127.0.0.1:8000/").as_deref(), Some("http://localhost:8000"));
        assert_eq!(url("Uvicorn running on http://0.0.0.0:8080 (Press CTRL+C to quit)").as_deref(), Some("http://localhost:8080"));
        assert_eq!(url("PHP 8.3 Development Server (http://localhost:8000) started").as_deref(), Some("http://localhost:8000"));
        assert_eq!(url("Server listening on port 3000").as_deref(), Some("http://localhost:3000"));
        assert_eq!(url("App running on port: 4000").as_deref(), Some("http://localhost:4000"));
        assert_eq!(url("Local: https://localhost:8443/").as_deref(), Some("https://localhost:8443"));
    }

    #[test]
    fn colored_vite_output() {
        assert_eq!(url("http://localhost:\x1b[1m5173\x1b[22m/").as_deref(), Some("http://localhost:5173"));
    }

    #[test]
    fn unrelated_lines() {
        assert_eq!(url("compiled successfully in 1.2s"), None);
        assert_eq!(url("Network: http://192.168.0.10:5173/"), None);
    }

    #[test]
    fn strip_ansi_removes_escape_codes() {
        assert_eq!(strip_ansi("\x1b[32mok\x1b[0m"), "ok");
    }
}
