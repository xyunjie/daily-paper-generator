pub fn compact_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn looks_like_jira_key(s: &str) -> bool {
    // e.g. ABC-123
    let bytes = s.as_bytes();
    if bytes.len() < 5 {
        return false;
    }
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_uppercase() {
        i += 1;
    }
    if i < 2 || i + 1 >= bytes.len() {
        return false;
    }
    if bytes[i] != b'-' {
        return false;
    }
    i += 1;
    let start_digits = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    i == bytes.len() && (i - start_digits) >= 1
}

pub fn looks_like_hex_hash(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 7 {
        return false;
    }
    t.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn contains_forbidden_markers(line: &str) -> bool {
    if line.contains("http://") || line.contains("https://") {
        return true;
    }
    // Only flag word/word style paths (slash with non-whitespace on both sides)
    {
        let bytes = line.as_bytes();
        for i in 1..bytes.len() {
            if bytes[i] == b'/' && bytes[i - 1] != b' ' && i + 1 < bytes.len() && bytes[i + 1] != b' ' {
                return true;
            }
        }
    }
    for token in line
        .split(|c: char| c.is_whitespace() || c == '\u{FF0C}' || c == ',' || c == ';' || c == '\u{FF1B}')
        .filter(|t| !t.is_empty())
    {
        if looks_like_jira_key(token) {
            return true;
        }
        if looks_like_hex_hash(token) {
            return true;
        }
    }
    false
}
