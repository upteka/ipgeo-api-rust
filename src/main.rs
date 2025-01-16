use axum::{
    extract::{Path, Query, connect_info::ConnectInfo},
    routing::get,
    Router,
    Json,
    http::{StatusCode, Request, HeaderMap},
    response::{IntoResponse, Response},
};
use maxminddb::geoip2;
use serde::{Serialize, Deserialize};
use std::collections::{BTreeMap, HashMap};
use std::net::{IpAddr, SocketAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use tokio::net::{TcpListener, lookup_host};
use tokio::signal;
use std::sync::OnceLock;
use serde_json::Value;

static ASN_DATA: OnceLock<Value> = OnceLock::new();

fn load_asn_data() -> &'static Value {
    ASN_DATA.get_or_init(|| {
        let data = include_str!("asn_info.json");
        serde_json::from_str(data).expect("Failed to parse ASN info JSON")
    })
}

fn get_asn_info_from_json(asn: u32, org_name: &str) -> (String, Option<String>) {
    let data = load_asn_data();
    
    // 首先检查是否有直接的 ASN 匹配
    if let Some(asn_info) = data["asn_info"].get(&asn.to_string()) {
        return (
            asn_info["name"].as_str().unwrap_or("Unknown").to_string(),
            Some(asn_info["type"].as_str().unwrap_or("Unknown").to_string())
        );
    }

    let org_name_lower = org_name.to_lowercase();
    
    // 检查云服务提供商和 CDN
    let cloud = &data["patterns"]["cloud"];
    for keyword in cloud["keywords"].as_array().unwrap() {
        let key = keyword.as_str().unwrap().to_lowercase();
        if org_name_lower.contains(&key) {
            if let Some(info) = cloud["info"].get(keyword.as_str().unwrap()) {
                return (
                    info.as_str().unwrap().to_string(),
                    Some(cloud["type"].as_str().unwrap().to_string())
                );
            }
        }
    }
    
    // 检查 ISP
    let isp = &data["patterns"]["isp"];
    for keyword in isp["keywords"].as_array().unwrap() {
        let key = keyword.as_str().unwrap().to_lowercase();
        if org_name_lower.contains(&key) {
            if let Some(info) = isp["info"].get(keyword.as_str().unwrap()) {
                return (
                    info.as_str().unwrap().to_string(),
                    Some(isp["type"][keyword.as_str().unwrap()].as_str().unwrap().to_string())
                );
            }
        }
    }
    
    (org_name.to_string(), None)
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct AsnInfo {
    number: u32,
    name: String,
    info: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Location {
    latitude: Option<f64>,
    longitude: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct CountryInfo {
    code: String,
    name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct IpInfo {
    ip: String,
    #[serde(rename = "as", skip_serializing_if = "Option::is_none")]
    asn: Option<AsnInfo>,
    addr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    country: Option<CountryInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    registered_country: Option<CountryInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    regions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    regions_short: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<String>,
}

fn get_des(names: &Option<BTreeMap<&str, &str>>, lang: &[&str]) -> String {
    if let Some(names) = names {
        for l in lang {
            if let Some(name) = names.get(l) {
                return name.to_string();
            }
        }
        names.get("en").map(|s| s.to_string()).unwrap_or_else(|| "Unknown".to_string())
    } else {
        "Unknown".to_string()
    }
}

fn get_country(country: &geoip2::country::Country) -> String {
    if let Some(name_map) = &country.names {
        let name = get_des(&Some(name_map.clone()), &["zh-CN", "en"]);
        if let Some(code) = country.iso_code {
            match code {
                "HK" => "中国香港".to_string(),
                "MO" => "中国澳门".to_string(),
                "TW" => "中国台湾".to_string(),
                _ => name
            }
        } else {
            name
        }
    } else {
        "Unknown".to_string()
    }
}

#[derive(Debug)]
enum IpGeoError {
    InvalidIp(String),
    ResolveError,
    DatabaseError(String),
    IoError(std::io::Error),
}

impl std::fmt::Display for IpGeoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIp(msg) => write!(f, "无效的IP地址: {}", msg),
            Self::ResolveError => write!(f, "域名解析失败"),
            Self::DatabaseError(msg) => write!(f, "数据库错误: {}", msg),
            Self::IoError(e) => write!(f, "IO错误: {}", e),
        }
    }
}

impl std::error::Error for IpGeoError {}

impl From<std::io::Error> for IpGeoError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

fn get_addr(ip: &str, prefix_len: u8) -> Result<String, IpGeoError> {
    let ip_addr = IpAddr::from_str(ip).map_err(|e| IpGeoError::InvalidIp(e.to_string()))?;
    let network_prefix = match ip_addr {
        IpAddr::V4(ipv4) => {
            let ip_int: u32 = u32::from(ipv4);
            let mask = !((1u32 << (32 - prefix_len)) - 1);
            let network = Ipv4Addr::from(ip_int & mask);
            format!("{}/{}", network, prefix_len)
        }
        IpAddr::V6(ipv6) => {
            let segments = ipv6.segments();
            let prefix = if prefix_len < 32 { 48 } else { 64 };
            let network_segments = segments.iter()
                .enumerate()
                .map(|(i, &seg)| {
                    if i * 16 < prefix {
                        seg
                    } else if i * 16 >= prefix + 16 {
                        0
                    } else {
                        let shift = 16 - (prefix - i * 16);
                        (seg >> shift) << shift
                    }
                })
                .collect::<Vec<_>>();
            let network = Ipv6Addr::new(
                network_segments[0], network_segments[1], network_segments[2], network_segments[3],
                network_segments[4], network_segments[5], network_segments[6], network_segments[7]
            );
            format!("{}/{}", network, prefix)
        }
    };
    Ok(network_prefix)
}

fn get_network_prefix(ip: &str) -> Result<String, IpGeoError> {
    let ip_addr = IpAddr::from_str(ip).map_err(|e| IpGeoError::InvalidIp(e.to_string()))?;
    match ip_addr {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            Ok(match octets[0] {
                223 if octets[1] >= 4 && octets[1] <= 7 => "223.4.0.0/14".to_string(),
                _ => format!("{}.{}.0.0/16", octets[0], octets[1])
            })
        }
        IpAddr::V6(_) => get_addr(ip, 48),
    }
}

