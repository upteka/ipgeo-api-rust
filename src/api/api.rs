use axum::{
    extract::{Path, Query, ConnectInfo},
    routing::get,
    Router,
    Json,
    http::{StatusCode, HeaderMap, HeaderName},
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use crate::geo::{get_ip_info, resolve_host};
use crate::utils::is_private_ip;
use tracing::{debug, info};
use once_cell::sync::Lazy;

// 使用静态HeaderName避免重复解析
static CDN_HEADERS: Lazy<[(HeaderName, &'static str); 8]> = Lazy::new(|| [
    (HeaderName::from_static("cf-connecting-ip"), "Cloudflare"),
    (HeaderName::from_static("fastly-client-ip"), "Fastly"),
    (HeaderName::from_static("x-azure-clientip"), "Azure"),
    (HeaderName::from_static("x-akamai-client-ip"), "Akamai"),
    (HeaderName::from_static("true-client-ip"), "Akamai"),
    (HeaderName::from_static("x-cdn-src-ip"), "CDN"),
    (HeaderName::from_static("x-real-ip"), "General"),
    (HeaderName::from_static("x-forwarded-for"), "General"),
]);

static FORWARDED_HEADER: Lazy<HeaderName> = Lazy::new(|| HeaderName::from_static("forwarded"));

#[inline]
pub fn get_real_ip(headers: &HeaderMap, socket_addr: SocketAddr) -> IpAddr {
    let socket_ip = socket_addr.ip();

    // 1. 优先检查cdn的头部
    for (header, provider) in CDN_HEADERS.iter().take(6) {
        if let Some(ip) = headers.get(header)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<IpAddr>().ok())
            .filter(|ip| !is_private_ip(*ip))
        {
            debug!("使用 {}({}) 中的IP: {}", header.as_str(), provider, ip);
            return ip;
        }
    }
    // 2. 优先检查通用的 X-Real-IP
    if let Some(real_ip) = headers.get(&CDN_HEADERS[6].0)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<IpAddr>().ok())
        .filter(|ip| !is_private_ip(*ip))
    {
        debug!("使用 X-Real-IP 中的IP: {}", real_ip);
        return real_ip;
    }
    
    // 3. 检查通用的 X-Forwarded-For
    if let Some(forwarded_for) = headers.get(&CDN_HEADERS[7].0)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(ip) = forwarded_for
            .split(',')
            .next()
            .and_then(|s| s.trim().parse::<IpAddr>().ok())
            .filter(|ip| !is_private_ip(*ip))
        {
            debug!("使用 X-Forwarded-For 中的IP: {}", ip);
            return ip;
        }
    }
    
    // 4. 最后检查标准 Forwarded 头
    if let Some(forwarded) = headers.get(&*FORWARDED_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_forwarded_header)
        .filter(|ip| !is_private_ip(*ip))
    {
        debug!("使用 Forwarded 头中的IP: {}", forwarded);
        return forwarded;
    }

    socket_ip
}

// 优化Forwarded头解析
#[inline]
fn parse_forwarded_header(header_value: &str) -> Option<IpAddr> {
    header_value
        .split(';')
        .find(|s| s.trim().starts_with("for="))
        .and_then(|pair| {
            pair.trim()
                .strip_prefix("for=")
                .map(|s| s.trim_matches(|c| c == '"' || c == '[' || c == ']'))
                .and_then(|s| s.parse().ok())
        })
}

async fn handle_ip_lookup(ip: IpAddr) -> Response {
    if is_private_ip(ip) {
        let addr = match ip {
            IpAddr::V4(ip) => {
                if ip.octets()[0] == 127 {
                    "127.0.0.0/8"
                } else if ip.octets()[0] == 10 {
                    "10.0.0.0/8"
                } else if ip.octets()[0] == 172 && (ip.octets()[1] >= 16) {
                    "172.16.0.0/12"
                } else if ip.octets()[0] == 192 && ip.octets()[1] == 168 {
                    "192.168.0.0/16"
                } else {
                    "private"
                }
            },
            IpAddr::V6(ip) => {
                if ip.segments()[0] & 0xffc0 == 0xfe80 {
                    "fe80::/10"
                } else if ip.segments()[0] & 0xfe00 == 0xfc00 {
                    "fc00::/7"
                } else {
                    "private"
                }
            }
        };
        
        let json = serde_json::json!({
            "ip": ip.to_string(),
            "addr": addr
        });
        
        return (
            [(axum::http::header::CONTENT_TYPE, "application/json; charset=utf-8")],
            Json(json)
        ).into_response();
    }
    
    let ip_str = ip.to_string();
    
    match get_ip_info(&ip_str).await {
        Ok(info) => (
            [(axum::http::header::CONTENT_TYPE, "application/json; charset=utf-8")],
            Json(info)
        ).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn root(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    let ip = get_real_ip(&headers, addr);
    handle_ip_lookup(ip).await
}

pub async fn api(
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    let ip = if let Some(host) = params.get("host") {
        match resolve_host(host).await {
            Ok(ip) => ip,
            Err(e) => return e.into_response(),
        }
    } else {
        get_real_ip(&headers, addr)
    };
    
    handle_ip_lookup(ip).await
}

pub async fn path_api(
    Path(host): Path<String>,
    _headers: HeaderMap,
    _addr: ConnectInfo<SocketAddr>,
) -> Response {
    let ip = match resolve_host(&host).await {
        Ok(ip) => ip,
        Err(e) => return e.into_response(),
    };
    
    handle_ip_lookup(ip).await
}

pub fn create_router() -> Router {
    Router::new()
        .route("/", get(root))
        .route("/api", get(api))
        .route("/api/{host}", get(path_api))
        .route("/{host}", get(path_api))
} 