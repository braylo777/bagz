//! Text helpers shared by the parsers, dedupe, ranking and agents.

use std::collections::BTreeSet;

/// Longest description kept per posting, in characters.
pub const DESCRIPTION_CAP: usize = 4000;

/// Decodes the HTML entities the feeds use: the five XML ones, `&nbsp;`, a
/// few typographic names, and numeric `&#NN;` / `&#xNN;` forms. Unknown
/// entities stay as written.
pub fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(pos) = rest.find('&') {
        out.push_str(&rest[..pos]);
        let tail = &rest[pos..];
        let end = tail[1..]
            .find(|c: char| c == ';' || c == '&' || c.is_whitespace())
            .map(|i| i + 1);
        match end {
            Some(e) if tail.as_bytes()[e] == b';' => {
                let name = &tail[1..e];
                if let Some(decoded) = entity(name) {
                    out.push(decoded);
                    rest = &tail[e + 1..];
                    continue;
                }
                out.push('&');
                rest = &tail[1..];
            }
            _ => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn entity(name: &str) -> Option<char> {
    if let Some(num) = name.strip_prefix('#') {
        let code = if let Some(hex) = num.strip_prefix('x').or_else(|| num.strip_prefix('X')) {
            u32::from_str_radix(hex, 16).ok()?
        } else {
            num.parse::<u32>().ok()?
        };
        return char::from_u32(code);
    }
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => ' ',
        "ndash" => '–',
        "mdash" => '—',
        "rsquo" => '’',
        "lsquo" => '‘',
        "rdquo" => '”',
        "ldquo" => '“',
        "hellip" => '…',
        "bull" => '•',
        "middot" => '·',
        _ => return None,
    })
}

/// Turns HTML (also entity-escaped HTML, as Greenhouse sends it) into plain
/// text with single spaces. Block tags become line breaks.
pub fn html_to_text(html: &str) -> String {
    let decoded = decode_entities(html);
    let mut out = String::with_capacity(decoded.len());
    let mut in_tag = false;
    let mut tag = String::new();
    for c in decoded.chars() {
        match (in_tag, c) {
            (false, '<') => {
                in_tag = true;
                tag.clear();
            }
            (true, '>') => {
                in_tag = false;
                let name = tag
                    .trim_start_matches('/')
                    .split(|c: char| c.is_whitespace() || c == '/')
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if matches!(
                    name.as_str(),
                    "p" | "br"
                        | "li"
                        | "div"
                        | "h1"
                        | "h2"
                        | "h3"
                        | "h4"
                        | "h5"
                        | "h6"
                        | "ul"
                        | "ol"
                        | "tr"
                ) {
                    out.push('\n');
                } else {
                    out.push(' ');
                }
            }
            (true, _) => tag.push(c),
            (false, _) => out.push(c),
        }
    }
    let text = decode_entities(&out);
    let lines: Vec<String> = text
        .lines()
        .map(collapse_spaces)
        .filter(|l| !l.is_empty())
        .collect();
    lines.join("\n")
}

/// Collapses every run of whitespace into one space and trims the ends.
pub fn collapse_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Cuts `s` to at most `max` characters, on a character boundary.
pub fn truncate_chars(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((i, _)) => s[..i].to_string(),
        None => s.to_string(),
    }
}

