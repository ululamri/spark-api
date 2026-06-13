use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub app_env: String,
    pub host: String,
    pub port: u16,
    pub web_origin: String,
    pub database_url: String,
    pub database_max_connections: u32,
    pub s3_endpoint: String,
    pub s3_bucket_public: String,
    pub s3_bucket_private: String,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
    pub s3_region: String,
    pub s3_presign_expires_seconds: i64,
    pub session_cookie_name: String,
    pub session_ttl_days: i64,
    pub cookie_secure: bool,
    pub admin_token: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let app_env = env_or("APP_ENV", "development");
        let cookie_secure_default = app_env == "production";

        Self {
            app_env,
            host: env_first(&["SPARK_API_HOST", "APP_HOST"], "127.0.0.1"),
            port: env_first(&["SPARK_API_PORT", "APP_PORT"], "8787")
                .parse()
                .unwrap_or(8787),
            web_origin: env_first(&["SPARK_WEB_ORIGIN", "WEB_ORIGIN"], "http://127.0.0.1:5173"),
            database_url: env_or(
                "DATABASE_URL",
                "postgres://spark:spark_dev_password@127.0.0.1:5432/spark",
            ),
            database_max_connections: env_or("DATABASE_MAX_CONNECTIONS", "5").parse().unwrap_or(5),
            s3_endpoint: env_or("S3_ENDPOINT", "http://127.0.0.1:9000"),
            s3_bucket_public: env_or("S3_BUCKET_PUBLIC", "spark-public"),
            s3_bucket_private: env_or("S3_BUCKET_PRIVATE", "spark-private"),
            s3_access_key: env_optional(&["S3_ACCESS_KEY", "MINIO_ROOT_USER", "MINIO_ACCESS_KEY"]),
            s3_secret_key: env_optional(&["S3_SECRET_KEY", "MINIO_ROOT_PASSWORD", "MINIO_SECRET_KEY"]),
            s3_region: env_or("S3_REGION", "us-east-1"),
            s3_presign_expires_seconds: env_or("S3_PRESIGN_EXPIRES_SECONDS", "900")
                .parse()
                .unwrap_or(900),
            session_cookie_name: env_or("SPARK_SESSION_COOKIE", "spark_session"),
            session_ttl_days: env_or("SPARK_SESSION_TTL_DAYS", "14").parse().unwrap_or(14),
            cookie_secure: env::var("SPARK_COOKIE_SECURE")
                .ok()
                .and_then(|value| parse_bool(&value))
                .unwrap_or(cookie_secure_default),
            admin_token: env::var("KARYRA_ADMIN_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty()),
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_first(keys: &[&str], default: &str) -> String {
    for key in keys {
        if let Ok(value) = env::var(key) {
            return value;
        }
    }

    default.to_string()
}

fn env_optional(keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Ok(value) = env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    None
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "on" => Some(true),
        "0" | "false" | "no" | "n" | "off" => Some(false),
        _ => None,
    }
}
