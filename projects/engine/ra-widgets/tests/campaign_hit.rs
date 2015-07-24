//! 战役页命中改为消费完整 content snapshot。

use ra_widgets::{
    menu_action::MenuAction,
    original_screen::OriginalScreen,
    ui_hit::{campaign_entry_at, hit_action},
};

#[test]
fn campaign_hits_use_content_snapshot() {
    let maps = [];
    assert_eq!(
        hit_action(
            OriginalScreen::Campaign,
            &maps,
            0,
            None,
            (722.0, 556.0),
            800.0,
            600.0,
            0, false,
        ),
        Some(MenuAction::Back)
    );
    // 盟军侧图中心附近。
    assert_eq!(
        hit_action(
            OriginalScreen::Campaign,
            &maps,
            0,
            None,
            (315.0, 90.0),
            800.0,
            600.0,
            0, false,
        ),
        Some(MenuAction::SelectCampaignAllied)
    );
    // 难度轨中心。
    assert_eq!(
        hit_action(
            OriginalScreen::Campaign,
            &maps,
            0,
            None,
            (314.0, 489.0),
            800.0,
            600.0,
            0, false,
        ),
        Some(MenuAction::CycleCampaignDifficulty)
    );
}

#[test]
fn campaign_entry_at_reads_content_snapshot() {
    assert_eq!(
        campaign_entry_at(315.0, 90.0, 800.0, 600.0),
        Some("allied")
    );
    assert_eq!(
        campaign_entry_at(722.0, 556.0, 800.0, 600.0),
        Some("back")
    );
    assert_eq!(
        campaign_entry_at(314.0, 489.0, 800.0, 600.0),
        Some("difficulty")
    );
}
