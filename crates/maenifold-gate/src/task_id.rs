use std::cmp::Ordering;

pub fn compare(left: &str, right: &str) -> Ordering {
    let mut left = left.chars().peekable();
    let mut right = right.chars().peekable();

    loop {
        match (left.peek().copied(), right.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left_char), Some(right_char))
                if left_char.is_ascii_digit() && right_char.is_ascii_digit() =>
            {
                let left_chunk = take_digit_chunk(&mut left);
                let right_chunk = take_digit_chunk(&mut right);
                let ordering = compare_digit_chunks(&left_chunk, &right_chunk);
                if !ordering.is_eq() {
                    return ordering;
                }
            }
            (Some(_), Some(_)) => {
                let ordering = left.next().cmp(&right.next());
                if !ordering.is_eq() {
                    return ordering;
                }
            }
        }
    }
}

fn take_digit_chunk<I>(chars: &mut std::iter::Peekable<I>) -> String
where
    I: Iterator<Item = char>,
{
    let mut chunk = String::new();
    while let Some(value) = chars.next_if(|value| value.is_ascii_digit()) {
        chunk.push(value);
    }
    chunk
}

fn compare_digit_chunks(left: &str, right: &str) -> Ordering {
    let left_trimmed = left.trim_start_matches('0');
    let right_trimmed = right.trim_start_matches('0');
    let left_normalized = if left_trimmed.is_empty() {
        "0"
    } else {
        left_trimmed
    };
    let right_normalized = if right_trimmed.is_empty() {
        "0"
    } else {
        right_trimmed
    };

    left_normalized
        .len()
        .cmp(&right_normalized.len())
        .then_with(|| left_normalized.cmp(right_normalized))
        .then_with(|| left.len().cmp(&right.len()))
}

#[cfg(test)]
mod tests {
    use super::compare;

    #[test]
    fn sorts_task_ids_naturally() {
        let mut task_ids = [
            "T-90.10".to_string(),
            "T-90.2".to_string(),
            "T-90.9".to_string(),
            "T-90.1".to_string(),
        ];
        task_ids.sort_by(|left, right| compare(left, right));
        assert_eq!(task_ids, ["T-90.1", "T-90.2", "T-90.9", "T-90.10"]);
    }

    #[test]
    fn non_task_ids_still_sort_deterministically() {
        let mut ids = ["alpha-10".to_string(), "alpha-2".to_string(), "alpha".to_string()];
        ids.sort_by(|left, right| compare(left, right));
        assert_eq!(ids, ["alpha", "alpha-2", "alpha-10"]);
    }
}