#[derive(Deserialize, Debug)]
struct GeoCNInfo {
    city: Option<String>,
    province: Option<String>,
    network: Option<String>,
}

fn get_short_name(name: &str) -> String {
    let suffixes = ["省", "市", "自治区", "特别行政区"];
    let mut result = name.to_string();
    for suffix in suffixes {
        result = result.trim_end_matches(suffix).to_string();
    }
    result
}

fn get_ip_info(ip: &str) -> Result<IpInfo, IpGeoError> {
    let mut info = IpInfo::default();
    info.ip = ip.to_string();
    let ip_addr = IpAddr::from_str(ip).map_err(|e| IpGeoError::InvalidIp(e.to_string()))?;
    println!("🔍 查询IP: {}", ip);

    info.addr = get_network_prefix(ip)?;

    let mmdb_path = std::env::var("MMDB_PATH").unwrap_or_else(|_| ".".to_string());

    // ASN 数据库查询（提前查询以便后面使用）
    let asn_path = format!("{}/GeoLite2-ASN.mmdb", mmdb_path);
    let asn_reader = maxminddb::Reader::open_readfile(&asn_path)
        .map_err(|e| IpGeoError::DatabaseError(format!("无法打开 ASN 数据库: {}", e)))?;
    
    if let Ok(asn_info) = asn_reader.lookup::<geoip2::Asn>(ip_addr) {
        if let Some(number) = asn_info.autonomous_system_number {
            let name = asn_info.autonomous_system_organization.unwrap_or("Unknown").to_string();
            let (info_str, ip_type) = get_asn_info_from_json(number, &name);
            info.asn = Some(AsnInfo {
                number,
                name,
                info: info_str,
            });
            info.r#type = ip_type;
        }
    }

    // 查询 GeoCN 数据库
    let geocn_path = format!("{}/GeoCN.mmdb", mmdb_path);
    let cn_reader = maxminddb::Reader::open_readfile(&geocn_path)
        .map_err(|e| IpGeoError::DatabaseError(format!("无法打开 GeoCN 数据库: {}", e)))?;
    
    if let Err(e) = cn_reader.lookup::<GeoCNInfo>(ip_addr)
        .map_err(|e| IpGeoError::DatabaseError(format!("GeoCN 查询失败: {}", e)))
        .and_then(|cn_info| {
            if let Some(network) = cn_info.network {
                info.addr = network;
            }

            let mut regions = Vec::new();
            let mut regions_short = Vec::new();

            if let Some(province) = cn_info.province {
                if !province.is_empty() {
                    regions.push(province.clone());
                    regions_short.push(get_short_name(&province));
                }
            }
            
            if let Some(city) = cn_info.city {
                if !city.is_empty() {
                    regions.push(city.clone());
                    regions_short.push(get_short_name(&city));
                }
            }

            if !regions.is_empty() {
                info.regions = Some(regions);
                info.regions_short = Some(regions_short);
            }

            info.country = Some(CountryInfo {
                code: "CN".to_string(),
                name: "中国".to_string(),
            });
            info.registered_country = info.country.clone();
            Ok(())
        }) {
        println!("❌ GeoCN 查询失败: {}", e);
    }

    // City 数据库查询
    let city_path = format!("{}/GeoLite2-City.mmdb", mmdb_path);
    let city_reader = maxminddb::Reader::open_readfile(&city_path)
        .map_err(|e| IpGeoError::DatabaseError(format!("无法打开 City 数据库: {}", e)))?;
    
    if let Err(e) = city_reader.lookup::<geoip2::City>(ip_addr)
        .map_err(|e| IpGeoError::DatabaseError(format!("City 查询失败: {}", e)))
        .and_then(|city_info| {
            if info.location.is_none() {
                if let Some(location) = city_info.location {
                    info.location = Some(Location {
                        latitude: location.latitude,
                        longitude: location.longitude,
                    });
                }
            }
            
            if info.country.is_none() {
                if let Some(country) = city_info.country {
                    info.country = Some(CountryInfo {
                        code: country.iso_code.unwrap_or("Unknown").to_string(),
                        name: get_country(&country),
                    });
                }
            }
            
            if info.registered_country.is_none() {
                if let Some(registered_country) = city_info.registered_country {
                    info.registered_country = Some(CountryInfo {
                        code: registered_country.iso_code.unwrap_or("Unknown").to_string(),
                        name: get_country(&registered_country),
                    });
                }
            }
            
            if info.regions.is_none() && info.regions_short.is_none() {
                if let Some(subdivisions) = city_info.subdivisions {
                    let mut regions = Vec::new();
                    let mut regions_short = Vec::new();
                    
                    for sub in subdivisions {
                        if let Some(names) = &sub.names {
                            let name = get_des(&Some(names.clone()), &["zh-CN", "en"]);
                            if !name.is_empty() && name != "Unknown" {
                                let short_name = get_short_name(&name);
                                regions.push(name);
                                regions_short.push(short_name);
                            }
                        }
                    }
                    
                    if !regions.is_empty() {
                        info.regions = Some(regions);
                        info.regions_short = Some(regions_short);
                    }
                }
            }
            Ok(())
        }) {
        println!("❌ City 查询失败: {}", e);
    }

    // 清理空的 regions
    if let Some(regions) = &info.regions {
        if regions.is_empty() {
            info.regions = None;
            info.regions_short = None;
        }
    }

    Ok(info)
}

