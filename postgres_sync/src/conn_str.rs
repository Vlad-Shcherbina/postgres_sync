pub(crate) struct ConnParts<'a> {
    pub(crate) user: &'a str,
    pub(crate) password: &'a str,
    pub(crate) host: &'a str,
    pub(crate) port: u16,
    pub(crate) db: &'a str,
}

impl<'a> ConnParts<'a> {
    pub(crate) fn parse(s: &'a str) -> Result<Self, ()> {
        let s = s.strip_prefix("postgresql://").ok_or(())?;
        let (creds, rest) = s.split_once('@').ok_or(())?;
        let (user, password) = creds.split_once(':').ok_or(())?;
        let (host_port, db) = rest.split_once('/').ok_or(())?;
        let (host, port_str) = host_port.split_once(':').ok_or(())?;
        let port: u16 = port_str.parse().map_err(|_| ())?;
        Ok(ConnParts {
            user,
            password,
            host,
            port,
            db,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_conn_str() {
        let parts = ConnParts::parse("postgresql://user:pass@localhost:5432/db").unwrap();
        assert_eq!(parts.user, "user");
        assert_eq!(parts.password, "pass");
        assert_eq!(parts.host, "localhost");
        assert_eq!(parts.port, 5432);
        assert_eq!(parts.db, "db");
    }
}
