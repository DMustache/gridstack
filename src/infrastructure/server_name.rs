use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

pub fn is_valid_server_name(value: &str) -> bool {
    parse_server_name(value).is_some()
}

pub fn parse_server_name(value: &str) -> Option<ServerNameParts<'_>> {
    let candidate = value.trim();
    if candidate.is_empty() {
        return None;
    }

    if candidate.starts_with('[') {
        let (host, port) = split_ipv6_literal_and_port(candidate)?;
        if !is_valid_ipv6_literal(host) {
            return None;
        }

        return Some(ServerNameParts { host, port });
    }

    let (host, port) = split_host_and_optional_port(candidate)?;
    if !(is_valid_ipv4_literal(host) || is_valid_dns_name(host)) {
        return None;
    }

    Some(ServerNameParts { host, port })
}

pub fn split_localpart_and_server_name(value: &str) -> Option<(&str, &str)> {
    let (localpart, server_name) = value.split_once(':')?;
    if localpart.is_empty() || !is_valid_server_name(server_name) {
        return None;
    }

    Some((localpart, server_name))
}

pub fn split_opaque_identifier_and_server_name(value: &str) -> Option<(&str, &str)> {
    for (index, character) in value.char_indices() {
        if character != ':' {
            continue;
        }

        let opaque_identifier = &value[..index];
        let server_name = &value[index + 1..];
        if opaque_identifier.is_empty() || !is_valid_server_name(server_name) {
            continue;
        }

        return Some((opaque_identifier, server_name));
    }

    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServerNameParts<'a> {
    pub host: &'a str,
    pub port: Option<&'a str>,
}

fn split_ipv6_literal_and_port(candidate: &str) -> Option<(&str, Option<&str>)> {
    let end_bracket_index = candidate.find(']')?;
    let host = &candidate[..=end_bracket_index];
    let remainder = &candidate[end_bracket_index + 1..];

    if remainder.is_empty() {
        return Some((host, None));
    }

    let port = remainder.strip_prefix(':')?;
    if !is_valid_port(port) {
        return None;
    }

    Some((host, Some(port)))
}

fn split_host_and_optional_port(candidate: &str) -> Option<(&str, Option<&str>)> {
    if let Some((host, port)) = candidate.rsplit_once(':') {
        if host.contains(':') {
            return None;
        }

        if !is_valid_port(port) {
            return None;
        }

        return Some((host, Some(port)));
    }

    Some((candidate, None))
}

fn is_valid_port(value: &str) -> bool {
    let length = value.len();
    if length == 0 || length > 5 {
        return false;
    }

    value.chars().all(|character| character.is_ascii_digit())
}

fn is_valid_ipv4_literal(value: &str) -> bool {
    Ipv4Addr::from_str(value).is_ok()
}

fn is_valid_ipv6_literal(value: &str) -> bool {
    let inner = value
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'));
    let Some(inner) = inner else {
        return false;
    };
    if inner.len() < 2 || inner.len() > 45 {
        return false;
    }

    Ipv6Addr::from_str(inner).is_ok()
}

fn is_valid_dns_name(value: &str) -> bool {
    if value.is_empty() || value.len() > 255 {
        return false;
    }
    if value.starts_with('.') || value.ends_with('.') {
        return false;
    }

    for label in value.split('.') {
        if label.is_empty() || label.len() > 63 {
            return false;
        }

        let label_bytes = label.as_bytes();
        let first = label_bytes[0] as char;
        let last = label_bytes[label_bytes.len() - 1] as char;
        if !is_ascii_alphanumeric(first) || !is_ascii_alphanumeric(last) {
            return false;
        }

        if !label
            .chars()
            .all(|character| is_ascii_alphanumeric(character) || character == '-')
        {
            return false;
        }
    }

    true
}

fn is_ascii_alphanumeric(character: char) -> bool {
    character.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::is_valid_server_name;

    #[test]
    fn accepts_matrix_spec_examples() {
        assert!(is_valid_server_name("matrix.org"));
        assert!(is_valid_server_name("matrix.org:8888"));
        assert!(is_valid_server_name("1.2.3.4"));
        assert!(is_valid_server_name("1.2.3.4:1234"));
        assert!(is_valid_server_name("[1234:5678::abcd]"));
        assert!(is_valid_server_name("[1234:5678::abcd]:5678"));
    }

    #[test]
    fn rejects_invalid_server_names() {
        assert!(!is_valid_server_name(""));
        assert!(!is_valid_server_name("matrix.org:"));
        assert!(!is_valid_server_name("matrix.org:abc"));
        assert!(!is_valid_server_name("matrix.org:123456"));
        assert!(!is_valid_server_name("1.2.3.999"));
        assert!(!is_valid_server_name("[1234:5678::abcd"));
        assert!(!is_valid_server_name("1234:5678::abcd"));
        assert!(!is_valid_server_name("-matrix.org"));
        assert!(!is_valid_server_name("matrix-.org"));
        assert!(!is_valid_server_name("matrix..org"));
    }
}