async fn resolve_host(host: &str) -> Result<IpAddr, IpGeoError> {
    if let Ok(ip) = IpAddr::from_str(host) {
        return Ok(ip);
    }
    
    lookup_host(format!("{}:0", host))
        .await
        .ok()
        .and_then(|mut addrs| addrs.next())
        .map(|addr| addr.ip())
        .ok_or(IpGeoError::ResolveError)
}


#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct HostInfo {
    host: String,
    ips: Vec<IpInfo>,
}

fn get_real_ip(headers: &HeaderMap, socket_addr: SocketAddr) -> IpAddr {
    // 按优先级尝试从不同的头部获取 IP
    let ip_headers = [
        "CF-Connecting-IP",     // Cloudflare
        "X-Real-IP",           // Nginx
        "X-Forwarded-For",     // 通用
        "True-Client-IP",      // Akamai
        "X-Client-IP",         // 通用
    ];

    for header_name in ip_headers {
        if let Some(ip_str) = headers.get(header_name).and_then(|v| v.to_str().ok()) {
            // 对于 X-Forwarded-For，取第一个 IP（客户端真实 IP）
            let ip = if header_name == "X-Forwarded-For" {
                ip_str.split(',').next().unwrap_or("").trim()
            } else {
                ip_str
            };
            
            if let Ok(ip) = IpAddr::from_str(ip) {
                if !is_private_ip(ip) {
                    return ip;
                }
            }
        }
    }

    // 如果没有找到有效的 IP，返回 socket 地址
    socket_addr.ip()
}

