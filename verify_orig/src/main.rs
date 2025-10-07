use postgres::fallible_iterator::FallibleIterator as _;

mod json;
mod chrono;
mod logger;

fn main() {
    logger::init_logger();
    let s = std::env::args().nth(1).unwrap();
    eprint!("Config::connect ... ");
    let config = s.parse::<postgres::Config>().unwrap();
    let mut client = config.connect(postgres::NoTls).unwrap();
    let row = client.query_one("SELECT 1 + 1", &[]).unwrap();
    let v: i32 = row.get(0);
    assert_eq!(v, 2);
    eprintln!("ok");

    eprint!("Client::connect ... ");
    let mut client = postgres::Client::connect(&s, postgres::NoTls).unwrap();
    eprintln!("ok");

    eprint!("SELECT 2 + 2 ... ");
    let row = client
        .query_one("SELECT 2 + 2", &[])
        .unwrap();
    let result: i32 = row.get(0);
    assert_eq!(result, 4);
    eprintln!("ok");

    eprint!("parameters ... ");
    let row = client
        .query_one("SELECT $1::INT4 + $2::INT4", &[&2i32, &2i32])
        .unwrap();
    let result: i32 = row.get(0);
    assert_eq!(result, 4);
    eprintln!("ok");

    eprint!("syntax error ... ");
    let e = client.query_one("foobar", &[]).err().unwrap();
    let e = format!("{e:?}");
    assert!(e.contains("syntax error at or near \\\"foobar\\\""), "{e}");
    assert!(e.contains("position: Some(Original(1))"), "{e}");
    eprintln!("ok");

    eprint!("error with hint ... ");
    let e = client
        .query_one("SELECT $1 + $2", &[&2i32, &2i32])
        .err().unwrap();
    let e = format!("{e:?}");
    assert!(e.contains("operator is not unique: unknown + unknown"), "{e}");  // error message
    assert!(e.contains("Could not choose a best candidate operator. You might need to add explicit type casts."), "{e}");  // hint
    assert!(e.contains("position: Some(Original(11))"), "{e}");
    eprintln!("ok");

    eprint!("table already exists (notice) ... ");
    client
        .batch_execute("CREATE TEMP TABLE IF NOT EXISTS ifexists_test (id INT)")
        .unwrap();
    client
        .batch_execute("CREATE TEMP TABLE IF NOT EXISTS ifexists_test (id INT)")
        .unwrap();
    eprintln!("ok");

    eprint!("batch_execute ... ");
    client.batch_execute("
        CREATE TEMP TABLE test (id INT PRIMARY KEY, value TEXT);
        INSERT INTO test VALUES (1, 'one'), (2, 'two');
    ").unwrap();
    let row = client
        .query_one("SELECT COUNT(*) FROM test", &[])
        .unwrap();
    let count: i64 = row.get(0);
    assert_eq!(count, 2);
    eprintln!("ok");

    eprint!("query ... ");
    let rows = client.query("SELECT value FROM test ORDER BY id", &[]).unwrap();
    assert_eq!(rows.len(), 2);
    let val: String = rows[0].get(0);
    assert_eq!(val, "one");
    let val: String = rows[1].get(0);
    assert_eq!(val, "two");
    eprintln!("ok");

    eprint!("query_raw ... ");
    let mut it = client
        .query_raw(
            "SELECT id, value FROM test ORDER BY id",
            std::iter::empty::<&i32>(),
        )
        .unwrap();
    let row = it.next().unwrap().unwrap();
    let id: i32 = row.get(0);
    let value: String = row.get(1);
    assert_eq!(id, 1);
    assert_eq!(value, "one");
    let row = it.next().unwrap().unwrap();
    let id: i32 = row.get(0);
    let value: String = row.get(1);
    assert_eq!(id, 2);
    assert_eq!(value, "two");
    assert!(it.next().unwrap().is_none());
    drop(it);
    eprintln!("ok");

    eprint!("borrow row.get ... ");
    let row = client
        .query_one("SELECT 'foo'::TEXT, 'bar'::BYTEA", &[])
        .unwrap();
    let text: &str = row.get(0);
    assert_eq!(text, "foo");
    let bytes: &[u8] = row.get(1);
    assert_eq!(bytes, b"bar");
    eprintln!("ok");

    eprint!("row.get by name ... ");
    let row = client
        .query_one("SELECT 1 AS one, 2 AS two", &[])
        .unwrap();
    let one: i32 = row.get("one");
    let two: i32 = row.get("two");
    assert_eq!(one, 1);
    assert_eq!(two, 2);
    eprintln!("ok");

    eprint!("execute ... ");
    let rows = client
        .execute("INSERT INTO test VALUES ($1, $2)", &[&3i32, &"three"])
        .unwrap();
    assert_eq!(rows, 1);
    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM test", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 3);
    eprintln!("ok");

    eprint!("transaction commit ... ");
    {
        let mut tx = client.transaction().unwrap();
        tx.execute("INSERT INTO test VALUES ($1, $2)", &[&4i32, &"four"]).unwrap();
        tx.commit().unwrap();
    }
    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM test WHERE id = 4", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
    eprintln!("ok");

    eprint!("transaction rollback ... ");
    {
        let mut tx = client.transaction().unwrap();
        tx.execute("INSERT INTO test VALUES ($1, $2)", &[&5i32, &"five"]).unwrap();
        // implicitly dropped without commit
    }
    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM test WHERE id = 5", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM test", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 4);
    eprintln!("ok");

    json::run(&mut client);
    chrono::run(&mut client);
}
