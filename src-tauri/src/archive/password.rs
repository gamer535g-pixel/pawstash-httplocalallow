const MAX_CANDIDATES: usize = 10;
const MAX_PASSWORD_CHARS: usize = 128;
const LABEL_LOOKAHEAD_CHARS: usize = 32;
const LABEL_LOOKAHEAD_WORDS: usize = 3;

const KEYWORDS: &[(&str, bool)] = &[
    ("password", true),
    ("passwd", true),
    ("pass", true),
    ("pwd", true),
    ("pw", true),
    ("пароль", true),
    ("解压码", false),
    ("密码", false),
    ("パスワード", false),
    ("비밀번호", false),
];

const STOPWORDS: &[&str] = &[
    "a",
    "above",
    "an",
    "and",
    "are",
    "as",
    "at",
    "be",
    "below",
    "by",
    "check",
    "comment",
    "comments",
    "description",
    "dm",
    "for",
    "from",
    "here",
    "in",
    "included",
    "inside",
    "is",
    "it",
    "message",
    "my",
    "n/a",
    "needed",
    "no",
    "none",
    "not",
    "of",
    "on",
    "or",
    "our",
    "please",
    "post",
    "protected",
    "required",
    "same",
    "see",
    "that",
    "the",
    "this",
    "to",
    "which",
    "will",
    "with",
    "yet",
    "you",
    "your",
];

pub fn extract_password_candidates(post_text: &str) -> Vec<String> {
    let text = html_text(post_text);
    let mut candidates: Vec<String> = Vec::new();
    let mut cursor = 0;
    while cursor < text.len() && candidates.len() < MAX_CANDIDATES {
        let rest = &text[cursor..];
        let Some(first) = rest.chars().next() else {
            break;
        };
        let at_word_start = text[..cursor]
            .chars()
            .next_back()
            .is_none_or(|previous| !previous.is_alphanumeric());
        let Some(keyword_len) = match_keyword(rest, at_word_start) else {
            cursor += first.len_utf8();
            continue;
        };
        let after_keyword = cursor + keyword_len;
        match parse_value(&text[after_keyword..]) {
            Some((value, consumed)) => {
                if !candidates.contains(&value) {
                    candidates.push(value);
                }
                cursor = after_keyword + consumed;
            }
            None => cursor = after_keyword,
        }
    }
    candidates
}

fn html_text(input: &str) -> String {
    if !input.contains('<') && !input.contains('&') {
        return input.to_string();
    }
    let fragment = scraper::Html::parse_fragment(input);
    fragment.root_element().text().collect::<Vec<_>>().join(" ")
}

fn match_keyword(rest: &str, at_word_start: bool) -> Option<usize> {
    for (keyword, needs_boundary) in KEYWORDS {
        if *needs_boundary && !at_word_start {
            continue;
        }
        let Some(len) = prefix_len_ignore_case(rest, keyword) else {
            continue;
        };
        let next = rest[len..].chars().next();
        if *needs_boundary && next.is_some_and(char::is_alphanumeric) {
            continue;
        }
        return Some(len);
    }
    None
}

fn prefix_len_ignore_case(text: &str, prefix: &str) -> Option<usize> {
    let mut consumed = 0;
    let mut chars = text.chars();
    for expected in prefix.chars() {
        let actual = chars.next()?;
        if actual.to_lowercase().next() != Some(expected) {
            return None;
        }
        consumed += actual.len_utf8();
    }
    Some(consumed)
}

fn is_inline_space(c: char) -> bool {
    c.is_whitespace() && c != '\n' && c != '\r'
}

fn skip_whitespace(text: &str) -> &str {
    text.trim_start_matches(char::is_whitespace)
}

fn parse_value(text: &str) -> Option<(String, usize)> {
    let total = text.len();
    let spaced = text.starts_with(char::is_whitespace);
    let mut rest = skip_whitespace(text);
    let mut labelled = false;

    if let Some(len) = prefix_len_ignore_case(rest, "is") {
        if rest[len..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric())
        {
            rest = skip_whitespace(&rest[len..]);
            labelled = true;
        }
    }
    if let Some(separator) = rest.chars().next() {
        let after = &rest[separator.len_utf8()..];
        let is_separator = match separator {
            ':' | '：' | '=' => true,
            '-' | '–' | '—' => after.chars().next().is_some_and(char::is_whitespace),
            _ => false,
        };
        if is_separator {
            rest = skip_whitespace(after);
            labelled = true;
        }
    }
    if !labelled {
        if let Some(after) = colon_after_label_words(rest) {
            rest = skip_whitespace(after);
            labelled = true;
        }
    }
    if !labelled && !spaced {
        return None;
    }

    let first = rest.chars().next()?;
    if let Some(close) = closing_quote(first) {
        let inner = &rest[first.len_utf8()..];
        let end = inner.find(close)?;
        let value = inner[..end].trim();
        let consumed = total - inner.len() + end + close.len_utf8();
        return accept(value, true).map(|value| (value, consumed));
    }

    let end = rest
        .find(|c: char| c.is_whitespace() || c == '<' || is_cjk_punctuation(c))
        .unwrap_or(rest.len());
    let consumed = total - rest.len() + end;
    let token = rest[..end].trim_end_matches(|c: char| {
        matches!(
            c,
            '.' | ',' | ';' | ':' | ')' | ']' | '}' | '"' | '\'' | '”' | '’' | '»'
        )
    });
    accept(token, labelled).map(|value| (value, consumed))
}