/// Lowercase ASCII words, digits kept, everything else a separator.
/// `C++`, `C#` and `.NET` keep their symbols so they stay matchable.
pub fn tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() || c == '+' || c == '#' {
            cur.extend(c.to_lowercase());
        } else {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// The set of distinct tokens in `s`.
pub fn token_set(s: &str) -> BTreeSet<String> {
    tokens(s).into_iter().collect()
}

const COMPANY_SUFFIXES: &[&str] = &[
    "inc",
    "llc",
    "ltd",
    "limited",
    "corp",
    "corporation",
    "co",
    "gmbh",
    "sa",
    "ag",
    "plc",
    "bv",
    "technologies",
    "technology",
    "labs",
    "hq",
];

/// Comparison key for a company name: tokens, legal suffixes dropped.
pub fn company_key(name: &str) -> String {
    let toks: Vec<String> = tokens(name)
        .into_iter()
        .filter(|t| !COMPANY_SUFFIXES.contains(&t.as_str()))
        .collect();
    if toks.is_empty() {
        tokens(name).join(" ")
    } else {
        toks.join(" ")
    }
}

/// Comparison key for a title: tokens, with common abbreviations expanded.
pub fn title_key(title: &str) -> String {
    tokens(title)
        .into_iter()
        .map(|t| match t.as_str() {
            "sr" => "senior".to_string(),
            "jr" => "junior".to_string(),
            "eng" | "engr" => "engineer".to_string(),
            "mgr" => "manager".to_string(),
            "swe" => "software engineer".to_string(),
            _ => t,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Comparison key for a location: the first place named, as tokens.
pub fn location_key(loc: &str) -> String {
    let first = loc.split(['•', ';', '|', '/']).next().unwrap_or("").trim();
    tokens(first).join(" ")
}

/// Days since 1970-01-01 to a civil (year, month, day).
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// A civil date to days since 1970-01-01.
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `YYYY-MM-DD` for a Unix time in milliseconds.
pub fn date_from_epoch_ms(ms: i64) -> String {
    let (y, m, d) = civil_from_days(ms.div_euclid(86_400_000));
    format!("{y:04}-{m:02}-{d:02}")
}

/// Days since 1970-01-01 for a `YYYY-MM-DD...` string.
pub fn days_from_iso(s: &str) -> Option<i64> {
    let b = s.get(..10)?;
    let mut parts = b.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// Takes the `YYYY-MM-DD` part of an ISO-8601 or RFC-3339 timestamp.
pub fn iso_date(s: &str) -> Option<String> {
    days_from_iso(s.trim())?;
    Some(s.trim()[..10].to_string())
}

/// Reads an RFC-822 date as RSS uses it (`Tue, 18 Nov 2026 09:00:00 +0000`).
pub fn rfc822_date(s: &str) -> Option<String> {
    let s = s.trim();
    let s = s.split_once(',').map(|(_, r)| r.trim()).unwrap_or(s);
    let mut parts = s.split_whitespace();
    let d: u32 = parts.next()?.parse().ok()?;
    let mon = parts.next()?.get(..3)?.to_ascii_lowercase();
    let m = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ]
    .iter()
    .position(|x| *x == mon)? as u32
        + 1;
    let y: i64 = parts.next()?.parse().ok()?;
    if !(1..=31).contains(&d) {
        return None;
    }
    Some(format!("{y:04}-{m:02}-{d:02}"))
}

/// Today's UTC date as days since 1970-01-01.
pub fn today_days() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    secs.div_euclid(86_400)
}

/// Today's UTC date as `YYYY-MM-DD`.
pub fn today_iso() -> String {
    let (y, m, d) = civil_from_days(today_days());
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_named_and_numeric_entities() {
        assert_eq!(decode_entities("a &amp; b &lt;p&gt;"), "a & b <p>");
        assert_eq!(decode_entities("&#65;&#x42;&quot;"), "AB\"");
        assert_eq!(decode_entities("caf&eacute;"), "caf&eacute;");
        assert_eq!(decode_entities("AT&T rocks"), "AT&T rocks");
        assert_eq!(decode_entities("trailing &"), "trailing &");
    }

    #[test]
    fn html_to_text_handles_escaped_html() {
        let html = "&lt;div&gt;&lt;p&gt;Hello &amp;amp; welcome&lt;/p&gt;&lt;ul&gt;&lt;li&gt;Rust&lt;/li&gt;&lt;/ul&gt;&lt;/div&gt;";
        assert_eq!(html_to_text(html), "Hello & welcome\nRust");
    }

    #[test]
    fn html_to_text_inline_tags_become_spaces() {
        assert_eq!(html_to_text("<b>Bold</b>text<br/>next"), "Bold text\nnext");
    }

    #[test]
    fn truncate_is_char_safe() {
        assert_eq!(truncate_chars("héllo", 2), "hé");
        assert_eq!(truncate_chars("hi", 10), "hi");
    }

    #[test]
    fn tokens_keep_language_symbols() {
        assert_eq!(tokens("C++ / C# & Go!"), vec!["c++", "c#", "go"]);
    }

    #[test]
    fn company_key_drops_suffixes() {
        assert_eq!(company_key("Acme, Inc."), "acme");
        assert_eq!(company_key("Acme Labs LLC"), "acme");
        assert_eq!(company_key("Inc"), "inc");
    }

    #[test]
    fn title_key_expands_abbreviations() {
        assert_eq!(title_key("Sr. Software Eng"), "senior software engineer");
    }

    #[test]
    fn location_key_takes_first_place() {
        assert_eq!(
            location_key("San Francisco, CA • New York, NY"),
            "san francisco ca"
        );
    }

    #[test]
    fn epoch_ms_to_date() {
        assert_eq!(date_from_epoch_ms(0), "1970-01-01");
        assert_eq!(date_from_epoch_ms(1_792_767_640_768), "2026-10-23");
    }

    #[test]
    fn civil_round_trip_known_dates() {
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(civil_from_days(11_017), (2000, 3, 1));
    }

    #[test]
    fn iso_and_rfc822_dates() {
        assert_eq!(
            iso_date("2026-10-23T19:16:55.670+00:00").as_deref(),
            Some("2026-10-23")
        );
        assert_eq!(iso_date("not a date"), None);
        assert_eq!(iso_date("2026-13-01"), None);
        assert_eq!(
            rfc822_date("Tue, 18 Nov 2026 09:00:00 +0000").as_deref(),
            Some("2026-11-18")
        );
        assert_eq!(rfc822_date("18 Nov 2026"), Some("2026-11-18".into()));
        assert_eq!(rfc822_date("Tue, 40 Nov 2026"), None);
        assert_eq!(rfc822_date("garbage"), None);
    }
}
