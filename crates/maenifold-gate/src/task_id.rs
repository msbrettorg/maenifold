use std::cmp::Ordering;

pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();
    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(&ac), Some(&bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    let a_num: String = a_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    let b_num: String = b_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    let a_val: u64 = a_num.parse().unwrap_or(0);
                    let b_val: u64 = b_num.parse().unwrap_or(0);
                    match a_val.cmp(&b_val) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    a_chars.next();
                    b_chars.next();
                    match ac.cmp(&bc) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }
            }
        }
    }
}

pub fn compare(left: &str, right: &str) -> Ordering {
    natural_cmp(left, right)
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::{compare, natural_cmp};

    #[test]
    fn natural_sort_orders_numerically() {
        assert_eq!(natural_cmp("T-90.2", "T-90.10"), Ordering::Less);
        assert_eq!(natural_cmp("T-90.9", "T-90.10"), Ordering::Less);
        assert_eq!(natural_cmp("T-90.10", "T-90.2"), Ordering::Greater);
        assert_eq!(natural_cmp("T-90.10", "T-90.10"), Ordering::Equal);
    }

    #[test]
    fn sorts_task_ids_naturally() {
        let mut task_ids = [
            "T-90.10".to_string(),
            "T-90.2".to_string(),
            "T-90.9".to_string(),
            "T-90.1".to_string(),
        ];
        task_ids.sort_by(|left, right| natural_cmp(left, right));
        assert_eq!(task_ids, ["T-90.1", "T-90.2", "T-90.9", "T-90.10"]);
    }

    #[test]
    fn non_task_ids_still_sort_deterministically() {
        let mut ids = [
            "alpha-10".to_string(),
            "alpha-2".to_string(),
            "alpha".to_string(),
        ];
        ids.sort_by(|left, right| compare(left, right));
        assert_eq!(ids, ["alpha", "alpha-2", "alpha-10"]);
    }
}