fn colon_after_label_words(text: &str) -> Option<&str> {
    let mut words = 0;
    let mut in_word = false;
    for (offset, c) in text.char_indices().take(LABEL_LOOKAHEAD_CHARS) {
        match c {
            ':' | '：' => return (words > 0).then(|| &text[offset + c.len_utf8()..]),
            c if c.is_alphabetic() => {
                if !in_word {
                    words += 1;
                    if words > LABEL_LOOKAHEAD_WORDS {
                        return None;
                    }
                }
                in_word = true;
            }
            c if is_inline_space(c) => in_word = false,
            _ => return None,
        }
    }
    None
}

fn closing_quote(open: char) -> Option<char> {
    match open {
        '"' => Some('"'),
        '\'' => Some('\''),
        '`' => Some('`'),
        '“' => Some('”'),
        '‘' => Some('’'),
        '«' => Some('»'),
        '「' => Some('」'),
        '『' => Some('』'),
        _ => None,
    }
}

fn is_cjk_punctuation(c: char) -> bool {
    matches!(
        c,
        '，' | '。'
            | '、'
            | '；'
            | '（'
            | '）'
            | '「'
            | '」'
            | '『'
            | '』'
            | '【'
            | '】'
            | '！'
            | '？'
    )
}

fn accept(token: &str, labelled: bool) -> Option<String> {
    let length = token.chars().count();
    if !(2..=MAX_PASSWORD_CHARS).contains(&length) {
        return None;
    }
    let lower = token.to_lowercase();
    if STOPWORDS.contains(&lower.as_str())
        || KEYWORDS.iter().any(|(keyword, _)| *keyword == lower)
        || lower.starts_with("http://")
        || lower.starts_with("https://")
    {
        return None;
    }
    if !labelled && token.chars().all(char::is_alphabetic) {
        return None;
    }
    Some(token.to_string())
}

#[cfg(test)]
mod tests {
    use super::extract_password_candidates as extract;

    #[test]
    fn finds_labelled_passwords() {
        for (text, expected) in [
            ("Password: lewdk@t77", vec!["lewdk@t77"]),
            ("pass : 9adn2", vec!["9adn2"]),
            (
                "Password  44d2cc8d5fa0fa325942d1180471f11e",
                vec!["44d2cc8d5fa0fa325942d1180471f11e"],
            ),
            ("PW=abc123.", vec!["abc123"]),
            ("pwd - hunter", vec!["hunter"]),
            ("The password is \"open sesame\"", vec!["open sesame"]),
            ("Password for zip: q1w2", vec!["q1w2"]),
            ("Password: pass123", vec!["pass123"]),
            (
                "<p>Hi!</p><p>PW: <strong>abc123</strong></p><p>Enjoy</p>",
                vec!["abc123"],
            ),
            ("Pass&nbsp;:&nbsp;k9&amp;z", vec!["k9&z"]),
            ("Пароль: котик2024", vec!["котик2024"]),
            ("密码：1234，解压码：5678", vec!["1234", "5678"]),
            ("パスワード：hoge123", vec!["hoge123"]),
            ("비밀번호: qwer1234", vec!["qwer1234"]),
            (
                "pass: abc1 then password: zz9 and again pw: abc1",
                vec!["abc1", "zz9"],
            ),
        ] {
            assert_eq!(extract(text), expected, "{text}");
        }
    }

    #[test]
    fn rejects_prose() {
        for text in [
            "my passion for art",
            "I passed the exam",
            "Password on your Fan-box account",
            "a password which refers to the old post",
            "the password is required",
            "pass-through cable",
            "compass: north",
        ] {
            assert!(extract(text).is_empty(), "{text}");
        }
    }

    #[test]
    fn caps_candidates() {
        let many = (0..20)
            .map(|i| format!("pw: key{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(extract(&many).len(), 10);
    }
}
