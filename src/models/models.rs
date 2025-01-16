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
        let (status, message) = match self {
            IpGeoError::InvalidIp(ip) => (
                axum::http::StatusCode::BAD_REQUEST,
                format!("Invalid IP address: {}", ip),
            ),
            IpGeoError::ResolveError => (
                axum::http::StatusCode::BAD_REQUEST,
                "Failed to resolve hostname".to_string(),
            ),
            IpGeoError::IoError(err) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("IO error: {}", err),
            ),
            IpGeoError::ParseError(err) => (
                axum::http::StatusCode::BAD_REQUEST,
                format!("IP parse error: {}", err),
            ),
        };
        
        axum::response::Response::builder()
            .status(status)
            .header("Content-Type", "text/plain")
            .body(message.into())
            .unwrap()
    }
} 