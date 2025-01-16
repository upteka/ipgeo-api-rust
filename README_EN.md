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

## Configuration

### Environment Variables

- `HOST`: Service listening address (default: 0.0.0.0)
- `PORT`: Service port (default: 8080)

## Usage

### Starting the Service

Basic start:
```bash
./target/release/ipgeo
```

Custom port:
```bash
PORT=3000 ./target/release/ipgeo
```

### API Endpoints

All API endpoints return responses in JSON format. Supports IPv4, IPv6 addresses and domain names, with automatic resolution of A and AAAA records.

#### 1. Direct Query
```http
GET /{ip or domain}
```
The simplest query method, directly passing IP or domain in the path.

Examples:
```bash
# IPv4 query
curl "http://localhost:8080/8.8.8.8"

# IPv6 query
curl "http://localhost:8080/2001:4860:4860::8888"

# Domain query
curl "http://localhost:8080/google.com"
```

#### 2. API Path Query
```http
GET /api/{ip or domain}
```
Standard RESTful interface with API prefix.

Examples:
```bash
# IPv4 query
curl "http://localhost:8080/api/1.1.1.1"

# Domain query (auto-resolution)
curl "http://localhost:8080/api/github.com"
```

#### 3. Query Parameter Method
```http
GET /api?host={ip or domain}
```
Using query parameters, suitable for scenarios requiring URL encoding.

Examples:
```bash
# IPv4 query
curl "http://localhost:8080/api?host=1.1.1.1"

# IPv6 query (URL encoded)
curl "http://localhost:8080/api?host=2001%3A4860%3A4860%3A%3A8888"

# Domain query
curl "http://localhost:8080/api?host=cloudflare.com"
```

#### 4. Get Current Client Information
```http
GET /
```
Get information about the client IP address making the request.

Example:
```bash
curl "http://localhost:8080/"
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


## Docker Deployment

### Using Pre-built Image

The easiest way is to use the pre-built Docker image, database files will be updated automatically:

```bash
docker run -d \
  --name ipgeo \
  -p 8080:8080 \
  tachy0nx/rust-ipgeo:latest
```

Parameter explanation:
- `-d`: Run container in background
- `-p 8080:8080`: Port mapping, format is `host_port:container_port`

Verify and manage:
```bash
# Verify service
curl http://localhost:8080/1.1.1.1

# Container management
docker logs ipgeo    # View logs
docker stop ipgeo    # Stop service
docker start ipgeo   # Start service
docker restart ipgeo # Restart service
```

Custom configuration:
```bash
# Modify port and listening address
docker run -d \
  --name ipgeo \
  -p 3000:8080 \
  -e HOST=127.0.0.1 \
  -e PORT=8080 \
  tachy0nx/rust-ipgeo:latest
```

### Docker Compose

```yaml
version: '3'
services:
  ipgeo:
    image: tachy0nx/rust-ipgeo:latest
    ports:
      - "8080:8080"
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

## Performance Testing

Using the oha tool for stress testing, test command:
```bash
oha -c 2000 -z 30s --urls-from-file urls.txt  # urls.txt contains a list of randomly generated IP addresses
```

Test results:

```
Summary:
  Success rate: 100.00%
  Total:        30.0589 secs
  Slowest:      1.1063 secs
  Fastest:      0.0003 secs
  Average:      0.0361 secs
  Requests/sec: 55326.4230

  Total data:   390.71 MiB
  Size/request: 246 B
  Size/sec:     13.00 MiB

Response time histogram:
  0.000 [1]       |
  0.111 [1655785] |■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
  0.221 [5059]    |
  0.332 [0]       |
  0.443 [0]       |
  0.553 [737]     |
  0.664 [258]     |
  0.775 [0]       |
  0.885 [0]       |
  0.996 [0]       |
  1.106 [543]     |

Response time distribution:
  10.00% in 0.0144 secs
  25.00% in 0.0218 secs
  50.00% in 0.0316 secs
  75.00% in 0.0454 secs
  90.00% in 0.0620 secs
  95.00% in 0.0733 secs
  99.00% in 0.0974 secs
  99.90% in 0.1513 secs
  99.99% in 1.0590 secs


Details (average, fastest, slowest):
  DNS+dialup:   0.5404 secs, 0.0007 secs, 1.0353 secs
  DNS-lookup:   0.0000 secs, 0.0000 secs, 0.0001 secs

Status code distribution:
```

## License

This project is licensed under the GNU General Public License v3.0 (GPL-3.0). See the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Feel free to submit Pull Requests to the [GitHub repository](https://github.com/upteka/ipgeo-api-rust).

## Issue Reporting

If you find any issues or have suggestions for improvements, please submit them on the [GitHub Issues](https://github.com/upteka/ipgeo-api-rust/issues) page. 