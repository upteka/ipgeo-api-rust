# IP 地理位置服务

[English](README_EN.md) | 简体中文

基于 Rust 构建的高性能 IP 地理位置服务，为 IP 地址和域名提供详细的地理和网络信息。

## 特性

- 🌍 IP 地理位置查询，提供详细信息
- 🏙️ 支持中国和国际地区位置查询
- 🔄 ASN（自治系统编号）信息
- 🌐 支持 IPv4 和 IPv6 地址
- 🚀 基于高性能 Axum web 框架
- 🗺️ 多数据库支持（GeoCN.mmdb、GeoLite2-City.mmdb、GeoLite2-ASN.mmdb）
- 🌐 RESTful API 接口
- 🔍 自动域名解析（支持 A 和 AAAA 记录）
- ⚡ 高性能：每秒可处理数万次请求
- 🐳 Docker 支持，便于部署

## 环境要求

- Rust 2021 edition 或更高版本
- MaxMind GeoIP2 数据库文件
  - GeoCN.mmdb（中国精确位置数据）
  - GeoLite2-City.mmdb（全球城市数据）
  - GeoLite2-ASN.mmdb（ASN 信息数据）

## 安装

1. 克隆仓库：
```bash
git clone https://github.com/upteka/ipgeo-api-rust.git
cd ipgeo-api-rust
```

2. 构建项目：
```bash
cargo build --release
```

## 配置

### 环境变量

- `MMDB_PATH`：MaxMind 数据库文件目录路径（默认：当前目录）
- `HOST`：服务监听地址（默认：0.0.0.0）
- `PORT`：服务端口（默认：8080）

### 数据库文件

请将以下数据库文件放置在 `MMDB_PATH` 指定的目录中：
- `GeoCN.mmdb`
- `GeoLite2-City.mmdb`
- `GeoLite2-ASN.mmdb`

## 使用方法

### 启动服务

基本启动：
```bash
./target/release/ipgeo
```

指定数据库路径：
```bash
MMDB_PATH=/path/to/mmdb ./target/release/ipgeo
```

自定义端口：
```bash
PORT=3000 ./target/release/ipgeo
```

### API 接口

所有 API 接口都返回 JSON 格式的响应。

1. **直接查询**
   ```
   GET /{ip或域名}
   示例：GET /8.8.8.8
   ```

2. **API 路径查询**
   ```
   GET /api/{ip或域名}
   示例：GET /api/google.com
   ```

3. **查询参数方式**
   ```
   GET /api?host={ip或域名}
   示例：GET /api?host=1.1.1.1
   ```

4. **获取当前客户端信息**
   ```
   GET /
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

## 项目依赖

主要依赖包括：
- `axum 0.7` - Web 框架
- `tokio 1.x` - 异步运行时
- `maxminddb 0.24` - MaxMind DB 读取器
- `serde 1.x` - 序列化框架
- `tower 0.4` - HTTP 服务组件
- `serde_json 1.x` - JSON 处理

## Docker 部署

1. 构建镜像：
```bash
docker build -t ipgeo .
```

2. 运行容器：
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