async fn root(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let ip = get_real_ip(&headers, addr);
    
    if is_private_ip(ip) {
        // 返回简单的 IP 信息
        return Json(IpInfo {
            ip: ip.to_string(),
            asn: None,
            addr: get_network_prefix(&ip.to_string()).unwrap_or_else(|_| String::new()),
            location: None,
            country: None,
            registered_country: None,
            regions: None,
            regions_short: None,
            r#type: Some("私有网络".to_string()),
        }).into_response();
    }
    
    match get_ip_info(&ip.to_string()) {
        Ok(info) => Json(info).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn api(
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let query_host = params.get("host").cloned().unwrap_or_else(|| {
        get_real_ip(&headers, addr).to_string()
    });
    match resolve_host(&query_host).await.and_then(|ip| get_ip_info(&ip.to_string())) {
        Ok(info) => Json(HostInfo {
            host: query_host,
            ips: vec![info],
        }).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn path_api(
    Path(host): Path<String>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let query_host = if host == "me" {
        get_real_ip(&headers, addr).to_string()
    } else {
        host
    };
    match resolve_host(&query_host).await.and_then(|ip| get_ip_info(&ip.to_string())) {
        Ok(info) => Json(HostInfo {
            host: query_host,
            ips: vec![info],
        }).into_response(),
        Err(e) => e.into_response(),
    }
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            // 10.0.0.0/8
            octets[0] == 10 ||
            // 172.16.0.0/12
            (octets[0] == 172 && (octets[1] >= 16 && octets[1] <= 31)) ||
            // 192.168.0.0/16
            (octets[0] == 192 && octets[1] == 168) ||
            // 169.254.0.0/16
            (octets[0] == 169 && octets[1] == 254) ||
            // 127.0.0.0/8
            octets[0] == 127
        }
        IpAddr::V6(ip) => {
            ip.is_loopback() || ip.is_unspecified() || ip.is_unique_local()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mmdb_path = std::env::var("MMDB_PATH").unwrap_or_else(|_| "/app/data".to_string());
    println!("📂 数据库路径: {}", mmdb_path);

    let app = Router::new()
        .route("/", get(root))
        .route("/api", get(api))
        .route("/api/:host", get(path_api))
        .route("/:host", get(path_api))
        .into_make_service_with_connect_info::<SocketAddr>();

    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("\n🚀 服务器已启动");
    println!("📡 监听地址: {}", addr);
    println!("\n📝 示例查询：");
    println!("  http://localhost:8080/");
    println!("  http://localhost:8080/8.8.8.8");
    println!("  http://localhost:8080/api/8.8.8.8");
    println!("  http://localhost:8080/api?host=google.com\n");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("\n👋 正在关闭服务器...");
}

impl IntoResponse for IpGeoError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::InvalidIp(msg) => (StatusCode::BAD_REQUEST, format!("无效的IP地址: {}", msg)),
            Self::ResolveError => (StatusCode::BAD_REQUEST, "域名解析失败".to_string()),
            Self::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("数据库错误: {}", msg)),
            Self::IoError(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("IO错误: {}", e)),
        };
        
        (status, Json(serde_json::json!({
            "error": message
        }))).into_response()
    }
}