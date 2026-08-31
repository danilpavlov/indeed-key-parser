#[derive(Debug)]
pub struct Config {
    pub secret: String,
    pub bind_addr: String,
    pub db_path: String,
}

impl Config {
    pub fn from_getter(get: impl Fn(&str) -> Option<String>) -> Result<Config, String> {
        let secret = get("WEBHOOK_SECRET").ok_or("WEBHOOK_SECRET not set")?;
        if secret.is_empty() {
            return Err("WEBHOOK_SECRET is empty".into());
        }
        Ok(Config {
            secret,
            bind_addr: get("BIND_ADDR").unwrap_or_else(|| "0.0.0.0:8080".into()),
            db_path: get("DB_PATH").unwrap_or_else(|| "codes.db".into()),
        })
    }

    pub fn from_env() -> Result<Config, String> {
        Self::from_getter(|k| std::env::var(k).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn getter(map: HashMap<&'static str, &'static str>) -> impl Fn(&str) -> Option<String> {
        move |k| map.get(k).map(|s| s.to_string())
    }

    #[test]
    fn defaults_apply_when_only_secret_set() {
        let cfg =
            Config::from_getter(getter(HashMap::from([("WEBHOOK_SECRET", "s3cr3t")]))).unwrap();
        assert_eq!(cfg.secret, "s3cr3t");
        assert_eq!(cfg.bind_addr, "0.0.0.0:8080");
        assert_eq!(cfg.db_path, "codes.db");
    }

    #[test]
    fn missing_secret_is_error() {
        let err = Config::from_getter(getter(HashMap::new())).unwrap_err();
        assert!(err.contains("WEBHOOK_SECRET"));
    }
}
