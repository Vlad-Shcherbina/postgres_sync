use crate::{BorrowToSql, Client, Error, Row, RowIter, ToSql};

pub struct Transaction<'a> {
    pub(crate) client: &'a mut Client,
    finished: bool,
}

impl Client {
    pub fn transaction(&mut self) -> Result<Transaction<'_>, Error> {
        self.batch_execute("BEGIN")?;
        Ok(Transaction {
            client: self,
            finished: false,
        })
    }
}

impl<'a> Transaction<'a> {
    pub fn commit(mut self) -> Result<(), Error> {
        if !self.finished {
            self.client.batch_execute("COMMIT")?;
            self.finished = true;
        }
        Ok(())
    }

    pub fn rollback(mut self) -> Result<(), Error> {
        if !self.finished {
            self.client.batch_execute("ROLLBACK")?;
            self.finished = true;
        }
        Ok(())
    }

    pub fn batch_execute(&mut self, query: &str) -> Result<(), Error> {
        self.client.batch_execute(query)
    }

    pub fn execute(&mut self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error> {
        self.client.execute(query, params)
    }

    pub fn query_one(&mut self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error> {
        self.client.query_one(query, params)
    }

    pub fn query_raw<P, I>(&mut self, query: &str, params: I) -> Result<RowIter, Error>
    where
        P: BorrowToSql,
        I: IntoIterator<Item = P>,
        I::IntoIter: ExactSizeIterator,
    {
        self.client.query_raw(query, params)
    }
}

impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.client.batch_execute("ROLLBACK");
            self.finished = true;
        }
    }
}
