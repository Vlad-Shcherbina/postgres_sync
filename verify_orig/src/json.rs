use serde_json::Value;

pub type Connection = postgres::Client;

pub fn run(conn: &mut Connection) {
    eprint!("json ... ");
    conn.batch_execute("CREATE TEMP TABLE json_test (id INT, json_val JSON, jsonb_val JSONB)").unwrap();
    let json_value: Value = serde_json::from_str(r#"{"a": 1, "b": [2, 3]}"#).unwrap();
    let json_null: Option<Value> = None;

    conn.execute(
        "INSERT INTO json_test (id, json_val, jsonb_val) VALUES (1, $1, $2)",
        &[&json_value, &json_value],
    ).unwrap();

    conn.execute(
        "INSERT INTO json_test (id, json_val, jsonb_val) VALUES (2, $1, $2)",
        &[&json_null, &json_null],
    ).unwrap();

    let row = conn.query_one("SELECT json_val, jsonb_val FROM json_test WHERE id = 1", &[]).unwrap();
    let retrieved_json: Value = row.get(0);
    let retrieved_jsonb: Value = row.get(1);
    assert_eq!(json_value, retrieved_json);
    assert_eq!(json_value, retrieved_jsonb);

    let row_null = conn.query_one("SELECT json_val, jsonb_val FROM json_test WHERE id = 2", &[]).unwrap();
    let retrieved_json_null: Option<Value> = row_null.get(0);
    let retrieved_jsonb_null: Option<Value> = row_null.get(1);
    assert_eq!(retrieved_json_null, None);
    assert_eq!(retrieved_jsonb_null, None);

    eprintln!("ok");

    eprint!("Json wrapper ... ");
    let pair = (10i32, 20i32);
    conn.execute(
        "INSERT INTO json_test (id, json_val, jsonb_val) VALUES (42, $1, $2)",
        &[&postgres::types::Json(&pair), &postgres::types::Json(&pair)],
    ).unwrap();
    let row = conn.query_one("SELECT json_val, jsonb_val FROM json_test WHERE id = 42", &[]).unwrap();
    let value: postgres::types::Json<(i32, i32)> = row.get(0);
    assert_eq!(value.0, pair);
    let value: postgres::types::Json<(i32, i32)> = row.get(1);
    assert_eq!(value.0, pair);
    eprintln!("ok");
}
