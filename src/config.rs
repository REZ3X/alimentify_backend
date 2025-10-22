use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub mongodb: MongoConfig,
    pub redis: RedisConfig,
    pub google_oauth: GoogleOAuthConfig,
    pub brevo: BrevoConfig,
    pub jwt: JwtConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    #[allow(dead_code)]
    pub host: String,
    pub environment: Environment,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Production,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MongoConfig {
    pub uri: String,
    pub database_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BrevoConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub from_email: String,
    pub from_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub api_keys: Vec<String>,
    pub cors_enabled: bool,
    pub api_key_enabled: bool,
    pub allowed_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        dotenvy
            ::from_filename(".env.local")
            .or_else(|_| dotenvy::dotenv())
            .ok();

        let environment = env
            ::var("NODE_ENV")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase();

        let is_production = environment == "production";

        let api_keys_str = env::var("API_KEYS").unwrap_or_default();
        let api_keys: Vec<String> = api_keys_str
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string())
            .collect();

        let dev_origins = env::var("DEV_FRONTEND_ORIGIN").unwrap_or_default();
        let prod_origins = env::var("PRODUCTION_FRONTEND_ORIGIN").unwrap_or_default();

        let allowed_origins: Vec<String> = (if is_production { prod_origins } else { dev_origins })
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string())
            .collect();

        let config = Config {
            server: ServerConfig {
                port: env
                    ::var("PORT")
                    .unwrap_or_else(|_| "4000".to_string())
                    .parse()?,
                host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                environment: if is_production {
                    Environment::Production
                } else {
                    Environment::Development
                },
            },
            mongodb: MongoConfig {
                uri: env::var("MONGODB_URI").expect("MONGODB_URI must be set"),
                database_name: env
                    ::var("MONGODB_DATABASE")
                    .unwrap_or_else(|_| "alimentify".to_string()),
            },
            redis: RedisConfig {
                url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            },
            google_oauth: GoogleOAuthConfig {
                client_id: env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID must be set"),
                client_secret: env
                    ::var("GOOGLE_CLIENT_SECRET")
                    .expect("GOOGLE_CLIENT_SECRET must be set"),
                redirect_uri: env
                    ::var("GOOGLE_REDIRECT_URI")
                    .unwrap_or_else(|_|
                        "http://localhost:4000/api/auth/google/callback".to_string()
                    ),
            },
            brevo: BrevoConfig {
                smtp_host: env
                    ::var("BREVO_SMTP_HOST")
                    .unwrap_or_else(|_| "smtp-relay.brevo.com".to_string()),
                smtp_port: env
                    ::var("BREVO_SMTP_PORT")
                    .unwrap_or_else(|_| "587".to_string())
                    .parse()?,
                smtp_user: env::var("BREVO_SMTP_USER").expect("BREVO_SMTP_USER must be set"),
                smtp_pass: env::var("BREVO_SMTP_PASS").expect("BREVO_SMTP_PASS must be set"),
                from_email: env::var("BREVO_FROM_EMAIL").expect("BREVO_FROM_EMAIL must be set"),
                from_name: env::var("BREVO_FROM_NAME").unwrap_or_else(|_| "Alimentify".to_string()),
            },
            jwt: JwtConfig {
                secret: env
                    ::var("JWT_SECRET")
                    .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string()),
                expiration_hours: env
                    ::var("JWT_EXPIRATION_HOURS")
                    .unwrap_or_else(|_| "24".to_string())
                    .parse()?,
            },
            security: SecurityConfig {
                api_keys,
                cors_enabled: is_production,
                api_key_enabled: is_production,
                allowed_origins,
            },
        };

        Ok(config)
    }

    #[allow(dead_code)]
    pub fn is_development(&self) -> bool {
        self.server.environment == Environment::Development
    }

    #[allow(dead_code)]
    pub fn is_production(&self) -> bool {
        self.server.environment == Environment::Production
    }
}
