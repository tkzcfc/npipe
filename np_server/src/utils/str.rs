pub fn is_ascii_nospace(s: &str) -> bool {
    s.is_ascii() && !s.contains(' ')
}

pub fn is_valid_username(s: &str) -> bool {
    s.len() >= 5 && s.len() <= 30 && is_ascii_nospace(s)
}

pub fn is_valid_password(s: &str) -> bool {
    s.len() >= 5 && s.len() <= 15 && is_ascii_nospace(s)
}
