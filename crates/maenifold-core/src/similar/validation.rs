pub(crate) fn normalize_find_similar_validation_input(input: &str) -> String {
    input
        .chars()
        .map(|character| match character {
            // T-QUAL-FSC2: RTM FR-7.4
            // Minimal NFKC-equivalent folding for the bracket confusable regression cases
            // covered by the C# oracle tests.
            '\u{FF3B}' => '[',
            '\u{FF3D}' => ']',
            _ => character,
        })
        .collect()
}

pub(crate) fn contains_bracket_like_delimiter(input: &str) -> bool {
    input.chars().any(|character| {
        matches!(
            character,
            '<' | '>'
                | '\u{300A}'
                | '\u{300B}'
                | '\u{3008}'
                | '\u{3009}'
                | '\u{3010}'
                | '\u{3011}'
                | '\u{300C}'
                | '\u{300D}'
                | '\u{300E}'
                | '\u{300F}'
                | '\u{3014}'
                | '\u{3015}'
                | '\u{3016}'
                | '\u{3017}'
                | '\u{27E6}'
                | '\u{27E7}'
                | '\u{27E8}'
                | '\u{27E9}'
                | '\u{27EA}'
                | '\u{27EB}'
                | '\u{27EC}'
                | '\u{27ED}'
                | '\u{27EE}'
                | '\u{27EF}'
                | '\u{2772}'
                | '\u{2773}'
                | '\u{276E}'
                | '\u{276F}'
        )
    })
}

pub(crate) fn is_low_information_find_similar_query(input: &str) -> bool {
    let mut has_non_ignored = false;

    for character in input.chars() {
        if character.is_whitespace() || character.is_control() {
            continue;
        }

        has_non_ignored = true;
        if character.is_alphanumeric() {
            return false;
        }
    }

    has_non_ignored
}
