//! Link syntax and credential destination policy for list pages.

use crate::error::Error;
use reqwest::header::{HeaderMap, LINK};
use url::Url;

fn invalid(message: &str) -> Error {
    Error::InvalidPagination(message.into())
}

/// Trust the configured origin, or the two public Webex REST aliases when
/// starting from either alias. Never trust a suffix match or arbitrary subdomain.
pub(super) fn validate_destination(origin: &Url, target: &Url) -> Result<(), Error> {
    let webex_alias = |url: &Url| {
        url.scheme() == "https"
            && url.port_or_known_default() == Some(443)
            && matches!(url.host_str(), Some("api.ciscospark.com" | "webexapis.com"))
    };
    if !target.username().is_empty()
        || target.password().is_some()
        || target.fragment().is_some()
        || !matches!(target.scheme(), "http" | "https")
        || !(target.origin() == origin.origin() || (webex_alias(origin) && webex_alias(target)))
    {
        return Err(invalid("destination is outside the trusted API origins"));
    }
    Ok(())
}

pub(super) fn resolve_destination(origin: &Url, base: &Url, reference: &str) -> Result<Url, Error> {
    // URL parsers normalize whitespace/backslashes; reject ambiguous input before
    // normalizing so malformed continuations cannot silently change destination.
    if reference
        .chars()
        .any(|c| c.is_ascii_whitespace() || c.is_control() || c == '\\')
    {
        return Err(invalid("invalid characters in destination"));
    }
    let target = base
        .join(reference)
        .map_err(|_| invalid("invalid destination URL"))?;
    validate_destination(origin, &target)?;
    Ok(target)
}

pub(super) fn next_link(
    headers: &HeaderMap,
    origin: &Url,
    base: &Url,
) -> Result<Option<Url>, Error> {
    let mut next = None;
    for value in headers.get_all(LINK) {
        let value = value
            .to_str()
            .map_err(|_| invalid("non-text Link header"))?;
        for reference in parse_next_links(value)? {
            let target = resolve_destination(origin, base, &reference)?;
            if next.as_ref().is_some_and(|previous| previous != &target) {
                return Err(invalid("conflicting next links"));
            }
            next = Some(target);
        }
    }
    Ok(next)
}

fn token(c: char) -> bool {
    c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c)
}

/// Parse RFC 8288 link-values, retaining URI references verbatim. In particular,
/// commas/semicolons inside quotes and angle brackets are not separators.
/// `parse_link_header` 0.4.1 was evaluated but strips whitespace and quotes before
/// splitting, losing relation lists and quoted delimiters (and rejecting bare queries).
fn parse_next_links(mut input: &str) -> Result<Vec<String>, Error> {
    let mut next = Vec::new();
    while !input.trim().is_empty() {
        input = input.trim_start_matches([' ', '\t', ',']);
        if input.is_empty() {
            break;
        }
        input = input
            .strip_prefix('<')
            .ok_or_else(|| invalid("expected '<' in Link header"))?;
        let end = input
            .find('>')
            .ok_or_else(|| invalid("unterminated Link URI"))?;
        let reference = &input[..end];
        input = &input[end + 1..];
        let mut rel = None;
        let mut anchored = false;
        loop {
            input = input.trim_start_matches([' ', '\t']);
            if input.is_empty() || input.starts_with(',') {
                break;
            }
            input = input
                .strip_prefix(';')
                .ok_or_else(|| invalid("expected Link parameter"))?
                .trim_start_matches([' ', '\t']);
            let name_end = input.find(|c| !token(c)).unwrap_or(input.len());
            if name_end == 0 {
                return Err(invalid("empty Link parameter name"));
            }
            let name = &input[..name_end];
            input = input[name_end..].trim_start_matches([' ', '\t']);
            let value = if let Some(rest) = input.strip_prefix('=') {
                input = rest.trim_start_matches([' ', '\t']);
                let (value, rest) = parameter_value(input)?;
                input = rest;
                Some(value)
            } else {
                None
            };
            if name.eq_ignore_ascii_case("rel") && rel.is_none() {
                rel = Some(value.ok_or_else(|| invalid("rel without a value"))?);
            } else if name.eq_ignore_ascii_case("anchor") {
                anchored = true;
            }
        }
        if rel.is_some_and(|value| {
            value
                .split_ascii_whitespace()
                .any(|r| r.eq_ignore_ascii_case("next"))
        }) {
            // An anchor changes link context. We cannot apply it to this response.
            if anchored {
                return Err(invalid("next link changes context with anchor"));
            }
            next.push(reference.to_owned());
        }
    }
    Ok(next)
}

