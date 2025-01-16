# IP Geo Service

English | [简体中文](README.md)

A high-performance IP geolocation service built with Rust, providing detailed geographical and network information for IP addresses and domain names.

## Features

- 🌍 IP Geolocation lookup with detailed information
- 🏙️ Support for both Chinese and international locations
- 🔄 ASN (Autonomous System Number) information
- 🌐 Support for both IPv4 and IPv6 addresses
- 🚀 High-performance Axum web framework
- 🗺️ Multiple database support (GeoCN.mmdb, GeoLite2-City.mmdb, GeoLite2-ASN.mmdb)
- 🌐 RESTful API endpoints
- 🔍 Automatic domain name resolution (A and AAAA records)

## Prerequisites

- Rust 2021 edition or later
- MaxMind GeoIP2 databases (GeoCN.mmdb, GeoLite2-City.mmdb, GeoLite2-ASN.mmdb)

## Installation

1. Clone the repository:
```bash
git clone https://github.com/upteka/ipgeo-api-rust.git
cd ipgeo-api-rust
```

2. Build the project:
```bash
cargo build --release
```

## Configuration

The service looks for MaxMind database files in the directory specified by the `MMDB_PATH` environment variable. If not set, it defaults to the current directory.

Required database files:
- `GeoCN.mmdb` - Chinese locations database
- `GeoLite2-City.mmdb` - Global city database
- `GeoLite2-ASN.mmdb` - ASN information database

## Usage

1. Start the service:
```bash
MMDB_PATH=/path/to/mmdb ./target/release/ipgeo
```

2. The service provides the following API endpoints:

- Query by query parameter:
  ```
  GET /?host={ip_or_domain}
  ```

- Query by path parameter:
  ```
  GET /{ip_or_domain}
  ```

For domain names, the service will automatically:
1. Resolve both A (IPv4) and AAAA (IPv6) records
2. Look up geolocation information for each resolved IP address
3. Return combined results in a single response

### Example Response

```json
{
  "host": "example.com",
  "ips": [
    {
      "ip": "93.184.216.34",
      "as": {
        "number": 15133,
        "name": "EdgeCast Networks",
        "info": ""
      },
      "addr": "93.184.216.0/24",
      "location": {
        "latitude": 34.0655,
        "longitude": -118.2389
      },
      "country": {
        "code": "US",
        "name": "United States"
      },
      "registered_country": {
        "code": "US",
        "name": "United States"
      },
      "regions": ["California", "Los Angeles"],
      "regions_short": ["CA", "LA"]
    }
  ]
}
```

## Dependencies

- `axum` - Web framework
- `tokio` - Async runtime
- `maxminddb` - MaxMind DB reader
- `serde` - Serialization framework
- `serde_json` - JSON support

## Docker Support

The project includes Docker support for easy deployment. Build and run using:

```bash
docker build -t ipgeo .
docker run -p 3000:3000 -v /path/to/mmdb:/mmdb -e MMDB_PATH=/mmdb ipgeo
```

## License

This project is licensed under the GNU General Public License v3.0 (GPL-3.0). See the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request to the [GitHub repository](https://github.com/upteka/ipgeo-api-rust). 