//! 命令拒绝 HUD 文案。

use ra_engine::CommandRejectReason;

#[test]
fn hud_labels_cover_economy_rejects() {
    assert_eq!(CommandRejectReason::InsufficientFunds.as_hud_label(), "资金不足");
    assert_eq!(CommandRejectReason::MissingPrerequisite.as_hud_label(), "前置不足");
    assert_eq!(CommandRejectReason::QueueFull.as_hud_label(), "队列已满");
}
