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
    assert_eq!(g.savour_delay_minutes, None);
    assert_eq!(g.prerequisite_power, vec![ra_types::TechnoName::parse("GAPOWR"), ra_types::TechnoName::parse("NAPOWR")]);
    assert_eq!(g.prerequisite_factory, vec![ra_types::TechnoName::parse("GAWEAP")]);
}

#[test]
fn parse_audio_visual_savour_delay() {
    let doc = IniDocument::parse(b"[AudioVisual]\nSavourDelay=0.1\n").unwrap();
    let g = RulesGlobals::from_rules(&doc);
    assert!((g.savour_delay_minutes.unwrap() - 0.1).abs() < 1e-9);
}

#[test]
fn from_layered_merges_general_override() {
    let base = IniDocument::parse(b"[General]\nRepairStep=8\nRepairPercent=15\n").unwrap();
    let top = IniDocument::parse(b"[General]\nRepairStep=16\n").unwrap();
    let policy = IniMergePolicy { default_entry: EntryMergePolicy::MergeSection };
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
    assert_eq!(g.prerequisite_power, vec![ra_types::TechnoName::parse("GAPOWR"), ra_types::TechnoName::parse("NAPOWR")]);
}

#[test]
fn parse_ai_base_spacing_and_naval_yard_adjacency() {
    let doc = IniDocument::parse(b"[General]\nAINavalYardAdjacency=20\n[AI]\nAIBaseSpacing=1\n").unwrap();
    let g = RulesGlobals::from_rules(&doc);
    assert_eq!(g.ai_base_spacing, Some(1));
    assert_eq!(g.ai_naval_yard_adjacency, Some(20));
}

#[test]
fn parse_ai_build_lists_ratios_and_iq() {
    let doc = IniDocument::parse(
        b"[AI]\nAIBaseSpacing=1\nPowerSurplus=100\nBaseSizeAdd=2\n\
BuildPower=NAPOWR,GAPOWR\nBuildRefinery=NAREFN\nRefineryRatio=.16\nRefineryLimit=4\n\
BuildBarracks=NAHAND\nBarracksRatio=.1\nBarracksLimit=2\n\
BuildWeapons=NAWEAP\nWarRatio=.1\nWarLimit=2\n\
BuildRadar=NARADR\nBuildTech=NATECH\n\
[IQ]\nMaxIQLevels=5\nProduction=3\n",
    )
    .unwrap();
    let g = RulesGlobals::from_rules(&doc);
    assert_eq!(g.ai_power_surplus, Some(100));
    assert_eq!(g.ai_base_size_add, Some(2));
    assert_eq!(g.ai_build_power, vec![ra_types::TechnoName::parse("NAPOWR"), ra_types::TechnoName::parse("GAPOWR")]);
    assert_eq!(g.ai_build_refinery, vec![ra_types::TechnoName::parse("NAREFN")]);
    assert!((g.ai_refinery_ratio.unwrap() - 0.16).abs() < 1e-9);
    assert_eq!(g.ai_refinery_limit, Some(4));
    assert_eq!(g.ai_build_barracks, vec![ra_types::TechnoName::parse("NAHAND")]);
    assert_eq!(g.ai_build_weapons, vec![ra_types::TechnoName::parse("NAWEAP")]);
    assert_eq!(g.ai_build_radar, vec![ra_types::TechnoName::parse("NARADR")]);
    assert_eq!(g.ai_build_tech, vec![ra_types::TechnoName::parse("NATECH")]);
    assert_eq!(g.iq_max_levels, Some(5));
    assert_eq!(g.iq_production, Some(3));
}
