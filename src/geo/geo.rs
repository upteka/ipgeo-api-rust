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
    // 如果输入已经是IP地址，直接返回
    if let Ok(ip) = host.parse() {
        return Ok(ip);
    }
    
    // 否则进行DNS解析
    let addr = lookup_host(format!("{}:0", host))
        .await
        .map_err(|_| IpGeoError::ResolveError)?
        .next()
        .ok_or(IpGeoError::ResolveError)?;
    
    Ok(addr.ip())
} 