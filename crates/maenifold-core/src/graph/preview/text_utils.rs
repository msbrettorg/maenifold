// T-01.16: low-level char/byte index helpers extracted from graph/preview.rs.
// No behavioural changes.

pub(super) fn truncate_chars(content: &str, char_count: usize) -> &str {
    if char_count == 0 {
        return "";
    }

    match content.char_indices().nth(char_count) {
        Some((index, _)) => &content[..index],
        None => content,
    }
}

pub(super) fn find_last_sentence_boundary(text: &str) -> Option<usize> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut last_index = None;

    for (index, ch) in chars.iter().enumerate() {
        if !matches!(ch, '.' | '!' | '?') {
            continue;
        }

        let is_boundary = matches!(chars.get(index + 1), Some(' ' | '\n') | None);

        if is_boundary {
            last_index = Some(index);
        }
    }

    last_index
}

pub(super) fn find_next_sentence_boundary(text: &str) -> Option<usize> {
    let chars = text.chars().collect::<Vec<_>>();

    for (index, ch) in chars.iter().enumerate() {
        if !matches!(ch, '.' | '!' | '?') {
            continue;
        }

        let is_boundary = matches!(chars.get(index + 1), Some(' ' | '\n') | None);
        if is_boundary {
            return Some(index);
        }
    }

    None
}

pub(super) fn byte_to_char_index(text: &str, byte_index: usize) -> usize {
    text[..byte_index.min(text.len())].chars().count()
}

pub(super) fn char_to_byte_index(text: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }

    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

pub(super) fn slice_chars(text: &str, start_char: usize, end_char: usize) -> &str {
    let start_byte = char_to_byte_index(text, start_char);
    let end_byte = char_to_byte_index(text, end_char);
    &text[start_byte..end_byte]
}

pub(super) fn skip_leading_whitespace_chars(
    text: &str,
    start_char: usize,
    upper_bound: usize,
) -> usize {
    let mut index = start_char.min(upper_bound);
    let end = upper_bound.min(text.chars().count());
    while index < end {
        let next = slice_chars(text, index, index + 1);
        if !next.chars().all(char::is_whitespace) {
            break;
        }
        index += 1;
    }
    index
}
