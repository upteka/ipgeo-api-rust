use axum::{
    extract::{Path, Query, ConnectInfo},
    routing::get,
    Router,
    Json,
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use crate::geo::{get_ip_info, resolve_host};
use crate::utils::is_private_ip;

pub fn get_real_ip(headers: &HeaderMap, socket_addr: SocketAddr) -> IpAddr {
    let real_ip = headers
        .get("X-Real-IP")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<IpAddr>().ok());

    let forwarded_for = headers
        .get("X-Forwarded-For")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<IpAddr>().ok());

    real_ip
        .or(forwarded_for)
        .unwrap_or_else(|| socket_addr.ip())
}

async fn handle_ip_lookup(ip: IpAddr) -> Response {
    if is_private_ip(ip) {
        let addr = match ip {
            IpAddr::V4(ip) => {
                if ip.octets()[0] == 127 {
                    "127.0.0.0/8"
                } else if ip.octets()[0] == 10 {
                    "10.0.0.0/8"
                } else if ip.octets()[0] == 172 && (ip.octets()[1] >= 16 && ip.octets()[1] <= 31) {
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
        
        return Json(serde_json::json!({
            "ip": ip.to_string(),
            "addr": addr
        })).into_response();
    }
    
    let ip_str = ip.to_string();
    
    match get_ip_info(&ip_str).await {
        Ok(info) => Json(info).into_response(),
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