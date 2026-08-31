pub fn is_authorized(header: Option<&str>, secret: &str) -> bool {
    match header.and_then(|v| v.strip_prefix("Bearer ")) {
        Some(token) => !secret.is_empty() && token == secret,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_matching_bearer() {
        assert!(is_authorized(Some("Bearer abc"), "abc"));
    }

    #[test]
    fn rejects_wrong_or_missing() {
        assert!(!is_authorized(Some("Bearer nope"), "abc"));
        assert!(!is_authorized(Some("abc"), "abc"));
        assert!(!is_authorized(None, "abc"));
        assert!(!is_authorized(Some("Bearer "), ""));
    }
}
