//! 全局节字段解码。

use ra_assets::*;

#[test]
fn parse_general_repair_speak_and_prerequisites() {
    let doc = IniDocument::parse(
        b"[General]\nRepairPercent=25\nRepairStep=16\nRepairRate=.032\nSpeakDelay=0.1\n\
PrerequisitePower=GAPOWR,NAPOWR\nPrerequisiteFactory=GAWEAP\n\
[MultiplayerDialogSettings]\nTechLevel=7\n\
[AudioVisual]\nSpeakDelay=0.2\n",
    )
    .unwrap();
    let g = RulesGlobals::from_rules(&doc);
    assert_eq!(g.repair_percent, Some(25));
    assert_eq!(g.repair_step, Some(16));
    assert!((g.repair_rate_minutes.unwrap() - 0.032).abs() < 1e-9);
    assert_eq!(g.multiplayer_tech_level, Some(7));
    // AudioVisual 优先于 General。
    assert!((g.speak_delay_minutes.unwrap() - 0.2).abs() < 1e-9);
    assert_eq!(
        g.prerequisite_power,
        vec![ra_types::TechnoName::parse("GAPOWR"), ra_types::TechnoName::parse("NAPOWR")]
    );
    assert_eq!(g.prerequisite_factory, vec![ra_types::TechnoName::parse("GAWEAP")]);
}

#[test]
fn from_layered_merges_general_override() {
    let base = IniDocument::parse(b"[General]\nRepairStep=8\nRepairPercent=15\n").unwrap();
    let top = IniDocument::parse(b"[General]\nRepairStep=16\n").unwrap();
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::MergeSection,
    };
    let docs = [base, top];
    let g = RulesGlobals::from_layered(LayeredIniView::new(&docs, &policy));
    assert_eq!(g.repair_step, Some(16));
    assert_eq!(g.repair_percent, Some(15));
}

#[test]
fn soft_parse_percent_suffix_and_keeps_prerequisites() {
    let doc = IniDocument::parse(
        b"[General]\nRepairPercent=25%\nRepairStep=nope\nRepairRate=bad\nSpeakDelay=\n\
PrerequisitePower=GAPOWR,NAPOWR\n\
[MultiplayerDialogSettings]\nTechLevel=10%\n",
    )
    .unwrap();
    let g = RulesGlobals::from_rules(&doc);
    assert_eq!(g.repair_percent, Some(25));
    assert_eq!(g.repair_step, None);
    assert_eq!(g.repair_rate_minutes, None);
    assert_eq!(g.speak_delay_minutes, None);
    assert_eq!(g.multiplayer_tech_level, Some(10));
    assert_eq!(
        g.prerequisite_power,
        vec![ra_types::TechnoName::parse("GAPOWR"), ra_types::TechnoName::parse("NAPOWR")]
    );
}
