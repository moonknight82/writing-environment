use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::time::Duration;

const MAX_CHUNK_UTF16: usize = 15_000;
const MAX_REVIEW_UTF16: usize = 1_000_000;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRequest {
    endpoint: String,
    language: String,
    text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestRequest {
    endpoint: String,
    language: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LanguageToolLanguage {
    #[serde(default)]
    name: String,
    #[serde(default)]
    code: String,
    #[serde(default)]
    long_code: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    address: String,
    language_name: String,
    encrypted: bool,
    private_network: bool,
    loopback: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewReplacement {
    value: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewContext {
    #[serde(default)]
    text: String,
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    length: usize,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewCategory {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRule {
    #[serde(default)]
    id: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    issue_type: String,
    #[serde(default)]
    category: ReviewCategory,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewMatch {
    message: String,
    #[serde(default)]
    short_message: String,
    #[serde(default)]
    replacements: Vec<ReviewReplacement>,
    offset: usize,
    length: usize,
    #[serde(default)]
    context: ReviewContext,
    #[serde(default)]
    rule: ReviewRule,
}

#[derive(Deserialize)]
struct LanguageToolResponse {
    #[serde(default)]
    matches: Vec<ReviewMatch>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewResult {
    matches: Vec<ReviewMatch>,
    checked_characters: usize,
    request_count: usize,
}

#[derive(Debug, Eq, PartialEq)]
struct TextChunk<'a> {
    start_utf16: usize,
    text: &'a str,
}

#[tauri::command]
pub async fn check_grammar_style(request: ReviewRequest) -> Result<ReviewResult, String> {
    let endpoint = review_endpoint(&request.endpoint)?;
    let language = review_language(&request.language)?;
    let character_count = request.text.encode_utf16().count();
    if character_count > MAX_REVIEW_UTF16 {
        return Err(format!(
            "Review supports up to {MAX_REVIEW_UTF16} characters in one sheet."
        ));
    }

    // The updater and review client share rustls. Installing the existing ring
    // provider here avoids pulling a second crypto implementation into Pi builds.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .user_agent("Writing Environment grammar review")
        .build()
        .map_err(|error| format!("Cannot prepare the grammar checker: {error}"))?;
    let review_text = mask_markdown_for_review(&request.text);
    debug_assert_eq!(review_text.encode_utf16().count(), character_count);
    let chunks = text_chunks(&review_text, MAX_CHUNK_UTF16);
    let mut matches = Vec::new();

    for chunk in &chunks {
        let response = client
            .post(endpoint.clone())
            .form(&[("language", language), ("text", chunk.text)])
            .send()
            .await
            .map_err(|error| connection_error(&endpoint, error))?;
        let status = response.status();
        if !status.is_success() {
            let detail = response.text().await.unwrap_or_default();
            let detail = detail.trim().chars().take(240).collect::<String>();
            return Err(if detail.is_empty() {
                format!("The grammar checker returned {status}.")
            } else {
                format!("The grammar checker returned {status}: {detail}")
            });
        }
        let mut checked = response
            .json::<LanguageToolResponse>()
            .await
            .map_err(|error| {
                format!("The grammar checker returned an unreadable response: {error}")
            })?;
        for finding in &mut checked.matches {
            finding.offset = finding.offset.saturating_add(chunk.start_utf16);
        }
        matches.append(&mut checked.matches);
    }

    matches.sort_by_key(|finding| finding.offset);
    Ok(ReviewResult {
        matches,
        checked_characters: character_count,
        request_count: chunks.len(),
    })
}

#[tauri::command]
pub async fn test_language_tool_connection(
    request: ConnectionTestRequest,
) -> Result<ConnectionTestResult, String> {
    let endpoint = review_endpoint(&request.endpoint)?;
    let language = review_language(&request.language)?;
    let languages_endpoint = languages_endpoint(&endpoint)?;
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(12))
        .user_agent("Writing Environment LanguageTool connection test")
        .build()
        .map_err(|error| format!("Cannot prepare the grammar checker: {error}"))?;
    let response = client
        .get(languages_endpoint)
        .send()
        .await
        .map_err(|error| connection_error(&endpoint, error))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "The LanguageTool server returned {status} during the connection test."
        ));
    }
    let languages = response
        .json::<Vec<LanguageToolLanguage>>()
        .await
        .map_err(|error| {
            format!("The server did not return a LanguageTool language list: {error}")
        })?;
    let selected = languages
        .iter()
        .find(|item| item.long_code == language || item.code == language)
        .ok_or_else(|| {
            format!("This LanguageTool server does not report support for {language}.")
        })?;

    Ok(ConnectionTestResult {
        address: endpoint_origin(&endpoint),
        language_name: selected.name.clone(),
        encrypted: endpoint.scheme() == "https",
        private_network: endpoint_is_private(&endpoint),
        loopback: endpoint_is_loopback(&endpoint),
    })
}

fn review_endpoint(value: &str) -> Result<Url, String> {
    let mut endpoint = Url::parse(value.trim()).map_err(|_| {
        "Enter a complete LanguageTool address beginning with http:// or https://.".to_string()
    })?;
    if !matches!(endpoint.scheme(), "http" | "https") {
        return Err("The LanguageTool address must use http:// or https://.".into());
    }
    if !endpoint.username().is_empty() || endpoint.password().is_some() {
        return Err("Do not put credentials in the LanguageTool address.".into());
    }
    if endpoint.fragment().is_some() {
        return Err("The LanguageTool address cannot contain a fragment.".into());
    }
    if endpoint.query().is_some() {
        return Err("The LanguageTool address cannot contain query parameters.".into());
    }
    if endpoint.scheme() == "http" && !endpoint_is_private(&endpoint) {
        return Err(
            "Unencrypted LanguageTool addresses are allowed only on this computer or a private network. Use HTTPS for any other self-hosted server."
                .into(),
        );
    }
    match endpoint.path().trim_end_matches('/') {
        "" => endpoint.set_path("/v2/check"),
        "/v2" => endpoint.set_path("/v2/check"),
        _ => {}
    }
    Ok(endpoint)
}

fn languages_endpoint(endpoint: &Url) -> Result<Url, String> {
    let mut languages = endpoint.clone();
    let path = languages.path().trim_end_matches('/');
    let Some(prefix) = path.strip_suffix("/check") else {
        return Err("Use a LanguageTool base address or an address ending in /v2/check.".into());
    };
    languages.set_path(&format!("{prefix}/languages"));
    Ok(languages)
}

fn endpoint_origin(endpoint: &Url) -> String {
    let host = endpoint.host_str().unwrap_or("configured server");
    match endpoint.port() {
        Some(port) => format!("{}://{host}:{port}", endpoint.scheme()),
        None => format!("{}://{host}", endpoint.scheme()),
    }
}

fn endpoint_is_private(endpoint: &Url) -> bool {
    let Some(host) = endpoint.host_str() else {
        return false;
    };
    let host = host.trim_matches(['[', ']']).to_ascii_lowercase();
    if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") {
        return true;
    }
    if let Ok(address) = host.parse::<IpAddr>() {
        return match address {
            IpAddr::V4(address) => {
                address.is_private() || address.is_loopback() || address.is_link_local()
            }
            IpAddr::V6(address) => {
                address.is_loopback()
                    || address.is_unique_local()
                    || address.is_unicast_link_local()
            }
        };
    }
    // Single-label hostnames are resolved by the writer's LAN/DNS configuration.
    !host.contains('.')
}

fn endpoint_is_loopback(endpoint: &Url) -> bool {
    let Some(host) = endpoint.host_str() else {
        return false;
    };
    let host = host.trim_matches(['[', ']']).to_ascii_lowercase();
    if host == "localhost" || host.ends_with(".localhost") {
        return true;
    }
    host.parse::<IpAddr>()
        .map(|address| address.is_loopback())
        .unwrap_or(false)
}

fn review_language(value: &str) -> Result<&str, String> {
    match value {
        "en-US" | "en-GB" | "pt-BR" | "pt-PT" => Ok(value),
        _ => Err("Choose a supported review language.".into()),
    }
}

fn connection_error(endpoint: &Url, error: reqwest::Error) -> String {
    let location = endpoint
        .host_str()
        .map(|host| match endpoint.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_string(),
        })
        .unwrap_or_else(|| "the configured service".into());
    if error.is_timeout() {
        format!("The grammar checker at {location} did not respond in time.")
    } else {
        format!("Cannot reach the grammar checker at {location}: {error}")
    }
}

fn text_chunks(text: &str, max_utf16: usize) -> Vec<TextChunk<'_>> {
    if text.is_empty() {
        return vec![TextChunk {
            start_utf16: 0,
            text,
        }];
    }

    let max_utf16 = max_utf16.max(2);
    let mut chunks = Vec::new();
    let mut remaining = text;
    let mut start_utf16 = 0;
    while !remaining.is_empty() {
        let mut utf16_length = 0;
        let mut byte_limit = remaining.len();
        let mut preferred_break = None;
        for (byte_index, character) in remaining.char_indices() {
            let character_length = character.len_utf16();
            if utf16_length + character_length > max_utf16 {
                byte_limit = byte_index;
                break;
            }
            utf16_length += character_length;
            if character == '\n' {
                preferred_break = Some(byte_index + character.len_utf8());
            }
        }

        if byte_limit == remaining.len() {
            chunks.push(TextChunk {
                start_utf16,
                text: remaining,
            });
            break;
        }

        let minimum_preferred = max_utf16 / 2;
        let split_at = preferred_break
            .filter(|byte_index| {
                remaining[..*byte_index].encode_utf16().count() >= minimum_preferred
            })
            .unwrap_or(byte_limit);
        let (chunk, rest) = remaining.split_at(split_at);
        chunks.push(TextChunk {
            start_utf16,
            text: chunk,
        });
        start_utf16 += chunk.encode_utf16().count();
        remaining = rest;
    }
    chunks
}

