//! `IniSection` Serde 反序列化。

use ra_assets::IniDocument;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct TechnoFields {
    #[serde(rename = "Strength")]
    strength: u32,
    #[serde(rename = "Cost")]
    cost: u32,
    #[serde(rename = "Naval")]
    naval: Option<bool>,
    #[serde(rename = "Primary")]
    primary: Option<String>,
    #[serde(rename = "Prerequisite", default)]
    prerequisite: Vec<String>,
}

#[test]
fn deserialize_section_scalars_and_list() {
    let doc = IniDocument::parse(
        b"[MTNK]\nStrength=400\nCost=800\nNaval=no\nPrimary=105mm\nPrerequisite=GAPILE,GAWEAP\n",
    )
    .unwrap();
    let fields: TechnoFields = doc.section("MTNK").unwrap().deserialize().unwrap();
    assert_eq!(fields.strength, 400);
    assert_eq!(fields.cost, 800);
    assert_eq!(fields.naval, Some(false));
    assert_eq!(fields.primary.as_deref(), Some("105mm"));
    assert_eq!(fields.prerequisite, vec!["GAPILE", "GAWEAP"]);
}

#[test]
fn missing_optional_is_none_and_default_list_empty() {
    let doc = IniDocument::parse(b"[E1]\nStrength=125\nCost=200\n").unwrap();
    let fields: TechnoFields = doc.section("E1").unwrap().deserialize().unwrap();
    assert_eq!(fields.strength, 125);
    assert_eq!(fields.naval, None);
    assert_eq!(fields.primary, None);
    assert!(fields.prerequisite.is_empty());
}

#[test]
fn last_duplicate_key_wins() {
    let doc = IniDocument::parse(b"[X]\nStrength=1\nStrength=9\nCost=0\n").unwrap();
    let fields: TechnoFields = doc.section("X").unwrap().deserialize().unwrap();
    assert_eq!(fields.strength, 9);
}

#[test]
fn bool_accepts_yes_no_and_digits() {
    #[derive(Deserialize)]
    struct Flags {
        #[serde(rename = "A")]
        a: bool,
        #[serde(rename = "B")]
        b: bool,
    }
    let doc = IniDocument::parse(b"[F]\nA=yes\nB=0\n").unwrap();
    let f: Flags = doc.section("F").unwrap().deserialize().unwrap();
    assert!(f.a);
    assert!(!f.b);
}

#[test]
fn parse_error_includes_section_and_key() {
    #[derive(Debug, Deserialize)]
    struct Fields {
        #[serde(rename = "Strength")]
        strength: u32,
    }
    let doc = IniDocument::parse(b"[MTNK]\nStrength=not-a-number\n").unwrap();
    let err = doc.section("MTNK").unwrap().deserialize::<Fields>().unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("[MTNK]"), "{msg}");
    assert!(msg.contains("Strength") || msg.contains("STRENGTH"), "{msg}");
    assert!(err.span.is_some(), "expected SourceSpan on field error");
}
