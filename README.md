# IP 地理位置服务

[English](README_EN.md) | 简体中文

基于 Rust 构建的高性能 IP 地理位置服务，为 IP 地址和域名提供详细的地理和网络信息。

## 特性

- 🌍 IP 地理位置查询，提供详细信息
- 🏙️ 支持中国和国际地区位置查询
- 🔄 ASN（自治系统编号）信息
- 🌐 支持 IPv4 和 IPv6 地址
- 🚀 基于高性能 Axum web 框架
- 🗺️ 使用数据库：GeoCN.mmdb、GeoLite2-City.mmdb、GeoLite2-ASN.mmdb
- 🌐 RESTful API 接口
- 🔍 自动域名解析（支持 A 和 AAAA 记录）
- ⚡ 高性能：每秒可处理数万次请求
- 🐳 Docker 支持，便于部署

## 环境要求

- Rust 2021 edition 或更高版本

## 配置

### 环境变量

- `HOST`：服务监听地址（默认：0.0.0.0）
- `PORT`：服务端口（默认：8080）

## 使用方法

### 启动服务

基本启动：
```bash
./target/release/ipgeo
```

自定义端口：
```bash
PORT=3000 ./target/release/ipgeo
```

### API 接口

所有 API 接口都返回 JSON 格式的响应。支持 IPv4、IPv6 地址和域名查询，自动解析域名的 A 和 AAAA 记录。

#### 1. 直接查询
```http
GET /{ip或域名}
```
最简单的查询方式，直接在路径中传入 IP 或域名。

示例：
```bash

# IPv4 查询
curl "http://localhost:8080/8.8.8.8"

# IPv6 查询
curl "http://localhost:8080/2001:4860:4860::8888"

# 域名查询
curl "http://localhost:8080/google.com"
```

#### 2. API 路径查询
```http
GET /api/{ip或域名}
```
带 API 前缀的标准 RESTful 接口。

示例：
```bash
# IPv4 查询
curl "http://localhost:8080/api/1.1.1.1"

# 域名查询（自动解析）
curl "http://localhost:8080/api/github.com"
```

#### 3. 查询参数方式
```http
GET /api?host={ip或域名}
```
使用查询参数的方式，适合需要 URL 编码的场景。

示例：
```bash
# IPv4 查询
curl "http://localhost:8080/api?host=1.1.1.1"

# IPv6 查询（URL 编码）
curl "http://localhost:8080/api?host=2001%3A4860%3A4860%3A%3A8888"

# 域名查询
curl "http://localhost:8080/api?host=cloudflare.com"
```

#### 4. 获取当前客户端信息
```http
GET /
```
获取发起请求的客户端 IP 地址信息。

示例：
```bash
curl "http://localhost:8080/"
```

### 响应示例

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

## Docker 部署

### 使用预构建镜像

最简单的方式是使用预构建的 Docker 镜像，数据库文件会自动更新：

```bash
docker run -d \
  --name ipgeo \
  -p 8080:8080 \
  tachy0nx/rust-ipgeo:latest
```

参数说明：
- `-d`: 后台运行容器
- `-p 8080:8080`: 端口映射，格式为 `主机端口:容器端口`

验证和管理：
```bash
# 验证服务
curl http://localhost:8080/1.1.1.1

# 容器管理
docker logs ipgeo    # 查看日志
docker stop ipgeo    # 停止服务
docker start ipgeo   # 启动服务
docker restart ipgeo # 重启服务
```

自定义配置：
```bash
# 修改端口和监听地址
docker run -d \
  --name ipgeo \
  -p 8080:8080 \
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

## 性能测试

使用 oha 工具进行压力测试，测试命令：
```bash
oha -c 2000 -z 30s --urls-from-file urls.txt  # urls.txt 包含随机生成IP 地址列表
```

测试结果如下：

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
  [200] 1662383 responses

Error distribution:
  [670] aborted due to deadline
```

## 性能优化建议

1. 使用生产环境构建：
```bash
cargo build --release
```

2. 调整系统限制：
```bash
# /etc/security/limits.conf
* soft nofile 65535
* hard nofile 65535
```

3. 使用负载均衡器（如 Nginx）进行反向代理

## 开源协议

本项目采用 GNU 通用公共许可证第3版 (GPL-3.0) 开源。详情请参阅 [LICENSE](LICENSE) 文件。

## 贡献

欢迎提交贡献！请随时向 [GitHub 仓库](https://github.com/upteka/ipgeo-api-rust) 提交 Pull Request。

## 问题反馈

如果您发现任何问题或有改进建议，请在 [GitHub Issues](https://github.com/upteka/ipgeo-api-rust/issues) 页面提交。 