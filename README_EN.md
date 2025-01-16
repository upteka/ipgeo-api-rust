# IP Geolocation Service

English | [简体中文](README.md)

A high-performance IP geolocation service built with Rust, providing detailed geographic and network information for IP addresses and domain names.

## Features

- 🌍 IP geolocation lookup with detailed information
- 🏙️ Support for both Chinese and international location queries
- 🔄 ASN (Autonomous System Number) information
- 🌐 Support for both IPv4 and IPv6 addresses
- 🚀 Built on high-performance Axum web framework
- 🗺️ Multiple database support (GeoCN.mmdb, GeoLite2-City.mmdb, GeoLite2-ASN.mmdb)
- 🌐 RESTful API interface
- 🔍 Automatic domain resolution (supports A and AAAA records)
- ⚡ High performance: handles tens of thousands of requests per second
- 🐳 Docker support for easy deployment

## Requirements

- Rust 2021 edition or higher
- MaxMind GeoIP2 database files
  - GeoCN.mmdb (China precise location data)
  - GeoLite2-City.mmdb (Global city database)
  - GeoLite2-ASN.mmdb (ASN information)

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

### Environment Variables

- `MMDB_PATH`: Directory path for MaxMind database files (default: current directory)
- `HOST`: Service listening address (default: 0.0.0.0)
- `PORT`: Service port (default: 8080)

### Database Files

Place the following database files in the directory specified by `MMDB_PATH`:
- `GeoCN.mmdb`
- `GeoLite2-City.mmdb`
- `GeoLite2-ASN.mmdb`

## Usage

### Starting the Service

Basic start:
```bash
./target/release/ipgeo
```

Specify database path:
```bash
MMDB_PATH=/path/to/mmdb ./target/release/ipgeo
```

Custom port:
```bash
PORT=3000 ./target/release/ipgeo
```

### API Endpoints

All API endpoints return responses in JSON format.

1. **Direct Query**
   ```
   GET /{ip or domain}
   Example: GET /8.8.8.8
   ```

2. **API Path Query**
   ```
   GET /api/{ip or domain}
   Example: GET /api/google.com
   ```

3. **Query Parameter Method**
   ```
   GET /api?host={ip or domain}
   Example: GET /api?host=1.1.1.1
   ```

4. **Get Current Client Information**
   ```
   GET /
   ```

### Response Example

```json
{
    "ip": "223.5.5.5",
    "as": {
        "number": 37963,
        "name": "Hangzhou Alibaba Advertising Co.,Ltd.",
        "info": "阿里云"
    },
    "addr": "223.4.0.0/14",
    "location": {
        "latitude": 30.2943,
        "longitude": 120.1663
    },
    "country": {
        "code": "CN",
        "name": "中国"
    },
    "registered_country": {
        "code": "CN",
        "name": "中国"
    },
    "regions": [
        "浙江省",
        "杭州市"
    ],
    "regions_short": [
        "浙江",
        "杭州"
    ],
    "type": "数据中心"
}
```

## Dependencies

Main dependencies include:
- `axum 0.7` - Web framework
- `tokio 1.x` - Async runtime
- `maxminddb 0.24` - MaxMind DB reader
- `serde 1.x` - Serialization framework
- `tower 0.4` - HTTP service components
- `serde_json 1.x` - JSON processing

## Docker Deployment

1. Build the image:
```bash
docker build -t ipgeo .
```

2. Run the container:
```bash
docker run -d \
  --name ipgeo \
  -p 8080:8080 \
  -v /path/to/mmdb:/app/data \
  -e MMDB_PATH=/app/data \
  ipgeo
```

### Docker Compose

```yaml
version: '3'
services:
  ipgeo:
    build: .
    ports:
      - "8080:8080"
    volumes:
      - /path/to/mmdb:/app/data
    environment:
      - MMDB_PATH=/app/data
    restart: unless-stopped
```

## Performance Optimization Tips

1. Use production build:
```bash
cargo build --release
```

2. Adjust system limits:
```bash
# /etc/security/limits.conf
* soft nofile 65535
* hard nofile 65535
```

3. Use a load balancer (like Nginx) for reverse proxy

## License

This project is licensed under the GNU General Public License v3.0 (GPL-3.0). See the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Feel free to submit Pull Requests to the [GitHub repository](https://github.com/upteka/ipgeo-api-rust).

## Issue Reporting

If you find any issues or have suggestions for improvements, please submit them on the [GitHub Issues](https://github.com/upteka/ipgeo-api-rust/issues) page. 