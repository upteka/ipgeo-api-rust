use std::sync::OnceLock;
use dashmap::DashMap;
use serde_json::Value;

trait IntoString {
    fn into_string(self) -> String;
}

impl IntoString for Box<str> {
    fn into_string(self) -> String {
        self.into()
    }
}

// ASN类型枚举
#[derive(Clone, PartialEq, Eq)]
pub enum AsnType {
    Type(Box<str>),
    Other,
}

impl AsnType {
    fn from_str(s: &str) -> Self {
        if s.is_empty() {
            Self::Other
        } else {
            Self::Type(s.into())
        }
    }
}

// ASN缓存优化结构
#[derive(Clone)]
pub struct AsnInfo {
    pub name: Box<str>,
    pub type_info: AsnType,
}

// 关键词信息结构
#[derive(Clone)]
pub struct KeywordInfo {
    pub name: Box<str>,
    pub type_info: AsnType,
}

// 关键词缓存优化结构
pub struct KeywordCache {
    isp_map: DashMap<Box<str>, KeywordInfo>,
    org_map: DashMap<Box<str>, KeywordInfo>,
}

impl Default for KeywordCache {
    fn default() -> Self {
        Self {
            isp_map: DashMap::with_capacity(1000),
            org_map: DashMap::with_capacity(1000),
        }
    }
}

// 全局缓存管理器
pub struct CacheManager {
    asn_cache: DashMap<u32, AsnInfo>,
    keyword_cache: KeywordCache,
}

// 全局单例
static CACHE_MANAGER: OnceLock<CacheManager> = OnceLock::new();

impl CacheManager {
    pub fn global() -> &'static CacheManager {
        CACHE_MANAGER.get_or_init(|| {
            CacheManager {
                asn_cache: DashMap::with_capacity(1000),
                keyword_cache: KeywordCache::default(),
            }
        })
    }

    // ASN缓存方法
    pub fn get_asn_info(&self, asn: u32) -> Option<(Box<str>, AsnType)> {
        self.asn_cache.get(&asn)
            .map(|info| (info.name.clone(), info.type_info.clone()))
    }

    // 关键词缓存方法
    pub fn get_keyword_info(&self, keyword: &str) -> Option<(Box<str>, AsnType)> {
        self.keyword_cache.isp_map
            .get(keyword)
            .or_else(|| self.keyword_cache.org_map.get(keyword))
            .map(|info| (info.name.clone(), info.type_info.clone()))
    }

    // 初始化ASN数据
    pub fn init_asn_data(&self, data: &Value) {
        if let Some(asn_info) = data.get("asn_info").and_then(Value::as_object) {
            let expected_size = asn_info.len();
            let mut keyword_buffer = Vec::with_capacity(expected_size * 2);

            for (asn_str, info) in asn_info {
                if let (Ok(asn), Some(name), Some(type_str)) = (
                    asn_str.parse::<u32>(),
                    info.get("name").and_then(Value::as_str),
                    info.get("type").and_then(Value::as_str)
                ) {
                    let type_info = AsnType::from_str(type_str);
                    let name: Box<str> = name.into();
                    
                    let asn_info = AsnInfo {
                        name: name.clone(),
                        type_info: type_info.clone(),
                    };
                    self.asn_cache.insert(asn, asn_info);

                    // 收集关键词信息
                    if let Some(keywords) = info.get("keywords").and_then(Value::as_array) {
                        keyword_buffer.clear();
                        keyword_buffer.extend(
                            keywords.iter()
                                .filter_map(|k| k.as_str())
                                .map(|k| {
                                    let keyword: Box<str> = k.into();
                                    (keyword, KeywordInfo {
                                        name: name.clone(),
                                        type_info: type_info.clone(),
                                    })
                                })
                        );

                        // 批量插入关键词
                        for (keyword, info) in keyword_buffer.drain(..) {
                            if let AsnType::Type(_) = &info.type_info {
                                self.keyword_cache.isp_map.insert(keyword, info);
                            }
                        }
                    }
                }
            }
        }
    }
} 