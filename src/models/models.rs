use serde::Serialize;
use std::net::AddrParseError;
use thiserror::Error;

#[derive(Debug, Serialize, Clone)]
pub struct AsnInfo {
    pub number: u32,
    pub name: String,
    pub info: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Location {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CountryInfo {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct IpInfo {
    pub ip: String,
    #[serde(rename = "as", skip_serializing_if = "Option::is_none")]
    pub asn: Option<AsnInfo>,
    pub addr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<CountryInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_country: Option<CountryInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regions_short: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct IpResponse {
    pub host: String,
    pub ips: Vec<IpInfo>,
}

#[derive(Debug, Error)]
pub enum IpGeoError {
    #[error("Invalid IP address: {0}")]
    InvalidIp(String),
    #[error("Failed to resolve host")]
    ResolveError,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("IP parse error: {0}")]
    ParseError(#[from] AddrParseError),
}

impl axum::response::IntoResponse for IpGeoError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_type, message) = match self {
            IpGeoError::InvalidIp(ip) => (
                axum::http::StatusCode::BAD_REQUEST,
                "INVALID_IP",
                format!("无效的IP地址: {}", ip),
            ),
            IpGeoError::ResolveError => (
                axum::http::StatusCode::BAD_REQUEST,
                "RESOLVE_ERROR",
                "无法解析域名".to_string(),
            ),
            IpGeoError::IoError(err) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "IO_ERROR",
                format!("IO错误: {}", err),
            ),
            IpGeoError::ParseError(err) => (
                axum::http::StatusCode::BAD_REQUEST,
                "PARSE_ERROR",
                format!("IP解析错误: {}", err),
            ),
        };
        
        let body = serde_json::json!({
            "code": status.as_u16(),
            "error": error_type,
            "message": message
        });
        
        (
            status,
            [(axum::http::header::CONTENT_TYPE, "application/json; charset=utf-8")],
            axum::Json(body)
        ).into_response()
    }
} 