fn parameter_value(input: &str) -> Result<(String, &str), Error> {
    if let Some(quoted) = input.strip_prefix('"') {
        let mut value = String::new();
        let mut escaped = false;
        for (offset, c) in quoted.char_indices() {
            if escaped {
                value.push(c);
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                return Ok((value, &quoted[offset + 1..]));
            } else if c.is_control() && c != '\t' {
                return Err(invalid("invalid quoted Link parameter"));
            } else {
                value.push(c);
            }
        }
        return Err(invalid("unterminated quoted Link parameter"));
    }
    let end = input.find(|c| !token(c)).unwrap_or(input.len());
    if end == 0 {
        return Err(invalid("empty Link parameter value"));
    }
    Ok((input[..end].to_owned(), &input[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_parameters_and_relation_lists() {
        for header in [
            r#"</rooms?cursor=a,b;c&before>; title="a,b; rel=\"prev\""; rel="next last""#,
            r#"</rooms?cursor=a,b;c&before>; REL = "last next"; title="a,b; \"x\"""#,
            r"</rooms?cursor=a,b;c&before>; rel=next",
        ] {
            assert_eq!(
                parse_next_links(header).unwrap(),
                ["/rooms?cursor=a,b;c&before"]
            );
        }
        assert_eq!(
            parse_next_links(r#"</a>; rel="prev", </b>; rel=last"#).unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(
            parse_next_links(r"</a>; rel=next; rel=prev").unwrap(),
            ["/a"]
        );
    }

    #[test]
    fn malformed_links_are_errors() {
        for header in [
            "not a link",
            "<broken",
            "</a>; rel=",
            "</a>; rel=\"next",
            "</a>; rel=next; anchor=\"/b\"",
        ] {
            assert!(
                matches!(parse_next_links(header), Err(Error::InvalidPagination(_))),
                "{header}"
            );
        }
    }

    #[test]
    fn only_explicit_trusted_origins_are_allowed() {
        let origin = Url::parse("https://api.ciscospark.com/v1/rooms").unwrap();
        for target in [
            "https://api.ciscospark.com/v1/rooms?cursor=x",
            "https://webexapis.com/v1/rooms?cursor=x",
        ] {
            assert!(validate_destination(&origin, &Url::parse(target).unwrap()).is_ok());
        }
        for target in [
            "https://webexapis.com.evil.example/v1/rooms",
            "https://evil.webexapis.com/v1/rooms",
            "http://api.ciscospark.com:443/v1/rooms",
            "https://webexapis.com:444/v1/rooms",
            "https://user:password@webexapis.com/v1/rooms",
            "https://webexapis.com/v1/rooms#fragment",
            "file:///etc/passwd",
            "https://example.org/v1/rooms",
        ] {
            assert!(
                validate_destination(&origin, &Url::parse(target).unwrap()).is_err(),
                "{target}"
            );
        }
        let local = Url::parse("http://127.0.0.1:1234/rooms").unwrap();
        assert!(validate_destination(&local, &local).is_ok());
        assert!(validate_destination(&local, &origin).is_err());
        assert!(
            validate_destination(&local, &Url::parse("http://127.0.0.1:5678/rooms").unwrap())
                .is_err()
        );
    }
}
