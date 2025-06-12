pub struct Config {
    pub(crate) user: String,
    pub(crate) password: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) db: String,
}

impl Config {
    fn parse_inner(s: &str) -> Result<Self, ()> {
        let s = s.strip_prefix("postgresql://").ok_or(())?;
        let (creds, rest) = s.split_once('@').ok_or(())?;
        let (user, password) = creds.split_once(':').ok_or(())?;
        let (host_port, db) = rest.split_once('/').ok_or(())?;
        let (host, port_str) = host_port.split_once(':').ok_or(())?;
        let port: u16 = port_str.parse().map_err(|_| ())?;
        Ok(Config {
            user: user.to_string(),
            password: password.to_string(),
            host: host.to_string(),
            port,
            db: db.to_string(),
        })
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        Self::parse_inner(s).map_err(|()| "invalid connection string".into())
    }

    pub fn connect(&self, _tls: crate::NoTls) -> Result<crate::Client, crate::Error> {
        crate::Client::connect_config(self, _tls)
    }
}

impl std::str::FromStr for Config {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let config = Config::parse("postgresql://user:pass@localhost:5432/db").unwrap();
        assert_eq!(config.user, "user");
        assert_eq!(config.password, "pass");
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.db, "db");
    }
}
