//! Westwood CSV：切分与 Serde 列序反序列化。

use ra_assets::{from_row, parse_westwood_csv_line};
use serde::Deserialize;

#[test]
fn parse_preserves_empty_columns() {
    let row = parse_westwood_csv_line("a,,b,");
    assert_eq!(row.len(), 4);
    assert_eq!(row.get(0), Some("a"));
    assert_eq!(row.get(1), Some(""));
    assert_eq!(row.get(2), Some("b"));
    assert_eq!(row.get(3), Some(""));
}

#[test]
fn parse_trims_fields_and_ignores_crlf() {
    let row = parse_westwood_csv_line(" Neutral , GACNST \r\n");
    assert_eq!(row.len(), 2);
    assert_eq!(row.get(0), Some("Neutral"));
    assert_eq!(row.get(1), Some("GACNST"));
}

#[derive(Debug, Deserialize, PartialEq)]
struct DemoRow {
    owner: String,
    type_id: String,
    health: u16,
    x: u16,
    y: u16,
    #[serde(default)]
    tag: String,
}

#[test]
fn from_row_binds_struct_fields_by_declaration_order() {
    let row: DemoRow = from_row("Americans,MTNK,256,5,6,TagA").expect("row");
    assert_eq!(
        row,
        DemoRow {
            owner: "Americans".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 5,
            y: 6,
            tag: "TagA".into(),
        }
    );
}

#[test]
fn from_row_missing_trailing_uses_default() {
    let row: DemoRow = from_row("Americans,MTNK,256,5,6").expect("row");
    assert!(row.tag.is_empty());
}

#[test]
fn from_row_rejects_non_numeric_required_cell() {
    let err = from_row::<DemoRow>("Americans,MTNK,256,xx,6").expect_err("bad x");
    assert!(err.to_string().contains("x") || err.to_string().contains("整数") || err.to_string().contains("无符号"));
}

#[test]
fn from_row_tuple_positional() {
    let (a, b, c): (String, u16, String) = from_row("hello,7,world").expect("tuple");
    assert_eq!((a.as_str(), b, c.as_str()), ("hello", 7, "world"));
}

#[test]
fn westwood_bool_scalars() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Flags {
        a: bool,
        b: bool,
    }
    let row: Flags = from_row("yes,no").expect("bools");
    assert_eq!(row, Flags { a: true, b: false });
}
