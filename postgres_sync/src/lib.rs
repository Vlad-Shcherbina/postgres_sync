use std::error::Error as StdError;
use std::io::{Read, Write};
use std::net::TcpStream;

use bytes::BytesMut;
use fallible_iterator::FallibleIterator;
use postgres_protocol::Oid;
use postgres_protocol::authentication::{
    md5_hash,
    sasl::{self, ChannelBinding, ScramSha256},
};
use postgres_protocol::message::backend;
use postgres_protocol::message::frontend;
use postgres_types::{IsNull, Type};

pub use fallible_iterator;
pub use postgres_types::{BorrowToSql, FromSql, ToSql};

mod conn_str;

pub type Error = Box<dyn StdError + Send + Sync>;

#[derive(Debug)]
pub struct DbError {
    severity: String,
    code: String,
    message: String,
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} ({})", self.severity, self.message, self.code)
    }
}

impl StdError for DbError {}

#[derive(Debug, Clone, Copy)]
pub struct NoTls;

pub struct Client {
    stream: TcpStream,
    read_buf: BytesMut,
    write_buf: BytesMut,
}

impl Client {
    pub fn connect(s: &str, _tls: NoTls) -> Result<Client, Error> {
        let parts = conn_str::ConnParts::parse(s).map_err(|()| "invalid connection string")?;
        let stream = TcpStream::connect((parts.host, parts.port))?;
        let user = parts.user;
        let db = parts.db;

        let mut this = Client {
            stream,
            read_buf: BytesMut::with_capacity(8192),
            write_buf: BytesMut::with_capacity(8192),
        };

        let mut params = Vec::new();
        params.push(("user", user));
        if !db.is_empty() {
            params.push(("database", db));
        }
        params.push(("client_encoding", "UTF8"));

        frontend::startup_message(params.iter().copied(), &mut this.write_buf)?;
        this.flush()?;

        this.handle_auth(user.as_bytes(), parts.password)?;

        loop {
            match this.read_message()? {
                backend::Message::ReadyForQuery(_) => break,
                backend::Message::BackendKeyData(_) => {}
                backend::Message::ParameterStatus(_) => {}
                backend::Message::ErrorResponse(body) => return Err(this.error_response(body).into()),
                _ => return Err("unexpected message".into()),
            }
        }

        Ok(this)
    }