fn mask_markdown_for_review(text: &str) -> String {
    let mut masked = vec![false; text.len()];
    let mut offset = 0;
    let mut in_frontmatter = text
        .lines()
        .next()
        .map(|line| line.trim_end_matches('\r') == "---")
        .unwrap_or(false);
    let mut frontmatter_started = false;
    let mut fence: Option<&str> = None;

    for line_with_newline in text.split_inclusive('\n') {
        let line = line_with_newline.trim_end_matches(['\r', '\n']);
        let line_end = offset + line.len();
        let trimmed = line.trim_start();
        let leading = line.len() - trimmed.len();

        if in_frontmatter {
            mark_range(&mut masked, offset, line_end);
            if frontmatter_started && matches!(trimmed, "---" | "...") {
                in_frontmatter = false;
            }
            frontmatter_started = true;
            offset += line_with_newline.len();
            continue;
        }

        if let Some(marker) = fence {
            mark_range(&mut masked, offset, line_end);
            if trimmed.starts_with(marker) {
                fence = None;
            }
            offset += line_with_newline.len();
            continue;
        }

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = Some(if trimmed.starts_with("```") {
                "```"
            } else {
                "~~~"
            });
            mark_range(&mut masked, offset, line_end);
            offset += line_with_newline.len();
            continue;
        }

        let prefix = markdown_line_prefix_length(trimmed);
        if prefix > 0 {
            mark_range(&mut masked, offset + leading, offset + leading + prefix);
        }
        if is_thematic_break(trimmed) {
            mark_range(&mut masked, offset, line_end);
        }
        mask_inline_markdown(text, &mut masked, offset, line_end);
        offset += line_with_newline.len();
    }

    text.char_indices()
        .flat_map(|(byte_index, character)| {
            if character != '\n' && masked[byte_index] {
                vec![' '; character.len_utf16()]
            } else {
                vec![character]
            }
        })
        .collect()
}

