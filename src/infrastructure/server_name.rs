use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ServerNamePresentationFormat {
    #[default]
    Canonical,
    HostOnly,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ServerName {
    host: ServerHost,
    port: Option<u16>,
    homeserver_name: String,
}

impl ServerName {
    pub fn try_new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        let candidate = value.trim();
        let parts = parse_server_name(candidate)?;
        let host = parse_server_host(parts.host)?;
        let port = parts.port.map(str::parse).transpose().ok().flatten();

        Some(Self {
            homeserver_name: Self::format_server_name(&host, port),
            host,
            port,
        })
    }

    fn format_server_name(host: &ServerHost, port: Option<u16>) -> String {
        if port.is_none() {
            return host.to_string();
        }
        format!("{}:{}", host, port.unwrap())
    }

    pub fn as_str(&self) -> &str {
        &self.homeserver_name
    }

    pub const fn host(&self) -> &ServerHost {
        &self.host
    }

    pub const fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn present(&self, format: ServerNamePresentationFormat) -> String {
        match format {
            ServerNamePresentationFormat::Canonical => self.homeserver_name.clone(),
            ServerNamePresentationFormat::HostOnly => self.host.to_string(),
        }
    }
}

impl std::fmt::Display for ServerName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.homeserver_name)
    }
}

impl Serialize for ServerName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ServerName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).ok_or_else(|| serde::de::Error::custom("invalid server name"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ServerHost {
    DnsName(String),
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
}

impl std::fmt::Display for ServerHost {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DnsName(value) => write!(formatter, "{value}"),
            Self::Ipv4(value) => write!(formatter, "{value}"),
            Self::Ipv6(value) => write!(formatter, "[{value}]"),
        }
    }
}

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

fn parse_server_host(value: &str) -> Option<ServerHost> {
    if is_valid_ipv6_literal(value) {
        let inner = value.strip_prefix('[')?.strip_suffix(']')?;
        return Ipv6Addr::from_str(inner).ok().map(ServerHost::Ipv6);
    }
    if let Ok(ipv4) = Ipv4Addr::from_str(value) {
        return Some(ServerHost::Ipv4(ipv4));
    }
    is_valid_dns_name(value).then_some(ServerHost::DnsName(value.to_owned()))
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

const fn is_ascii_alphanumeric(character: char) -> bool {
    character.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::{ServerHost, ServerName, ServerNamePresentationFormat, is_valid_server_name};

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

    #[test]
    fn exposes_structured_host_and_port() {
        let server_name =
            ServerName::try_new("matrix.example.org:8448").expect("server name should be valid");
        assert_eq!(
            server_name.host(),
            &ServerHost::DnsName("matrix.example.org".to_owned())
        );
        assert_eq!(server_name.port(), Some(8448));
    }

    #[test]
    fn supports_presentation_format() {
        let server_name = ServerName::try_new("[2001:db8::1]:8448").expect("valid ipv6");
        assert_eq!(
            server_name.present(ServerNamePresentationFormat::Canonical),
            "[2001:db8::1]:8448"
        );
        assert_eq!(
            server_name.present(ServerNamePresentationFormat::HostOnly),
            "[2001:db8::1]"
        );
    }
}
