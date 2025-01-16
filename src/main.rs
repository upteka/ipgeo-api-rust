use axum::{
    extract::{Path, Query},
    routing::get,
    Router,
    Json,
    serve,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use maxminddb::geoip2;
use serde::{Serialize, Deserialize};
use std::collections::{BTreeMap, HashMap};
use std::net::IpAddr;
use std::str::FromStr;
use tokio::net::{TcpListener, lookup_host};
use tokio::signal;

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
    #[serde(rename = "as")]
    asn: Option<AsnInfo>,
    addr: String,
    location: Option<Location>,
    country: Option<CountryInfo>,
    registered_country: Option<CountryInfo>,
    regions: Option<Vec<String>>,
    regions_short: Option<Vec<String>>,
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
        if name == "香港" || name == "澳门" || name == "台湾" {
            return format!("中国{}", name);
        }
        name
    } else {
        "Unknown".to_string()
    }
}

fn get_addr(ip: &str, prefix_len: u8) -> String {
    let ip_addr = IpAddr::from_str(ip).unwrap();
    match ip_addr {
        IpAddr::V4(_) => format!("{}/{}", ip, prefix_len),
        IpAddr::V6(_) => format!("{}/{}", ip, if prefix_len < 32 { 48 } else { 64 }), // IPv6 使用更合适的网段长度
    }
}

// Assuming GeoCN.mmdb has 'network' directly in the struct
#[derive(Deserialize, Debug)]
struct GeoCNInfo<'a> {
    #[serde(flatten, bound(deserialize = "'de: 'a"))]
    city: geoip2::City<'a>,
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

fn get_ip_info(ip: &str) -> IpInfo {
    let mut info = IpInfo::default();
    info.ip = ip.to_string();
    let ip_addr = IpAddr::from_str(ip).unwrap();

    // 根据 IP 类型设置默认网段
    let default_prefix = match ip_addr {
        IpAddr::V4(_) => 16,
        IpAddr::V6(_) => 48,
    };
    info.addr = get_addr(ip, default_prefix);

    let mmdb_path = std::env::var("MMDB_PATH").unwrap_or_else(|_| ".".to_string());
    eprintln!("使用数据库路径: {}", mmdb_path);

    // 列出目录内容
    if let Ok(entries) = std::fs::read_dir(&mmdb_path) {
        eprintln!("目录 {} 的内容:", mmdb_path);
        for entry in entries {
            if let Ok(entry) = entry {
                eprintln!("  - {}", entry.path().display());
            }
        }
    } else {
        eprintln!("无法读取目录 {}", mmdb_path);
    }

    // 优先查询 GeoCN 数据库
    let geocn_path = format!("{}/GeoCN.mmdb", mmdb_path);
    eprintln!("尝试打开 GeoCN 数据库: {}", geocn_path);
    match maxminddb::Reader::open_readfile(&geocn_path) {
        Ok(cn_reader) => {
            eprintln!("成功打开 GeoCN 数据库");
            match cn_reader.lookup::<GeoCNInfo>(ip_addr) {
                Ok(cn_info) => {
                    eprintln!("成功查询 GeoCN 数据");
                    // 设置网段信息
                    if let Some(network) = cn_info.network {
                        info.addr = network;
                    } else {
                        info.addr = get_addr(ip, 16); // 默认使用 /16
                    }

                    // 设置地区信息
                    if let Some(subdivisions) = cn_info.city.subdivisions {
                        let mut regions = Vec::new();
                        let mut regions_short = Vec::new();
                        
                        for sub in subdivisions {
                            if let Some(names) = &sub.names {
                                if let Some(name) = names.get("zh-CN") {
                                    let full_name = name.to_string();
                                    let short_name = get_short_name(name);
                                    regions.push(full_name);
                                    regions_short.push(short_name);
                                }
                            }
                        }
                        
                        if !regions.is_empty() {
                            info.regions = Some(regions);
                            info.regions_short = Some(regions_short);
                        }
                    }

                    // 设置国家信息
                    info.country = Some(CountryInfo {
                        code: "CN".to_string(),
                        name: "中国".to_string(),
                    });
                    info.registered_country = info.country.clone();
                }
                Err(e) => eprintln!("GeoCN 查询失败: {}", e),
            }
        }
        Err(e) => eprintln!("无法打开 GeoCN 数据库: {} (错误: {})", geocn_path, e),
    }

    // 如果 GeoCN 没有数据，使用 MaxMind City 数据库
    if info.country.is_none() {
        let city_path = format!("{}/GeoLite2-City.mmdb", mmdb_path);
        eprintln!("尝试打开 City 数据库: {}", city_path);
        match maxminddb::Reader::open_readfile(&city_path) {
            Ok(city_reader) => {
                eprintln!("成功打开 City 数据库");
                match city_reader.lookup::<geoip2::City>(ip_addr) {
                    Ok(city_info) => {
                        eprintln!("成功查询 City 数据");
                        info.addr = get_addr(ip, 16); // 默认使用 /16

                        if let Some(location) = city_info.location {
                            info.location = Some(Location {
                                latitude: location.latitude,
                                longitude: location.longitude,
                            });
                        }
                        if let Some(country) = city_info.country {
                            info.country = Some(CountryInfo {
                                code: country.iso_code.unwrap_or("Unknown").to_string(),
                                name: get_country(&country),
                            });
                        }
                        if let Some(registered_country) = city_info.registered_country {
                            info.registered_country = Some(CountryInfo {
                                code: registered_country.iso_code.unwrap_or("Unknown").to_string(),
                                name: get_country(&registered_country),
                            });
                        }
                        if let Some(subdivisions) = city_info.subdivisions {
                            let mut regions = Vec::new();
                            let mut regions_short = Vec::new();
                            
                            for sub in subdivisions {
                                if let Some(names) = &sub.names {
                                    let name = get_des(&Some(names.clone()), &["zh-CN", "en"]);
                                    let short_name = get_short_name(&name);
                                    regions.push(name);
                                    regions_short.push(short_name);
                                }
                            }
                            
                            if !regions.is_empty() {
                                info.regions = Some(regions);
                                info.regions_short = Some(regions_short);
                            }
                        }
                    }
                    Err(e) => eprintln!("City 查询失败: {}", e),
                }
            }
            Err(e) => eprintln!("无法打开 City 数据库: {} (错误: {})", city_path, e),
        }
    }

    // 查询 ASN 信息
    let asn_path = format!("{}/GeoLite2-ASN.mmdb", mmdb_path);
    eprintln!("尝试打开 ASN 数据库: {}", asn_path);
    match maxminddb::Reader::open_readfile(&asn_path) {
        Ok(asn_reader) => {
            eprintln!("成功打开 ASN 数据库");
            match asn_reader.lookup::<geoip2::Asn>(ip_addr) {
                Ok(asn_info) => {
                    eprintln!("成功查询 ASN 数据");
                    if let Some(number) = asn_info.autonomous_system_number {
                        info.asn = Some(AsnInfo {
                            number,
                            name: asn_info.autonomous_system_organization.unwrap_or("Unknown").to_string(),
                            info: String::new(),
                        });
                    }
                }
                Err(e) => eprintln!("ASN 查询失败: {}", e),
            }
        }
        Err(e) => eprintln!("无法打开 ASN 数据库: {} (错误: {})", asn_path, e),
    }

    info
}

