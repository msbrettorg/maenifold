//! Message chunking for Teams long-message delivery.
//!
//! Ported from agency-cowork/skills/teams/scripts/chunking.py.
//! Teams chat messages render poorly above ~4000 chars; this module splits
//! long messages into numbered chunks sent as sequential messages.

use std::time::Duration;

// Source: chunking.py:24-27
pub const DEFAULT_MAX_CHUNK: usize = 3800;
pub const MAX_CHUNKS: usize = 8;
pub const CHUNK_OVERHEAD: usize = 20;
pub const INTER_CHUNK_DELAY: Duration = Duration::from_millis(300);

/// Split `text` into chunks that each fit within `max_len` characters.
///
/// Splitting strategy (priority order): paragraph (`\n\n`) → line (`\n`)
/// → sentence (`[.!?]\s`) → space → hard cut. Each chunk except in the
/// single-chunk case gets a `(i/N)` suffix.
///
/// Source: chunking.py:30-78 `chunk_message()`.
pub fn chunk_message(text: &str, max_len: usize) -> Vec<String> {
    chunk_message_with_cap(text, max_len, MAX_CHUNKS)
}

/// Source: chunking.py:30-78 `chunk_message()` (with explicit max_chunks).
pub fn chunk_message_with_cap(text: &str, max_len: usize, max_chunks: usize) -> Vec<String> {
    if text.is_empty() || text.trim().is_empty() {
        return vec![String::new()];
    }

    // Fast path: fits in one message. Source: chunking.py:55-56
    if text.chars().count() <= max_len {
        return vec![text.to_string()];
    }

    // Source: chunking.py:58-65
    let usable = if max_len > CHUNK_OVERHEAD && (max_len - CHUNK_OVERHEAD) >= 200 {
        max_len - CHUNK_OVERHEAD
    } else {
        // Degenerate: skip suffixes
        let mut raw = split_on_boundaries(text, max_len);
        if raw.len() > max_chunks {
            raw.truncate(max_chunks);
        }
        return raw;
    };

    let mut raw_chunks = split_on_boundaries(text, usable);

    // Enforce max_chunks cap — merge trailing chunks. Source: chunking.py:69-77
    if raw_chunks.len() > max_chunks {
        let tail: Vec<String> = raw_chunks.split_off(max_chunks - 1);
        let mut remainder = tail.join("\n\n");
        if remainder.chars().count() > usable {
            let cut = char_byte_index(&remainder, usable.saturating_sub(20));
            remainder.truncate(cut);
            remainder.push_str("\n\n…(truncated)");
        }
        raw_chunks.push(remainder);
    }

    // Source: chunking.py:80-81
    if raw_chunks.len() == 1 {
        return raw_chunks;
    }

    // Add (i/N) suffixes. Source: chunking.py:84-85
    let total = raw_chunks.len();
    raw_chunks
        .into_iter()
        .enumerate()
        .map(|(i, c)| format!("{}\n\n({}/{})", c, i + 1, total))
        .collect()
}

/// Recursively split `text` on the best available boundary.
/// Source: chunking.py:88-119 `_split_on_boundaries()`.
fn split_on_boundaries(text: &str, max_len: usize) -> Vec<String> {
    if text.chars().count() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks: Vec<String> = Vec::new();
    let mut remaining = text.to_string();

    while !remaining.is_empty() {
        if remaining.chars().count() <= max_len {
            chunks.push(remaining);
            break;
        }

        let split_char_pos = find_split_point(&remaining, max_len);
        let split_byte = char_byte_index(&remaining, split_char_pos);

        // Preserve paragraph/line boundary. Source: chunking.py:104-110
        let (chunk, rest) = if split_byte > 0 && remaining.as_bytes()[split_byte - 1] == b'\n' {
            let c = remaining[..split_byte].to_string();
            let r = remaining[split_byte..].trim_start_matches('\n').to_string();
            (c, r)
        } else {
            let c = remaining[..split_byte].trim_end().to_string();
            let r = remaining[split_byte..].trim_start_matches('\n').to_string();
            (c, r)
        };

        let chunk_empty = chunk.is_empty();
        if !chunk_empty {
            chunks.push(chunk);
            remaining = rest;
        } else if !rest.is_empty() {
            // Safety: avoid infinite loop. Source: chunking.py:115-117
            let cut = char_byte_index(&remaining, max_len);
            chunks.push(remaining[..cut].to_string());
            remaining = remaining[cut..].to_string();
        } else {
            break;
        }
    }

    chunks
}

/// Find the best position (in chars) to split `text` within `max_len` chars.
/// Source: chunking.py:122-153 `_find_split_point()`.
fn find_split_point(text: &str, max_len: usize) -> usize {
    let window: String = text.chars().take(max_len).collect();
    let quarter = max_len / 4;

    // 1. Paragraph boundary
    if let Some(b) = window.rfind("\n\n") {
        let p = window[..b].chars().count();
        if p > quarter {
            return p + 1;
        }
    }

    // 2. Line boundary
    if let Some(b) = window.rfind('\n') {
        let p = window[..b].chars().count();
        if p > quarter {
            return p + 1;
        }
    }

    // 3. Sentence boundary ([.!?] followed by whitespace)
    let chars: Vec<char> = window.chars().collect();
    let mut last_sentence: Option<usize> = None;
    for i in 0..chars.len().saturating_sub(1) {
        if matches!(chars[i], '.' | '!' | '?') && chars[i + 1].is_whitespace() {
            let end = i + 2;
            if end > quarter {
                last_sentence = Some(end);
            }
        }
    }
    if let Some(e) = last_sentence {
        return e;
    }

    // 4. Word boundary
    if let Some(b) = window.rfind(' ') {
        let p = window[..b].chars().count();
        if p > quarter {
            return p + 1;
        }
    }

    // 5. Hard cut
    max_len
}

/// Convert a char-count index to a byte index within `s`.
fn char_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_returned_as_is() {
        assert_eq!(chunk_message("hello", 100), vec!["hello".to_string()]);
    }

    #[test]
    fn empty_text_returns_empty() {
        assert_eq!(chunk_message("", 100), vec!["".to_string()]);
        assert_eq!(chunk_message("   ", 100), vec!["".to_string()]);
    }

    #[test]
    fn long_text_splits_on_paragraph() {
        let para = "a".repeat(500);
        let text = format!("{}\n\n{}\n\n{}", para, para, para);
        let chunks = chunk_message(&text, 600);
        assert!(chunks.len() >= 2);
        for c in &chunks {
            assert!(c.chars().count() <= 600, "chunk too long: {}", c.len());
        }
        assert!(chunks.last().unwrap().contains("/"));
    }

    #[test]
    fn caps_at_max_chunks() {
        let text = "x".repeat(100_000);
        let chunks = chunk_message(&text, 1000);
        assert!(chunks.len() <= MAX_CHUNKS);
        assert!(chunks.last().unwrap().contains("…(truncated)"));
    }

    #[test]
    fn suffix_format() {
        let text = "a".repeat(8000);
        let chunks = chunk_message(&text, 1000);
        assert!(chunks.len() > 1);
        let n = chunks.len();
        assert!(chunks[0].ends_with(&format!("(1/{})", n)));
        assert!(chunks.last().unwrap().contains(&format!("({}/{})", n, n)));
    }
}