fn markdown_line_prefix_length(line: &str) -> usize {
    if line.starts_with('#') {
        let count = line.bytes().take_while(|byte| *byte == b'#').count();
        if count <= 6 && line.as_bytes().get(count) == Some(&b' ') {
            return count + 1;
        }
    }
    if line.starts_with("> ") {
        return 2;
    }
    if matches!(line.as_bytes().first(), Some(b'-' | b'+' | b'*'))
        && line.as_bytes().get(1) == Some(&b' ')
    {
        return 2;
    }
    let digits = line
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digits > 0
        && matches!(line.as_bytes().get(digits), Some(b'.' | b')'))
        && line.as_bytes().get(digits + 1) == Some(&b' ')
    {
        return digits + 2;
    }
    0
}

fn is_thematic_break(line: &str) -> bool {
    let symbols = line
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<Vec<_>>();
    symbols.len() >= 3
        && symbols
            .iter()
            .all(|character| *character == symbols[0] && matches!(character, '-' | '*' | '_'))
}

fn mask_inline_markdown(text: &str, masked: &mut [bool], start: usize, end: usize) {
    let bytes = text.as_bytes();
    let mut index = start;
    while index < end {
        if masked[index] {
            index += 1;
            continue;
        }
        if bytes[index] == b'`' {
            let run = bytes[index..end]
                .iter()
                .take_while(|byte| **byte == b'`')
                .count();
            let marker = &text[index..index + run];
            if let Some(relative) = text[index + run..end].find(marker) {
                let close = index + run + relative + run;
                mark_range(masked, index, close);
                index = close;
                continue;
            }
        }
        if bytes[index] == b'<' {
            if let Some(relative) = text[index..end].find('>') {
                let close = index + relative + 1;
                mark_range(masked, index, close);
                index = close;
                continue;
            }
        }
        if bytes[index] == b'[' {
            if let Some(label_close_relative) = text[index + 1..end].find("](") {
                let label_close = index + 1 + label_close_relative;
                if let Some(destination_close_relative) = text[label_close + 2..end].find(')') {
                    let destination_close = label_close + 2 + destination_close_relative + 1;
                    mark_range(masked, index, index + 1);
                    mark_range(masked, label_close, destination_close);
                    if index > start && bytes[index - 1] == b'!' {
                        mark_range(masked, index - 1, index);
                    }
                    index = destination_close;
                    continue;
                }
            }
        }
        if matches!(bytes[index], b'*' | b'_' | b'~') || (bytes[index] == b'\\' && index + 1 < end)
        {
            mark_range(masked, index, index + 1);
        }
        index += 1;
    }
}