// 添加错误处理
#[derive(Debug)]
enum ApiError {
    ResolveError,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::ResolveError => (StatusCode::BAD_REQUEST, "域名解析失败"),
        };
        
        (status, Json(serde_json::json!({
            "error": message
        }))).into_response()
    }
}

async fn resolve_host(host: &str) -> Result<IpAddr, ApiError> {
    if let Ok(ip) = IpAddr::from_str(host) {
        return Ok(ip);
    }
    
    lookup_host(format!("{}:0", host))
        .await
        .ok()
        .and_then(|mut addrs| addrs.next())
        .map(|addr| addr.ip())
        .ok_or(ApiError::ResolveError)
}

async fn index() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "message": "IP地理位置查询服务",
        "endpoints": {
            "/api?host=example.com": "使用查询参数查询",
            "/api/example.com": "使用路径参数查询",
            "/8.8.8.8": "直接查询IP地址"
        }
    }))
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct HostInfo {
    host: String,
    ips: Vec<IpInfo>,
}

async fn api(Query(params): Query<HashMap<String, String>>) -> Result<Json<HostInfo>, ApiError> {
    let query_host = params.get("host").cloned().unwrap_or_else(|| "127.0.0.1".to_string());
    let ip = resolve_host(&query_host).await?;
    let info = get_ip_info(&ip.to_string());
    Ok(Json(HostInfo {
        host: query_host,
        ips: vec![info],
    }))
}

async fn path_api(Path(host): Path<String>) -> Result<Json<HostInfo>, ApiError> {
    let ip = resolve_host(&host).await?;
    let info = get_ip_info(&ip.to_string());
    Ok(Json(HostInfo {
        host,
        ips: vec![info],
    }))
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/api", get(api))
        .route("/api/{host}", get(path_api))
        .route("/{host}", get(path_api))
        .with_state(());
    
    let addr = "0.0.0.0:8080";
    println!("正在尝试启动服务器...");
    
    match TcpListener::bind(addr).await {
        Ok(listener) => {
            println!("服务器启动成功！");
            println!("可以通过以下地址访问：");
            println!("  http://{}", addr);
            println!("  http://localhost:8080");
            println!("\n示例查询：");
            println!("  http://localhost:8080/8.8.8.8");
            println!("  http://localhost:8080/api/google.com");
            println!("  http://localhost:8080/api?host=github.com");
            
            if let Err(err) = serve(listener, app.into_make_service())
                .with_graceful_shutdown(shutdown_signal())
                .await 
            {
                eprintln!("服务器运行错误: {}", err);
            }
        }
        Err(err) => {
            eprintln!("无法启动服务器: {}", err);
            eprintln!("请检查：");
            eprintln!("1. 端口 8080 是否已被占用");
            eprintln!("2. 是否有权限绑定该端口");
            eprintln!("3. 防火墙设置是否允许该连接");
            std::process::exit(1);
        }
    }
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

    println!("正在关闭服务器...");
}