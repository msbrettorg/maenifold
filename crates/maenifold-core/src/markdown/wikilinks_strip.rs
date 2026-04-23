//! T-01.17: bracket-stripping logic for [[WikiLinks]] extracted from
//! [`super::wikilinks`] to keep the parent file under the per-file LOC budget.
//! No behavioral changes.

pub fn strip_wikilink_brackets(text: &str) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let mut stripped = String::with_capacity(text.len());
    let mut index = 0usize;

    while index < chars.len() {
        if chars[index] == '[' && chars.get(index + 1) == Some(&'[') {
            let mut cursor = index + 2;
            let mut closing = None;
            let mut invalid = false;

            while cursor + 1 < chars.len() {
                match (chars[cursor], chars[cursor + 1]) {
                    ('[', '[') => {
                        invalid = true;
                        break;
                    }
                    (']', ']') => {
                        closing = Some(cursor);
                        break;
                    }
                    ('[', _) | (']', _) => {
                        invalid = true;
                        break;
                    }
                    _ => cursor += 1,
                }
            }

            if invalid {
                if let Some(end) = find_next_closing_wikilink(&chars, cursor + 1) {
                    stripped.extend(chars[index..end].iter());
                    index = end;
                    continue;
                }

                stripped.extend(chars[index..].iter());
                break;
            }

            if let Some(end) = closing {
                let inner = chars[index + 2..end].iter().collect::<String>();
                let visible = inner
                    .split_once('|')
                    .map_or(inner.as_str(), |(_, display)| display)
                    .trim();
                append_with_boundary_spacing(&mut stripped, visible);
                if needs_boundary_space(stripped.chars().last(), chars.get(end + 2).copied()) {
                    stripped.push(' ');
                }
                index = end + 2;
                continue;
            }

            stripped.extend(chars[index..].iter());
            break;
        }

        stripped.push(chars[index]);
        index += 1;
    }

    stripped
}

fn append_with_boundary_spacing(stripped: &mut String, visible: &str) {
    if visible.is_empty() {
        return;
    }

    if needs_boundary_space(stripped.chars().last(), visible.chars().next()) {
        stripped.push(' ');
    }
    stripped.push_str(visible);
}

fn needs_boundary_space(left: Option<char>, right: Option<char>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if is_word_character(left) && is_word_character(right))
}

fn is_word_character(ch: char) -> bool {
    ch.is_alphanumeric()
}

fn find_next_closing_wikilink(chars: &[char], start: usize) -> Option<usize> {
    let mut cursor = start;
    while cursor + 1 < chars.len() {
        if chars[cursor] == ']' && chars[cursor + 1] == ']' {
            return Some(cursor + 2);
        }
        cursor += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::strip_wikilink_brackets;

    #[test]
    fn strips_basic_wikilinks() {
        assert_eq!(
            strip_wikilink_brackets("See [[concept]] today"),
            "See concept today"
        );
    }

    #[test]
    fn keeps_alias_display_text() {
        assert_eq!(
            strip_wikilink_brackets("Open [[target|display text]] now"),
            "Open display text now"
        );
    }

    #[test]
    fn inserts_spacing_to_avoid_token_merging() {
        assert_eq!(
            strip_wikilink_brackets("foo[[bar]]baz and [[zip]]zap"),
            "foo bar baz and zip zap"
        );
    }

    #[test]
    fn separates_adjacent_links_without_padding_punctuation() {
        assert_eq!(strip_wikilink_brackets("[[a]][[b]]"), "a b");
        assert_eq!(strip_wikilink_brackets("([[a]])"), "(a)");
    }

    #[test]
    fn leaves_malformed_nested_links_unchanged() {
        assert_eq!(strip_wikilink_brackets("[[foo[[bar]]"), "[[foo[[bar]]");
        assert_eq!(strip_wikilink_brackets("[[foo[[bar]]]]"), "[[foo[[bar]]]]");
    }
}
