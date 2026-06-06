use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub web_origin: String,
    pub database_url: String,
    pub s3_endpoint: String,
    pub s3_bucket_public: String,
    pub s3_bucket_private: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            host: env::var("SPARK_API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("SPARK_API_PORT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(8787),
            web_origin: env::var("SPARK_WEB_ORIGIN")
                .unwrap_or_else(|_| "http://127.0.0.1:5173".to_string()),
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://spark:spark_dev_password@127.0.0.1:5432/spark".to_string()
            }),
            s3_endpoint: env::var("S3_ENDPOINT")
                .unwrap_or_else(|_| "http://127.0.0.1:9000".to_string()),
            s3_bucket_public: env::var("S3_BUCKET_PUBLIC")
                .unwrap_or_else(|_| "spark-public".to_string()),
            s3_bucket_private: env::var("S3_BUCKET_PRIVATE")
                .unwrap_or_else(|_| "spark-private".to_string()),
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
