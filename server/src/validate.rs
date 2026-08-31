pub fn normalize_code(raw: &str) -> Option<String> {
    let digits: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if (6..=8).contains(&digits.len()) && digits.chars().all(|c| c.is_ascii_digit()) {
        Some(digits)
    } else {
        None
    }
}

pub fn valid_account(account: &str) -> bool {
    !account.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_spaced_code() {
        assert_eq!(normalize_code("123 456").as_deref(), Some("123456"));
    }

    #[test]
    fn rejects_bad_codes() {
        assert!(normalize_code("12345").is_none()); // too short
        assert!(normalize_code("123456789").is_none()); // too long
        assert!(normalize_code("12a456").is_none()); // non-digit
    }

    #[test]
    fn account_must_be_non_empty() {
        assert!(valid_account("Corp"));
        assert!(!valid_account("   "));
    }
}
