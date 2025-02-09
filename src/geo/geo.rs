use std::sync::{Arc, RwLock};
use maxminddb::{geoip2, MaxMindDBError};
use std::net::IpAddr;
use tokio::net::lookup_host;
use std::path::Path;
use crate::models::{IpInfo, AsnInfo as ModelAsnInfo, Location, CountryInfo, IpGeoError};
use crate::utils::{get_country, get_short_name};
use crate::cache::CacheManager;
use tracing::info;
use once_cell::sync::Lazy;

// 使用Arc<RwLock>替代OnceLock
static ASN_READER: Lazy<Arc<RwLock<maxminddb::Reader<Vec<u8>>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(maxminddb::Reader::open_readfile("data/GeoLite2-ASN.mmdb")
        .expect("Failed to open ASN database")))
});

static GEOCN_READER: Lazy<Arc<RwLock<maxminddb::Reader<Vec<u8>>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(maxminddb::Reader::open_readfile("data/GeoCN.mmdb")
        .expect("Failed to open GeoCN database")))
});

static CITY_READER: Lazy<Arc<RwLock<maxminddb::Reader<Vec<u8>>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(maxminddb::Reader::open_readfile("data/GeoLite2-City.mmdb")
        .expect("Failed to open City database")))
});

// 添加重新加载函数
pub fn reload_database(db_type: &str, path: &Path) -> std::io::Result<()> {
    match db_type {
        "ASN" => {
            let new_reader = maxminddb::Reader::open_readfile(path)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            if let Ok(mut reader) = ASN_READER.write() {
                *reader = new_reader;
                info!("ASN database reloaded successfully");
            }
        }
        "GeoCN" => {
            let new_reader = maxminddb::Reader::open_readfile(path)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            if let Ok(mut reader) = GEOCN_READER.write() {
                *reader = new_reader;
                info!("GeoCN database reloaded successfully");
            }
        }
        "City" => {
            let new_reader = maxminddb::Reader::open_readfile(path)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            if let Ok(mut reader) = CITY_READER.write() {
                *reader = new_reader;
                info!("City database reloaded successfully");
            }
        }
        _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Unknown database type")),
    }
    Ok(())
}

// 修改获取读取器的函数
pub fn get_asn_reader() -> Arc<RwLock<maxminddb::Reader<Vec<u8>>>> {
    ASN_READER.clone()
}

pub fn get_geocn_reader() -> Arc<RwLock<maxminddb::Reader<Vec<u8>>>> {
    GEOCN_READER.clone()
}

pub fn get_city_reader() -> Arc<RwLock<maxminddb::Reader<Vec<u8>>>> {
    CITY_READER.clone()
}

pub async fn init_mmdb_readers() -> std::io::Result<()> {
    let data_dir = Path::new("data");
    let db_manager = super::database::DatabaseManager::new(data_dir.to_path_buf());
    
    // 初始化ASN数据
    let asn_data = {
        let path = db_manager.get_data_file_path("asn_info.json");
        let data = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("Failed to read ASN info at {:?}", path));
        serde_json::from_str(&data)
            .unwrap_or_else(|_| panic!("Failed to parse ASN info at {:?}", path))
    };
    
    // 初始化缓存
    CacheManager::global().init_asn_data(&asn_data);
    
    // 初始更新数据库
    db_manager.update_databases().await?;
    
    // 启动自动更新任务
    db_manager.start_auto_update().await;
    
    Ok(())
}

