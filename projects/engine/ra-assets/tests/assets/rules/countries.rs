//! 自 `engine/ra-assets/src/rules/countries.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-assets/src/rules/countries.rs :: tests
use ra_assets::{IniDocument, rules::countries::*};

const SAMPLE: &str = r#"
[Countries]
0=Americans
1=French
2=Russians
3=Unused
4=YuriCountry

[Americans]
UIName=Name:Americans
Prefix=USA
Color=Gold
Side=GDI
Multiplay=yes

[French]
UIName=Name:French
Prefix=FRA
Color=LightBlue
Side=GDI
Multiplay=yes

[Russians]
UIName=Name:Russians
Prefix=RUS
Color=DarkRed
Side=Nod
Multiplay=yes

[Unused]
Multiplay=no

[YuriCountry]
UIName=Name:Yuri
Prefix=YUR
Color=Purple
Side=ThirdSide
Multiplay=yes
MultiplayObsolete=no

[Sides]
GDI=Americans,French
Nod=Russians
ThirdSide=YuriCountry
Civilian=Neutral

[GDI]
Sidebar.MixFileIndex=1
Sidebar.YuriFileNames=yes

[Nod]
Sidebar.MixFileIndex=2

[FourthSide]
Sidebar.MixFileIndex=4
Sidebar.YuriFileNames=yes
MultiplayerScore.Background=mpfscrnl.shp
MultiplayerScore.Palette=mpsscrnlf.pal
"#;

#[test]
fn parses_countries_and_sides_in_list_order() {
    let doc = IniDocument::parse(SAMPLE.as_bytes()).unwrap();
    let reg = CountryRegistry::from_rules(&doc);
    assert_eq!(reg.len(), 5);
    assert_eq!(reg.countries()[0].id, "Americans");
    assert_eq!(reg.countries()[0].prefix, "USA");
    assert_eq!(reg.countries()[0].side, "GDI");
    assert!(reg.countries()[0].multiplay);
    assert!(!reg.countries()[3].multiplay);
    assert_eq!(reg.sides().len(), 4);
    assert_eq!(reg.sides()[0].id, "GDI");
    assert_eq!(reg.sides()[0].countries, vec!["Americans", "French"]);
    let gdi = reg.side_chrome("GDI").unwrap();
    assert_eq!(gdi.mix_file_index, Some(1));
    assert!(gdi.yuri_file_names);
    let nod = reg.side_chrome("Nod").unwrap();
    assert_eq!(nod.mix_file_index, Some(2));
    assert!(!nod.yuri_file_names);
    // ThirdSide 无显式键：内核保持空，由 edition adaptor 填。
    let third = reg.side_chrome("ThirdSide").unwrap();
    assert_eq!(third.mix_file_index, None);
    assert!(!third.yuri_file_names);
}

#[test]
fn parses_open_side_chrome_score_overrides() {
    const RULES: &str = r#"
[Countries]
0=Guild1

[Guild1]
Side=FifthSide
Multiplay=yes
File.LoadScreen=ls800haihead.shp
File.LoadScreenPAL=mplshh.pal

[Sides]
GDI=Americans
FifthSide=Guild1

[FifthSide]
Sidebar.MixFileIndex=5
Sidebar.YuriFileNames=no
MultiplayerScore.Background=mpxscrnl.shp
MultiplayerScore.Palette=mpxscrn.pal
EVA.Tag=Foehn
"#;
    let doc = IniDocument::parse(RULES.as_bytes()).unwrap();
    let reg = CountryRegistry::from_rules(&doc);
    let fifth = reg.side_chrome("FifthSide").unwrap();
    assert_eq!(fifth.mix_file_index, Some(5));
    assert!(!fifth.yuri_file_names);
    assert_eq!(fifth.score_background.as_deref(), Some("mpxscrnl.shp"));
    assert_eq!(fifth.score_palette.as_deref(), Some("mpxscrn.pal"));
    assert_eq!(fifth.eva_tag.as_deref(), Some("Foehn"));
    let guild = reg.get("Guild1").unwrap();
    assert_eq!(guild.load_screen, "ls800haihead.shp");
    assert_eq!(guild.load_screen_pal, "mplshh.pal");
}

#[test]
fn skirmish_filter_drops_non_multiplay() {
    let doc = IniDocument::parse(SAMPLE.as_bytes()).unwrap();
    let reg = CountryRegistry::from_rules(&doc);
    let ids: Vec<_> = reg.skirmish_countries().iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, vec!["Americans", "French", "Russians", "YuriCountry"]);
}

#[test]
fn special_ui_name_from_required_houses_and_superweapon() {
    const RULES: &str = r#"
[Countries]
0=Americans
1=Confederation

[Americans]
Multiplay=yes
[Confederation]
Multiplay=yes

[InfantryTypes]
0=TERROR
1=E1

[TERROR]
UIName=Name:TERROR
RequiredHouses=Confederation

[E1]
UIName=Name:E1

[BuildingTypes]
0=GAPILE

[GAPILE]
UIName=Name:GAPILE
RequiredHouses=Americans
SuperWeapon=ParaDrop

[SuperWeaponTypes]
0=ParaDrop

[ParaDrop]
UIName=Name:PARA
"#;
    let doc = IniDocument::parse(RULES.as_bytes()).unwrap();
    let reg = CountryRegistry::from_rules(&doc);
    assert_eq!(reg.get("Confederation").unwrap().special_ui_name, "Name:TERROR");
    assert_eq!(reg.get("Americans").unwrap().special_ui_name, "Name:PARA");
}
