use postgres::{Connection, TlsMode};

fn main() {
    let conn_str = std::env::args().nth(1).unwrap();
    let conn = Connection::connect(conn_str, TlsMode::None)
        .unwrap();
    println!("connected");
    conn.query("SELECT 40 + 2", &[])
        .unwrap()
        .iter()
        .for_each(|row| {
            let value: i32 = row.get(0);
            println!("{value}");
        });
}