pub async fn get_ip_info(ip_str: &str) -> Result<IpInfo, IpGeoError> {
    let ip: IpAddr = ip_str.parse()?;
    
    // 查询ASN信息
    let (asn, asn_type) = if let Ok(reader) = get_asn_reader().read() {
        if let Ok(asn) = reader.lookup::<geoip2::Asn>(ip) {
            let number = asn.autonomous_system_number.unwrap_or(0);
            let org_name = asn.autonomous_system_organization.unwrap_or("").to_string();
            
            // 从缓存获取ASN详细信息
            let (name, asn_type) = if let Some((name, type_info)) = CacheManager::global().get_asn_info(number) {
                (name.into_string(), Some(type_info))
            } else {
                (org_name, None)
            };
            
            (Some(ModelAsnInfo {
                number,
                name: name.clone(),
                info: name,
            }), asn_type)
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    // 构建IP信息
    let mut info = IpInfo {
        ip: ip_str.to_string(),
        asn,
        addr: String::new(),
        location: None,
        country: None,
        registered_country: None,
        regions: None,
        regions_short: None,
        r#type: None,
    };
    
    // 设置网络类型
    if let Some(asn_type) = asn_type {
        use crate::cache::AsnType;
        info.r#type = Some(match asn_type {
            AsnType::Type(t) => t.into_string(),
            AsnType::Other => "其他网络".to_string(),
        });
    }

    // 查询地理位置信息
    if let Ok(reader) = get_city_reader().read() {
        if let Ok(city) = reader.lookup::<geoip2::City>(ip) {
            // 处理位置信息
            if let (Some(lat), Some(lon)) = (
                city.location.as_ref().and_then(|l| l.latitude),
                city.location.as_ref().and_then(|l| l.longitude)
            ) {
                info.location = Some(Location {
                    latitude: Some(lat),
                    longitude: Some(lon),
                });
            }
            
            // 处理国家信息
            if let Some(country) = city.country {
                let name = get_country(&country);
                if !name.is_empty() {
                    info.country = Some(CountryInfo {
                        code: country.iso_code.unwrap_or_default().to_string(),
                        name,
                    });
                }
            }
            
            // 处理注册国家信息
            if let Some(registered_country) = city.registered_country {
                let name = get_country(&registered_country);
                if !name.is_empty() {
                    info.registered_country = Some(CountryInfo {
                        code: registered_country.iso_code.unwrap_or_default().to_string(),
                        name,
                    });
                }
            }
            
            // 处理地区信息
            let mut regions = Vec::with_capacity(2);
            let mut regions_short = Vec::with_capacity(2);
            
            // 添加省级信息
            if let Some(subdivisions) = city.subdivisions {
                if let Some(province) = subdivisions.first() {
                    if let Some(names) = &province.names {
                        if let Some(name) = names.get("zh-CN") {
                            let province_name = if !name.ends_with("省") 
                                && !name.ends_with("自治区") 
                                && !name.ends_with("特别行政区") {
                                format!("{}省", name)
                            } else {
                                name.to_string()
                            };
                            regions.push(province_name);
                            regions_short.push(get_short_name(name).to_string());
                        }
                    }
                }
            }
            
            // 添加市级信息
            if let Some(city_info) = city.city {
                if let Some(names) = city_info.names {
                    if let Some(name) = names.get("zh-CN") {
                        let city_name = if !name.ends_with("市") {
                            format!("{}市", name)
                        } else {
                            name.to_string()
                        };
                        regions.push(city_name);
                        regions_short.push(get_short_name(name).to_string());
                    }
                }
            }
            
            if !regions.is_empty() {
                info.regions = Some(regions);
                info.regions_short = Some(regions_short);
            }
        }
    }
    
    // 设置地址信息
    if info.asn.is_some() {
        match ip {
            IpAddr::V4(ipv4) => {
                let octets = ipv4.octets();
                info.addr = format!("{}.{}.0.0/16", octets[0], octets[1]);
            }
            IpAddr::V6(ipv6) => {
                let segments = ipv6.segments();
                info.addr = format!("{:x}:{:x}::/32", segments[0], segments[1]);
            }
        }
    }
    
    Ok(info)
}

pub async fn resolve_host(host: &str) -> Result<IpAddr, IpGeoError> {
    // 首先验证是否为有效的IP地址格式
    if let Ok(ip) = host.parse() {
        // 验证IP地址的有效性
        match ip {
            IpAddr::V4(ipv4) => {
                let octets = ipv4.octets();
                // 检查是否为有效的公网IP地址
                if octets[0] == 0 || // 0.0.0.0/8
                   octets == [255, 255, 255, 255] || // 广播地址
                   octets == [0, 0, 0, 0] || // 未指定地址
                   (octets[0] == 192 && octets[1] == 0 && octets[2] == 2) || // 文档地址
                   (octets[0] == 198 && octets[1] == 51 && octets[2] == 100) || // 文档地址
                   (octets[0] == 203 && octets[1] == 0 && octets[2] == 113) // 文档地址
                {
                    return Err(IpGeoError::InvalidIp(format!("无效的IPv4地址: {}", host)));
                }
            },
            IpAddr::V6(ipv6) => {
                let segments = ipv6.segments();
                if segments == [0, 0, 0, 0, 0, 0, 0, 0] || // 未指定地址
                   (segments[0] == 0x2001 && segments[1] == 0xdb8) // 文档地址
                {
                    return Err(IpGeoError::InvalidIp(format!("无效的IPv6地址: {}", host)));
                }
            }
        }
        return Ok(ip);
    }
    
    // 验证域名格式
    if !is_valid_domain(host) {
        return Err(IpGeoError::ResolveError);
    }
    
    // 如果是有效域名，尝试解析
    match tokio::time::timeout(
        std::time::Duration::from_secs(1), // 设置1秒超时
        lookup_host(format!("{}:0", host))
    ).await {
        Ok(Ok(mut addrs)) => {
            // 优先返回IPv4地址
            while let Some(addr) = addrs.next() {
                match addr.ip() {
                    IpAddr::V4(_) => return Ok(addr.ip()),
                    _ => continue,
                }
            }
            // 如果没有IPv4地址，返回第一个IPv6地址
            addrs.next()
                .map(|addr| addr.ip())
                .ok_or(IpGeoError::ResolveError)
        },
        Ok(Err(_)) => Err(IpGeoError::ResolveError),
        Err(_) => Err(IpGeoError::TimeoutError),
    }
}

/// 检查是否为有效的域名格式
fn is_valid_domain(host: &str) -> bool {
    // 域名的基本验证规则
    // 1. 长度在1-253之间
    if host.len() < 1 || host.len() > 253 {
        return false;
    }
    
    // 2. 只包含字母、数字、点和连字符
    if !host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') {
        return false;
    }
    
    // 3. 不能以点或连字符开始或结束
    if host.starts_with('.') || host.starts_with('-') || 
       host.ends_with('.') || host.ends_with('-') {
        return false;
    }
    
    // 4. 至少包含一个点（排除纯数字的情况）
    if !host.contains('.') || host.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return false;
    }
    
    // 5. 每个标签（点之间的部分）长度在1-63之间
    let labels: Vec<&str> = host.split('.').collect();
    for label in labels {
        if label.len() < 1 || label.len() > 63 {
            return false;
        }
    }
    
    true
} 