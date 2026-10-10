//! Tokenizable strings, e.g. `[LocalizedText Strings\Objects:Foo_Name]`.
//!
//! Resolution is delegated to a lookup closure so the content manager can
//! load string tables lazily.

/// Expand `[Token args...]` tokens. Unknown tokens are left as-is.
pub fn expand(text: &str, lookup: &mut dyn FnMut(&str, &str) -> Option<String>) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = find_close(after) else {
            out.push_str(&rest[start..]);
            return out;
        };
        let inner = &after[..end];
        out.push_str(&eval_token(inner, lookup).unwrap_or_else(|| format!("[{inner}]")));
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

fn find_close(s: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        match c {
            '[' => depth += 1,
            ']' if depth == 0 => return Some(i),
            ']' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn eval_token(inner: &str, lookup: &mut dyn FnMut(&str, &str) -> Option<String>) -> Option<String> {
    let inner = expand(inner, lookup);
    let mut parts = inner.split_whitespace();
    let name = parts.next()?;
    let args: Vec<&str> = parts.collect();
    match name {
        "LocalizedText" => {
            let key = args.first()?;
            let (asset, k) = key.split_once(':')?;
            let mut s = lookup(asset, k)?;
            // Positional substitutions {0}, {1}, ...
            for (i, a) in args.iter().skip(1).enumerate() {
                s = s.replace(&format!("{{{i}}}"), a);
            }
            Some(s)
        }
        "EscapedText" => Some(args.join(" ")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_localized_text() {
        let mut look = |asset: &str, key: &str| {
            if asset == "Strings\\Objects" && key == "Turnip_Name" {
                Some("Turnip {0}".to_string())
            } else {
                None
            }
        };
        assert_eq!(expand("[LocalizedText Strings\\Objects:Turnip_Name X]!", &mut look), "Turnip X!");
        assert_eq!(expand("plain [Unknown a] text", &mut look), "plain [Unknown a] text");
    }
}
