use crate::models::IpInfo;
use maxminddb::geoip2;
use std::collections::BTreeMap;
use std::net::IpAddr;

pub fn get_des(names: &Option<BTreeMap<&str, &str>>, lang: &[&str]) -> String {
    if let Some(names) = names {
        for lang_code in lang {
            if let Some(name) = names.get(lang_code) {
                return name.to_string();
            }
        }
    }
    String::new()
}

pub fn get_country(country: &geoip2::country::Country) -> String {
    let lang = &["zh-CN", "en"];
    get_des(&country.names, lang)
}

pub fn get_short_name(name: &str) -> String {
    // 移除常见后缀
    let name = name.trim()
        .replace("省", "")
        .replace("自治区", "")
        .replace("维吾尔", "")
        .replace("壮族", "")
        .replace("回族", "")
        .replace("市", "")
        .replace("特别行政区", "");
    
    // 如果是直辖市，直接返回
    if ["北京", "上海", "天津", "重庆"].contains(&name.as_str()) {
        return name;
    }
    
    // 如果是特别行政区，直接返回
    if ["香港", "澳门"].contains(&name.as_str()) {
        return name;
    }
    
    // 如果长度小于等于2，直接返回
    if name.chars().count() <= 2 {
        return name.to_string();
    }
    
    // 否则取前两个字
    name.chars().take(2).collect()
}

pub fn calculate_ipinfo_size(info: &IpInfo) -> usize {
    let mut size = std::mem::size_of::<IpInfo>();

    size += info.ip.capacity();
    size += info.addr.capacity();

    if let Some(asn) = &info.asn {
        size += std::mem::size_of::<crate::models::AsnInfo>();
        size += asn.name.capacity();
        size += asn.info.capacity();
    }

    if let Some(_location) = &info.location {
        size += std::mem::size_of::<crate::models::Location>();
    }

    if let Some(country) = &info.country {
        size += std::mem::size_of::<crate::models::CountryInfo>();
        size += country.code.capacity();
        size += country.name.capacity();
    }

    if let Some(registered_country) = &info.registered_country {
        size += std::mem::size_of::<crate::models::CountryInfo>();
        size += registered_country.code.capacity();
        size += registered_country.name.capacity();
    }

    if let Some(regions) = &info.regions {
        size += std::mem::size_of::<Vec<String>>();
        for region in regions {
            size += region.capacity();
        }
    }

    if let Some(regions_short) = &info.regions_short {
        size += std::mem::size_of::<Vec<String>>();
        for region in regions_short {
            size += region.capacity();
        }
    }

    if let Some(r#type) = &info.r#type {
        size += r#type.capacity();
    }

    size
}

pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.octets()[0] == 10 || // 10.0.0.0/8
            (ip.octets()[0] == 172 && (ip.octets()[1] >= 16 && ip.octets()[1] <= 31)) || // 172.16.0.0/12
            (ip.octets()[0] == 192 && ip.octets()[1] == 168) || // 192.168.0.0/16
            ip.octets()[0] == 127 || // 127.0.0.0/8
            ip.is_loopback() ||
            ip.is_link_local() ||
            ip.is_broadcast() ||
            ip.is_documentation() ||
            ip.is_unspecified()
        }
        IpAddr::V6(ip) => {
            ip.is_loopback() ||
            ip.is_unspecified() ||
            ip.segments()[0] & 0xffc0 == 0xfe80 || // fe80::/10
            ip.segments()[0] & 0xfe00 == 0xfc00 // fc00::/7
        }
    }
} 