#!/bin/sh

# 使用环境变量 MMDB_PATH，如果未设置则使用当前目录
MMDB_PATH=${MMDB_PATH:-.}

update_databases() {
    echo "正在更新 GeoLite2-City.mmdb..."
    curl -L -o "${MMDB_PATH}/GeoLite2-City.mmdb" "https://github.com/P3TERX/GeoLite.mmdb/raw/download/GeoLite2-City.mmdb" || return 1

    echo "正在更新 GeoLite2-ASN.mmdb..."
    curl -L -o "${MMDB_PATH}/GeoLite2-ASN.mmdb" "https://github.com/P3TERX/GeoLite.mmdb/raw/download/GeoLite2-ASN.mmdb" || return 1

    echo "正在更新 GeoCN.mmdb..."
    curl -L -o "${MMDB_PATH}/GeoCN.mmdb" "http://github.com/ljxi/GeoCN/releases/download/Latest/GeoCN.mmdb" || return 1

    return 0
}

# 首次运行时更新数据库
update_databases || exit 1

# 启动后台更新进程
(
    while true; do
        sleep 86400  # 24小时
        update_databases
    done
) &

# 启动主程序并保持前台运行
exec ./ipgeo "$@" 