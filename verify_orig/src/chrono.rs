use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use postgres::types::{Date, Timestamp};

pub type Connection = postgres::Client;

pub fn run(conn: &mut Connection) {
    eprint!("chrono ... ");
    conn.batch_execute(
        "CREATE TEMP TABLE chrono_test (
            id INT,
            ts TIMESTAMP,
            tstz TIMESTAMPTZ,
            date DATE,
            time TIME
        )",
    ).unwrap();

    // NaiveDateTime / TIMESTAMP
    let ts_val = NaiveDateTime::parse_from_str(
        "1970-01-01 00:00:00.010000000",
        "%Y-%m-%d %H:%M:%S%.f",
    ).unwrap();
    conn.execute(
        "INSERT INTO chrono_test (id, ts) VALUES (1, $1)",
        &[&ts_val],
    ).unwrap();
    let row = conn
        .query_one("SELECT ts FROM chrono_test WHERE id = 1", &[])
        .unwrap();
    let retrieved: NaiveDateTime = row.get(0);
    assert_eq!(ts_val, retrieved);

    // DateTime<Utc> / TIMESTAMPTZ
    let tstz_val = DateTime::parse_from_rfc3339("2010-02-09T23:11:45.120200000Z")
        .unwrap()
        .with_timezone(&Utc);
    conn.execute(
        "INSERT INTO chrono_test (id, tstz) VALUES (2, $1)",
        &[&tstz_val],
    ).unwrap();
    let row = conn
        .query_one("SELECT tstz FROM chrono_test WHERE id = 2", &[])
        .unwrap();
    let retrieved: DateTime<Utc> = row.get(0);
    assert_eq!(tstz_val, retrieved);

    // NaiveDate / DATE
    let date_val = NaiveDate::from_ymd_opt(1965, 9, 25).unwrap();
    conn.execute(
        "INSERT INTO chrono_test (id, date) VALUES (3, $1)",
        &[&date_val],
    ).unwrap();
    let row = conn
        .query_one("SELECT date FROM chrono_test WHERE id = 3", &[])
        .unwrap();
    let retrieved: NaiveDate = row.get(0);
    assert_eq!(date_val, retrieved);

    // NaiveTime / TIME
    let time_val = NaiveTime::parse_from_str("11:19:33.100314000", "%H:%M:%S%.f")
        .unwrap();
    conn.execute(
        "INSERT INTO chrono_test (id, time) VALUES (4, $1)",
        &[&time_val],
    ).unwrap();
    let row = conn
        .query_one("SELECT time FROM chrono_test WHERE id = 4", &[])
        .unwrap();
    let retrieved: NaiveTime = row.get(0);
    assert_eq!(time_val, retrieved);

    // NULL values
    conn.execute(
        "INSERT INTO chrono_test (id, ts, tstz, date, time) VALUES (5, $1, $2, $3, $4)",
        &[&Option::<NaiveDateTime>::None, &Option::<DateTime<Utc>>::None, &Option::<NaiveDate>::None, &Option::<NaiveTime>::None],
    ).unwrap();
    let row = conn
        .query_one("SELECT ts, tstz, date, time FROM chrono_test WHERE id = 5", &[])
        .unwrap();
    let ts_null: Option<NaiveDateTime> = row.get(0);
    let tstz_null: Option<DateTime<Utc>> = row.get(1);
    let date_null: Option<NaiveDate> = row.get(2);
    let time_null: Option<NaiveTime> = row.get(3);
    assert!(ts_null.is_none());
    assert!(tstz_null.is_none());
    assert!(date_null.is_none());
    assert!(time_null.is_none());

    // special values using wrappers
    conn.execute(
        "INSERT INTO chrono_test (id, ts, tstz, date) VALUES (6, $1, $2, $3)",
        &[
            &Timestamp::<NaiveDateTime>::PosInfinity,
            &Timestamp::<DateTime<Utc>>::PosInfinity,
            &Date::<NaiveDate>::PosInfinity,
        ],
    ).unwrap();
    conn.execute(
        "INSERT INTO chrono_test (id, ts, tstz, date) VALUES (7, $1, $2, $3)",
        &[
            &Timestamp::<NaiveDateTime>::NegInfinity,
            &Timestamp::<DateTime<Utc>>::NegInfinity,
            &Date::<NaiveDate>::NegInfinity,
        ],
    ).unwrap();

    let row = conn
        .query_one("SELECT ts, tstz, date FROM chrono_test WHERE id = 6", &[])
        .unwrap();
    let ts_pos: Timestamp<NaiveDateTime> = row.get(0);
    let tstz_pos: Timestamp<DateTime<Utc>> = row.get(1);
    let date_pos: Date<NaiveDate> = row.get(2);
    assert_eq!(ts_pos, Timestamp::PosInfinity);
    assert_eq!(tstz_pos, Timestamp::PosInfinity);
    assert_eq!(date_pos, Date::PosInfinity);

    let row = conn
        .query_one("SELECT ts, tstz, date FROM chrono_test WHERE id = 7", &[])
        .unwrap();
    let ts_neg: Timestamp<NaiveDateTime> = row.get(0);
    let tstz_neg: Timestamp<DateTime<Utc>> = row.get(1);
    let date_neg: Date<NaiveDate> = row.get(2);
    assert_eq!(ts_neg, Timestamp::NegInfinity);
    assert_eq!(tstz_neg, Timestamp::NegInfinity);
    assert_eq!(date_neg, Date::NegInfinity);

    eprintln!("ok");
}