fn mark_range(masked: &mut [bool], start: usize, end: usize) {
    let length = masked.len();
    for value in &mut masked[start.min(length)..end.min(length)] {
        *value = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn endpoint_accepts_a_base_or_complete_language_tool_url() {
        assert_eq!(
            review_endpoint("http://127.0.0.1:8081").unwrap().as_str(),
            "http://127.0.0.1:8081/v2/check"
        );
        assert_eq!(
            review_endpoint("https://example.test/custom/check")
                .unwrap()
                .as_str(),
            "https://example.test/custom/check"
        );
        assert!(review_endpoint("file:///tmp/check").is_err());
        assert!(review_endpoint("https://user:secret@example.test/v2/check").is_err());
        assert!(review_endpoint("http://example.test/v2/check").is_err());
        assert!(review_endpoint("https://example.test/v2/check?apiKey=secret").is_err());
        assert!(review_endpoint("http://192.168.0.125:8081/v2/check").is_ok());
        assert!(review_endpoint("http://stargazer:8081/v2/check").is_ok());
    }

    #[test]
    fn derives_the_no_text_language_endpoint() {
        let endpoint = review_endpoint("http://127.0.0.1:8081/v2/check").unwrap();
        assert_eq!(
            languages_endpoint(&endpoint).unwrap().as_str(),
            "http://127.0.0.1:8081/v2/languages"
        );
    }

    #[test]
    fn markdown_masking_preserves_utf16_offsets_and_visible_prose() {
        let source = "---\ntitle: Hidden\n---\n# The *wrong* heading 🌊\n\n- This are [visible words](https://example.test).\n\n`code are hidden`\n```rust\nlet sentence = wrong;\n```\n";
        let masked = mask_markdown_for_review(source);
        assert_eq!(masked.encode_utf16().count(), source.encode_utf16().count());
        assert_eq!(masked.matches('\n').count(), source.matches('\n').count());
        assert!(masked.contains("The  wrong  heading 🌊"));
        assert!(masked.contains("This are  visible words"));
        assert!(!masked.contains("Hidden"));
        assert!(!masked.contains("example.test"));
        assert!(!masked.contains("code are hidden"));
        assert!(!masked.contains("sentence = wrong"));
    }

    #[test]
    fn chunks_preserve_all_text_and_utf16_offsets() {
        let text = "First paragraph.\nSecond 🌊 paragraph.\nThird paragraph.";
        let chunks = text_chunks(text, 24);
        assert!(chunks.len() > 1);
        assert_eq!(
            chunks.iter().map(|chunk| chunk.text).collect::<String>(),
            text
        );
        for (index, chunk) in chunks.iter().enumerate() {
            let prior = chunks[..index]
                .iter()
                .map(|item| item.text.encode_utf16().count())
                .sum::<usize>();
            assert_eq!(chunk.start_utf16, prior);
            assert!(chunk.text.encode_utf16().count() <= 24);
        }
    }

    #[test]
    fn a_short_or_empty_sheet_uses_one_request() {
        assert_eq!(text_chunks("A short sheet.", 100).len(), 1);
        assert_eq!(text_chunks("", 100).len(), 1);
    }

    #[test]
    fn local_http_response_becomes_review_findings() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
        let address = listener.local_addr().expect("local test address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept review request");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 2048];
            loop {
                let read = stream.read(&mut buffer).expect("read review request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n")
                else {
                    continue;
                };
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|value| value.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if request.len() >= header_end + 4 + content_length {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&request);
            assert!(request.contains("language=en-US"));
            assert!(request.contains("text=This+are+wrong."));

            let body = r#"{"matches":[{"message":"Possible agreement error.","shortMessage":"Agreement","replacements":[{"value":"is"}],"offset":5,"length":3,"context":{"text":"This are wrong.","offset":5,"length":3},"rule":{"id":"AGREEMENT","description":"Agreement","issueType":"grammar","category":{"id":"GRAMMAR","name":"Grammar"}}}]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .expect("write review response");
        });

        let result = tauri::async_runtime::block_on(check_grammar_style(ReviewRequest {
            endpoint: format!("http://{address}/v2/check"),
            language: "en-US".into(),
            text: "This are wrong.".into(),
        }))
        .expect("check local LanguageTool response");
        server.join().expect("join local test server");

        assert_eq!(result.checked_characters, 15);
        assert_eq!(result.request_count, 1);
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].offset, 5);
        assert_eq!(result.matches[0].replacements[0].value, "is");
        assert_eq!(result.matches[0].rule.category.name, "Grammar");
    }

    #[test]
    fn connection_test_reads_languages_without_sending_manuscript_text() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
        let address = listener.local_addr().expect("local test address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept connection test");
            let mut request = [0_u8; 2048];
            let read = stream.read(&mut request).expect("read connection test");
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request.starts_with("GET /v2/languages HTTP/1.1"));
            assert!(!request.contains("text="));

            let body = r#"[{"name":"English (US)","code":"en","longCode":"en-US"}]"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .expect("write language response");
        });

        let result =
            tauri::async_runtime::block_on(test_language_tool_connection(ConnectionTestRequest {
                endpoint: format!("http://{address}/v2/check"),
                language: "en-US".into(),
            }))
            .expect("test local LanguageTool connection");
        server.join().expect("join local test server");

        assert_eq!(result.address, format!("http://{address}"));
        assert_eq!(result.language_name, "English (US)");
        assert!(!result.encrypted);
        assert!(result.private_network);
        assert!(result.loopback);
    }
}
