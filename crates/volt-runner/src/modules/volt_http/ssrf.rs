use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};

/// Resolve a hostname to socket addresses, validate all IPs against the
/// SSRF blocklist, and return the first valid address for DNS pinning.
pub(super) fn resolve_and_validate_host(
    host: &str,
    port: u16,
    allow_private: bool,
) -> Result<SocketAddr, String> {
    let addrs: Vec<SocketAddr> = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("failed to resolve host '{host}': {e}"))?
        .collect();

    if addrs.is_empty() {
        return Err(format!("host '{host}' did not resolve to any address"));
    }

    if !allow_private {
        for addr in &addrs {
            if is_forbidden_outbound_ip(addr.ip()) {
                return Err("local and private network targets are not allowed".to_string());
            }
        }
    }

    Ok(addrs[0])
}

pub(super) fn normalize_request_url(
    url: &str,
    allow_private_networks: bool,
) -> Result<String, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("http.fetch request URL must not be empty".to_string());
    }

    let parsed =
        reqwest::Url::parse(trimmed).map_err(|error| format!("invalid request URL: {error}"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(format!(
                "unsupported request URL scheme '{scheme}'; only http and https are allowed"
            ));
        }
    }

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("embedded credentials are not allowed in request URLs".to_string());
    }

    validate_request_host(&parsed, allow_private_networks)?;
    Ok(parsed.to_string())
}

fn validate_request_host(url: &reqwest::Url, allow_private_networks: bool) -> Result<(), String> {
    if allow_private_networks {
        return Ok(());
    }

    let host = url
        .host_str()
        .ok_or_else(|| "request URL must include a host".to_string())?;
    if host.eq_ignore_ascii_case("localhost") {
        return Err("local and private network targets are not allowed".to_string());
    }

    let port = url
        .port_or_known_default()
        .ok_or_else(|| "request URL must include a valid port".to_string())?;
    let mut resolved_any = false;
    for address in (host, port)
        .to_socket_addrs()
        .map_err(|error| format!("failed to resolve request host '{host}': {error}"))?
    {
        resolved_any = true;
        if is_forbidden_outbound_ip(address.ip()) {
            return Err("local and private network targets are not allowed".to_string());
        }
    }

    if !resolved_any {
        return Err(format!(
            "request host '{host}' did not resolve to an address"
        ));
    }

    Ok(())
}

pub(super) fn is_forbidden_outbound_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_forbidden_ipv4(address),
        IpAddr::V6(address) => is_forbidden_ipv6(address),
    }
}

fn is_forbidden_ipv4(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    address.is_private()
        || address.is_loopback()
        || address.is_link_local()
        || address.is_broadcast()
        || address.is_documentation()
        || address.is_multicast()
        || address.is_unspecified()
        || octets[0] == 0
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
        || (octets[0] == 198 && (octets[1] == 18 || octets[1] == 19))
}

fn is_forbidden_ipv6(address: Ipv6Addr) -> bool {
    let segments = address.segments();
    address.is_loopback()
        || address.is_unique_local()
        || address.is_unicast_link_local()
        || address.is_multicast()
        || address.is_unspecified()
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        || (segments[0] & 0xffc0) == 0xfec0
}