    fn handle_auth(&mut self, user: &[u8], password: &str) -> Result<(), Error> {
        loop {
            match self.read_message()? {
                backend::Message::AuthenticationOk => break,
                backend::Message::AuthenticationCleartextPassword => {
                    // TODO: untested
                    frontend::password_message(password.as_bytes(), &mut self.write_buf)?;
                    self.flush()?;
                }
                backend::Message::AuthenticationMd5Password(body) => {
                    // TODO: untested
                    let output = md5_hash(user, password.as_bytes(), body.salt());
                    frontend::password_message(output.as_bytes(), &mut self.write_buf)?;
                    self.flush()?;
                }
                backend::Message::AuthenticationSasl(body) => {
                    let mut has_scram = false;
                    let mut mechs = body.mechanisms();
                    while let Some(mech) = mechs.next()? {
                        if mech == sasl::SCRAM_SHA_256 {
                            has_scram = true;
                        }
                    }
                    if !has_scram {
                        return Err("unsupported authentication".into());
                    }

                    let mut scram =
                        ScramSha256::new(password.as_bytes(), ChannelBinding::unsupported());

                    frontend::sasl_initial_response(
                        sasl::SCRAM_SHA_256,
                        scram.message(),
                        &mut self.write_buf,
                    )?;
                    self.flush()?;

                    let body = match self.read_message()? {
                        backend::Message::AuthenticationSaslContinue(body) => body,
                        backend::Message::ErrorResponse(body) => return Err(self.error_response(body).into()),
                        _ => return Err("unexpected message".into()),
                    };

                    scram.update(body.data())?;

                    frontend::sasl_response(scram.message(), &mut self.write_buf)?;
                    self.flush()?;

                    let body = match self.read_message()? {
                        backend::Message::AuthenticationSaslFinal(body) => body,
                        backend::Message::ErrorResponse(body) => return Err(self.error_response(body).into()),
                        _ => return Err("unexpected message".into()),
                    };

                    scram.finish(body.data())?;
                }
                backend::Message::ErrorResponse(body) => {
                    return Err(self.error_response(body).into());
                }
                _ => return Err("unsupported authentication".into()),
            }
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Error> {
        self.stream.write_all(&self.write_buf)?;
        self.stream.flush()?;
        self.write_buf.clear();
        Ok(())
    }

    fn read_message(&mut self) -> Result<backend::Message, Error> {
        loop {
            if let Some(message) = backend::Message::parse(&mut self.read_buf)? {
                return Ok(message);
            }
            let mut buf = [0u8; 8192];
            let n = self.stream.read(&mut buf)?;
            if n == 0 {
                return Err("unexpected EOF".into());
            }
            self.read_buf.extend_from_slice(&buf[..n]);
        }
    }

    fn error_response(&self, body: backend::ErrorResponseBody) -> DbError {
        let mut severity = String::new();
        let mut code = String::new();
        let mut message = String::new();
        let mut fields = body.fields();
        while let Some(field) = fields.next().unwrap() {
            match field.type_() {
                b'S' => severity = String::from_utf8_lossy(field.value_bytes()).into_owned(),
                b'C' => code = String::from_utf8_lossy(field.value_bytes()).into_owned(),
                b'M' => message = String::from_utf8_lossy(field.value_bytes()).into_owned(),
                _ => {}
            }
        }
        DbError { severity, code, message }
    }

    fn drain_ready(&mut self) -> Result<(), Error> {
        loop {
            match self.read_message()? {
                backend::Message::ReadyForQuery(_) => return Ok(()),
                backend::Message::ErrorResponse(body) => {
                    return Err(self.error_response(body).into())
                }
                _ => {}
            }
        }
    }

    #[allow(clippy::type_complexity)]
    fn prepare_query(
        &mut self,
        query: &str,
        params_len: usize,
    ) -> Result<(Vec<Type>, Vec<(String, Oid)>), Error> {
        let param_oids = vec![0; params_len];
        frontend::parse("", query, param_oids.iter().copied(), &mut self.write_buf)?;
        frontend::describe(b'S', "", &mut self.write_buf)?;
        frontend::sync(&mut self.write_buf);
        self.flush()?;

        let mut param_types = Vec::new();
        let mut columns = Vec::new();
        loop {
            match self.read_message()? {
                backend::Message::ParseComplete => {}
                backend::Message::ParameterDescription(body) => {
                    let mut it = body.parameters();
                    while let Some(oid) = it.next()? {
                        let ty = Type::from_oid(oid).unwrap_or(Type::TEXT);
                        param_types.push(ty);
                    }
                }
                backend::Message::RowDescription(body) => {
                    let mut fields = body.fields();
                    while let Some(field) = fields.next()? {
                        columns.push((field.name().to_string(), field.type_oid()));
                    }
                }
                backend::Message::NoData => {}
                backend::Message::ReadyForQuery(_) => break,
                backend::Message::ErrorResponse(body) => {
                    let err = self.error_response(body);
                    self.drain_ready()?;
                    return Err(err.into());
                }
                _ => return Err("unexpected message".into()),
            }
        }

        Ok((param_types, columns))
    }

    fn bind_execute<P, I>(
        &mut self,
        params: I,
        param_types: &[Type],
        mut rows: Option<&mut Vec<Vec<Option<Vec<u8>>>>>,
    ) -> Result<u64, Error>
    where
        P: BorrowToSql,
        I: IntoIterator<Item = P>,
        I::IntoIter: ExactSizeIterator,
    {
        let params: Vec<P> = params.into_iter().collect();
        assert_eq!(param_types.len(), params.len());
        let param_formats: Vec<i16> = params
            .iter()
            .zip(param_types)
            .map(|(p, t)| p.borrow_to_sql().encode_format(t) as i16)
            .collect();

        frontend::bind(
            "",
            "",
            param_formats,
            params.iter().zip(param_types.iter()),
            |(param, ty), buf| match param.borrow_to_sql().to_sql_checked(ty, buf)? {
                IsNull::No => Ok(postgres_protocol::IsNull::No),
                IsNull::Yes => Ok(postgres_protocol::IsNull::Yes),
            },
            Some(1),
            &mut self.write_buf,
        )
        .map_err(|e| match e {
            frontend::BindError::Conversion(e) => e,
            frontend::BindError::Serialization(e) => Box::new(e) as Error,
        })?;
        frontend::execute("", 0, &mut self.write_buf)?;
        frontend::sync(&mut self.write_buf);
        self.flush()?;

        let mut rows_affected = 0;
        loop {
            match self.read_message()? {
                backend::Message::BindComplete => {}
                backend::Message::DataRow(body) => {
                    if let Some(out) = rows.as_mut() {
                        out.push(self.parse_data_row(body)?);
                    }
                }
                backend::Message::CommandComplete(body) => {
                    let tag = body.tag().map_err(|e| Box::new(e) as Error)?;
                    rows_affected = tag
                        .rsplit(' ')
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                }
                backend::Message::EmptyQueryResponse => rows_affected = 0,
                backend::Message::ReadyForQuery(_) => return Ok(rows_affected),
                backend::Message::ErrorResponse(body) => {
                    let err = self.error_response(body);
                    self.drain_ready()?;
                    return Err(err.into());
                }
                _ => return Err("unexpected message".into()),
            }
        }
    }

    pub fn query_raw<P, I>(&mut self, query: &str, params: I) -> Result<RowIter, Error>
    where
        P: BorrowToSql,
        I: IntoIterator<Item = P>,
        I::IntoIter: ExactSizeIterator,
    {
        let params = params.into_iter();
        let (param_types, columns) = self.prepare_query(query, params.len())?;
        let params: Vec<P> = params.collect();
        let mut rows = Vec::new();
        self.bind_execute(params, &param_types, Some(&mut rows))?;

        Ok(RowIter {
            columns,
            rows: rows.into_iter(),
        })
    }

    pub fn execute(&mut self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error> {
        let (param_types, _) = self.prepare_query(query, params.len())?;
        self.bind_execute(params.iter().copied(), &param_types, None)
    }

    pub fn query_one(&mut self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error> {
        let mut it = self.query_raw(query, params.iter().copied())?;
        let first = it.next()?.ok_or("no rows returned")?;
        if it.next()?.is_some() {
            return Err("more than one row returned".into());
        }
        Ok(first)
    }

    pub fn batch_execute(&mut self, query: &str) -> Result<(), Error> {
        frontend::query(query, &mut self.write_buf)?;
        self.flush()?;

        loop {
            match self.read_message()? {
                backend::Message::ReadyForQuery(_) => return Ok(()),
                backend::Message::CommandComplete(_)
                | backend::Message::EmptyQueryResponse
                | backend::Message::RowDescription(_)
                | backend::Message::DataRow(_) => {}
                backend::Message::ErrorResponse(body) => {
                    let err = self.error_response(body);
                    self.drain_ready()?;
                    return Err(err.into());
                }
                _ => return Err("unexpected message".into()),
            }
        }
    }

    fn parse_data_row(&self, body: backend::DataRowBody) -> Result<Vec<Option<Vec<u8>>>, Error> {
        let mut out = Vec::new();
        let mut ranges = body.ranges();
        let buf = body.buffer();
        while let Some(range) = ranges.next()? {
            match range {
                Some(r) => out.push(Some(buf[r].to_vec())),
                None => out.push(None),
            }
        }
        Ok(out)
    }
}

pub struct Row {
    columns: Vec<(String, Oid)>,
    values: Vec<Option<Vec<u8>>>,
}

impl Row {
    pub fn get<T>(&self, idx: usize) -> T
    where
        for<'a> T: FromSql<'a>,
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Debug,
    {
        let (_, oid) = &self.columns[idx];
        let ty = Type::from_oid(*oid).unwrap_or(Type::TEXT);
        let raw = self.values[idx].as_deref();
        match FromSql::from_sql_nullable(&ty, raw) {
            Ok(v) => v,
            Err(_) => {
                let s = std::str::from_utf8(raw.expect("NULL value")).unwrap();
                s.parse().unwrap()
            }
        }
    }
}

pub struct RowIter {
    columns: Vec<(String, Oid)>,
    rows: std::vec::IntoIter<Vec<Option<Vec<u8>>>>,
}

impl FallibleIterator for RowIter {
    type Item = Row;
    type Error = Error;

    fn next(&mut self) -> Result<Option<Row>, Error> {
        Ok(self.rows.next().map(|values| Row {
            columns: self.columns.clone(),
            values,
        }))
    }
}
