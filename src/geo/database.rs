use std::path::{Path, PathBuf};
use tokio::time::{Duration, interval};
use std::time::SystemTime;
use tracing::info;

const UPDATE_INTERVAL: Duration = Duration::from_secs(86400); // 24小时

pub struct DatabaseManager {
    data_dir: PathBuf,
}

struct DatabaseUrl {
    name: &'static str,
    url: &'static str,
}

const DATABASE_URLS: [DatabaseUrl; 3] = [
    DatabaseUrl {
        name: "GeoLite2-City.mmdb",
        url: "https://github.com/P3TERX/GeoLite.mmdb/raw/download/GeoLite2-City.mmdb",
    },
    DatabaseUrl {
        name: "GeoLite2-ASN.mmdb",
        url: "https://github.com/P3TERX/GeoLite.mmdb/raw/download/GeoLite2-ASN.mmdb",
    },
    DatabaseUrl {
        name: "GeoCN.mmdb",
        url: "http://github.com/ljxi/GeoCN/releases/download/Latest/GeoCN.mmdb",
    },
];

impl DatabaseManager {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    async fn download_database(&self, url: &str, path: &Path) -> std::io::Result<()> {
        info!("Downloading database from {}", url);
        let response = reqwest::get(url).await.map_err(|e| std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to download: {}", e)
        ))?;
        
        let bytes = response.bytes().await.map_err(|e| std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get bytes: {}", e)
        ))?;
        
        tokio::fs::write(path, bytes).await?;
        info!("Successfully downloaded database to {:?}", path);
        Ok(())
    }

    async fn copy_asn_info(&self) -> std::io::Result<()> {
        let target_path = self.data_dir.join("asn_info.json");
        if !target_path.exists() {
            info!("Copying asn_info.json to data directory");
            // 首先尝试从源代码目录复制
            let source_paths = [
                "src/asn_info.json",
                "../src/asn_info.json",
                "/usr/local/share/ipgeo/asn_info.json",
            ];

            for source_path in source_paths {
                if let Ok(content) = tokio::fs::read_to_string(source_path).await {
                    tokio::fs::write(&target_path, content).await?;
                    info!("Successfully copied asn_info.json to {:?}", target_path);
                    return Ok(());
                }
            }

            // 如果找不到源文件，创建一个空的JSON对象
            info!("Could not find asn_info.json source, creating empty file");
            tokio::fs::write(&target_path, "{}").await?;
        }
        Ok(())
    }

    pub async fn update_databases(&self) -> std::io::Result<()> {
        if !self.data_dir.exists() {
            tokio::fs::create_dir_all(&self.data_dir).await?;
        }

        // 确保 asn_info.json 存在
        self.copy_asn_info().await?;

        for db in &DATABASE_URLS {
            let db_path = self.data_dir.join(db.name);
            let should_update = if db_path.exists() {
                if let Ok(metadata) = tokio::fs::metadata(&db_path).await {
                    if let Ok(modified) = metadata.modified() {
                        let elapsed = SystemTime::now().duration_since(modified)
                            .unwrap_or(Duration::from_secs(0));
                        elapsed > UPDATE_INTERVAL
                    } else {
                        true
                    }
                } else {
                    true
                }
            } else {
                true
            };

            if should_update {
                if let Err(e) = self.download_database(db.url, &db_path).await {
                    info!("Failed to download {}: {}", db.name, e);
                }
            }
        }

        Ok(())
    }

    pub fn get_data_file_path(&self, filename: &str) -> PathBuf {
        let data_paths = [
            self.data_dir.as_path(),
            Path::new("../data"),
            Path::new("/usr/local/share/ipgeo"),
        ];
        
        for base_path in data_paths.iter() {
            let file_path = base_path.join(filename);
            if file_path.exists() {
                return file_path;
            }
        }
        
        self.data_dir.join(filename)
    }

    pub async fn start_auto_update(&self) {
        let data_dir = self.data_dir.clone();
        let manager = DatabaseManager::new(data_dir);
        
        tokio::spawn(async move {
            let mut interval = interval(UPDATE_INTERVAL);
            loop {
                interval.tick().await;
                info!("Starting scheduled database update");
                if let Err(e) = manager.update_databases().await {
                    info!("Failed to update databases: {}", e);
                }
            }
        });
    }
} 