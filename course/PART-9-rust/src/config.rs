use anyhow::Context;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub app_url: String,
    pub node_env: String,

    pub database_url: String,
    pub db_pool_min: u32,
    pub db_pool_max: u32,

    pub jwt_access_secret: String,
    pub jwt_refresh_secret: String,
    pub jwt_access_expires_in_secs: u64,
    pub jwt_refresh_expires_in_secs: u64,
    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,

    pub cors_origins: Vec<String>,

    pub feature_trials_enabled: bool,
    pub trial_days: i64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3000".into())
                .parse()
                .context("PORT must be a valid port number")?,

            app_url: std::env::var("APP_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),

            node_env: std::env::var("NODE_ENV")
                .unwrap_or_else(|_| "development".into()),

            database_url: std::env::var("DATABASE_URL")
                .context("DATABASE_URL must be set")?,

            db_pool_min: std::env::var("DATABASE_POOL_MIN")
                .unwrap_or_else(|_| "2".into())
                .parse()
                .context("DATABASE_POOL_MIN must be a number")?,

            db_pool_max: std::env::var("DATABASE_POOL_MAX")
                .unwrap_or_else(|_| "10".into())
                .parse()
                .context("DATABASE_POOL_MAX must be a number")?,

            jwt_access_secret: std::env::var("JWT_ACCESS_SECRET")
                .context("JWT_ACCESS_SECRET must be set")?,

            jwt_refresh_secret: std::env::var("JWT_REFRESH_SECRET")
                .context("JWT_REFRESH_SECRET must be set")?,

            // Parse "15m", "1h", "30d" into seconds
            jwt_access_expires_in_secs: parse_duration_to_secs(
                &std::env::var("JWT_ACCESS_EXPIRES_IN").unwrap_or_else(|_| "900".into()),
            )?,

            jwt_refresh_expires_in_secs: parse_duration_to_secs(
                &std::env::var("JWT_REFRESH_EXPIRES_IN").unwrap_or_else(|_| "2592000".into()),
            )?,

            stripe_secret_key: std::env::var("STRIPE_SECRET_KEY")
                .context("STRIPE_SECRET_KEY must be set")?,

            stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET")
                .context("STRIPE_WEBHOOK_SECRET must be set")?,

            cors_origins: std::env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:5173".into())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),

            feature_trials_enabled: std::env::var("FEATURE_TRIALS_ENABLED")
                .unwrap_or_else(|_| "true".into())
                .parse()
                .unwrap_or(true),

            trial_days: std::env::var("TRIAL_DAYS")
                .unwrap_or_else(|_| "14".into())
                .parse()
                .context("TRIAL_DAYS must be a number")?,
        })
    }

    pub fn is_production(&self) -> bool {
        self.node_env == "production"
    }
}

fn parse_duration_to_secs(s: &str) -> anyhow::Result<u64> {
    // Try as raw seconds first
    if let Ok(n) = s.parse::<u64>() {
        return Ok(n);
    }
    // Parse suffixes: s, m, h, d
    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: u64 = num_str.parse().context("Invalid duration number")?;
    match unit {
        "s" => Ok(num),
        "m" => Ok(num * 60),
        "h" => Ok(num * 3600),
        "d" => Ok(num * 86400),
        _ => anyhow::bail!("Unknown duration unit: {}", unit),
    }
}
