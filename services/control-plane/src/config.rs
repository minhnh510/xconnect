use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Clone, Debug)]
pub struct Config {
    pub listen_addr: SocketAddr,
    pub jwt_secret: String,
    pub access_token_ttl_seconds: i64,
    pub refresh_token_ttl_seconds: i64,
    pub turn_secret: Option<String>,
    pub turn_uris: Vec<String>,
    pub tls_pin_mode: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let host =
            std::env::var("HOST").unwrap_or_else(|_| IpAddr::V4(Ipv4Addr::UNSPECIFIED).to_string());
        let port = std::env::var("PORT")
            .ok()
            .and_then(|raw| raw.parse::<u16>().ok())
            .unwrap_or(8080);

        let ip = host.parse::<IpAddr>()?;
        let listen_addr = SocketAddr::new(ip, port);

        let jwt_secret =
            std::env::var("JWT_SECRET").unwrap_or_else(|_| "change-me-in-prod".to_string());

        let access_token_ttl_seconds = std::env::var("ACCESS_TOKEN_TTL_SECONDS")
            .ok()
            .and_then(|raw| raw.parse().ok())
            .unwrap_or(900);
        let refresh_token_ttl_seconds = std::env::var("REFRESH_TOKEN_TTL_SECONDS")
            .ok()
            .and_then(|raw| raw.parse().ok())
            .unwrap_or(60 * 60 * 24 * 14);

        let turn_secret = std::env::var("TURN_SECRET").ok();
        let turn_uris = std::env::var("TURN_URIS")
            .unwrap_or_default()
            .split(',')
            .filter(|v| !v.trim().is_empty())
            .map(|v| v.trim().to_string())
            .collect::<Vec<_>>();

        let tls_pin_mode = std::env::var("TLS_PIN_MODE").unwrap_or_else(|_| "disabled".to_string());

        Ok(Self {
            listen_addr,
            jwt_secret,
            access_token_ttl_seconds,
            refresh_token_ttl_seconds,
            turn_secret,
            turn_uris,
            tls_pin_mode,
        })
    }
}
