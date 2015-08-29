//! 对局软件光标：`mouse.shp` 边缘滚屏 / 贴边禁止 / 部署帧 → winit `CustomCursor`。

use ra_widgets::battle_order_icons::{
    load_battle_edge_cursors, DecodedBattleEdgeCursors, DecodedMouseCursorFrame, MOUSE_SCROLL_DIR_COUNT,
};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Cursor, CursorIcon, CustomCursor};

use crate::host::battle_input::{BattlePointer, EdgeScrollCursor, EdgeScrollDir};

use super::Shell;

/// 已上传到平台的对局 `mouse.shp` 软件光标。
pub(crate) struct BattleMouseCursorSet {
    scroll: [CustomCursor; MOUSE_SCROLL_DIR_COUNT],
    blocked: [CustomCursor; MOUSE_SCROLL_DIR_COUNT],
    deploy: CustomCursor,
}

impl Shell {
    /// 从安装资源装入边缘滚屏等软件光标（每进程一次；失败则继续用系统占位）。
    pub(super) fn ensure_battle_mouse_cursors(&mut self, event_loop: &ActiveEventLoop) {
        if self.battle_mouse_cursors.is_some() || self.battle_mouse_cursors_tried {
            return;
        }
        self.ensure_menu_assets();
        let Some(source) = self.menu_assets.as_ref().and_then(|a| a.source.as_ref())
        else {
            return;
        };
        self.battle_mouse_cursors_tried = true;
        let Some(decoded) = load_battle_edge_cursors(source)
        else {
            tracing::warn!("对局软件光标装入失败 · 缺少 mouse.shp / mousepal.pal");
            return;
        };
        match BattleMouseCursorSet::from_decoded(event_loop, &decoded) {
            Some(set) => {
                tracing::info!("对局软件光标 · scroll/blocked×8 + deploy · mouse.shp");
                self.battle_mouse_cursors = Some(set);
                // 强制下一帧按新图集重设指针。
                self.battle_pointer = BattlePointer::Default;
            }
            None => tracing::warn!("对局软件光标 · CustomCursor 创建失败"),
        }
    }

    pub(super) fn sync_battle_edge_cursor(&mut self) {
        let cur = if self.screen == ra_widgets::original_screen::OriginalScreen::Battle {
            self.battle_controller
                .as_ref()
                .map(|c| c.battle_pointer())
                .unwrap_or(BattlePointer::Default)
        } else {
            BattlePointer::Default
        };
        self.apply_battle_pointer(cur);
    }

    pub(super) fn apply_battle_pointer(&mut self, cur: BattlePointer) {
        if cur == self.battle_pointer {
            return;
        }
        let Some(window) = self.window.as_ref()
        else {
            return;
        };
        let cursor = self.resolve_battle_cursor(cur);
        window.set_cursor(cursor);
        self.battle_pointer = cur;
    }

    fn resolve_battle_cursor(&self, cur: BattlePointer) -> Cursor {
        if let Some(set) = self.battle_mouse_cursors.as_ref() {
            if let Some(c) = set.cursor_for(cur) {
                return Cursor::Custom(c);
            }
        }
        Cursor::Icon(system_battle_cursor_fallback(cur))
    }
}

impl BattleMouseCursorSet {
    fn from_decoded(event_loop: &ActiveEventLoop, decoded: &DecodedBattleEdgeCursors) -> Option<Self> {
        let scroll = create_dir_cursors(event_loop, &decoded.scroll)?;
        let blocked = create_dir_cursors(event_loop, &decoded.blocked)?;
        let deploy = create_custom_cursor(event_loop, &decoded.deploy)?;
        Some(Self {
            scroll,
            blocked,
            deploy,
        })
    }

    fn cursor_for(&self, cur: BattlePointer) -> Option<CustomCursor> {
        match cur {
            BattlePointer::Deploy => Some(self.deploy.clone()),
            BattlePointer::Edge(EdgeScrollCursor::Scroll(dir)) => {
                dir_index(dir).map(|i| self.scroll[i].clone())
            }
            BattlePointer::Edge(EdgeScrollCursor::Blocked(dir)) => {
                dir_index(dir).map(|i| self.blocked[i].clone())
            }
            BattlePointer::Default | BattlePointer::Edge(EdgeScrollCursor::Default) => None,
        }
    }
}

fn create_dir_cursors(
    event_loop: &ActiveEventLoop,
    frames: &[DecodedMouseCursorFrame; MOUSE_SCROLL_DIR_COUNT],
) -> Option<[CustomCursor; MOUSE_SCROLL_DIR_COUNT]> {
    let mut out: Vec<CustomCursor> = Vec::with_capacity(MOUSE_SCROLL_DIR_COUNT);
    for frame in frames {
        out.push(create_custom_cursor(event_loop, frame)?);
    }
    out.try_into().ok()
}

fn create_custom_cursor(
    event_loop: &ActiveEventLoop,
    frame: &DecodedMouseCursorFrame,
) -> Option<CustomCursor> {
    let w = u16::try_from(frame.image.width()).ok()?;
    let h = u16::try_from(frame.image.height()).ok()?;
    if w == 0 || h == 0 {
        return None;
    }
    let hx = frame.hotspot_x.min(w.saturating_sub(1));
    let hy = frame.hotspot_y.min(h.saturating_sub(1));
    let source = CustomCursor::from_rgba(frame.image.as_raw().to_vec(), w, h, hx, hy).ok()?;
    Some(event_loop.create_custom_cursor(source))
}

fn dir_index(dir: EdgeScrollDir) -> Option<usize> {
    match dir {
        EdgeScrollDir::North => Some(0),
        EdgeScrollDir::NorthEast => Some(1),
        EdgeScrollDir::East => Some(2),
        EdgeScrollDir::SouthEast => Some(3),
        EdgeScrollDir::South => Some(4),
        EdgeScrollDir::SouthWest => Some(5),
        EdgeScrollDir::West => Some(6),
        EdgeScrollDir::NorthWest => Some(7),
        EdgeScrollDir::None => None,
    }
}

fn system_battle_cursor_fallback(cur: BattlePointer) -> CursorIcon {
    match cur {
        BattlePointer::Default => CursorIcon::Default,
        BattlePointer::Deploy => CursorIcon::Cell,
        BattlePointer::Edge(EdgeScrollCursor::Default) => CursorIcon::Default,
        BattlePointer::Edge(EdgeScrollCursor::Scroll(EdgeScrollDir::North)) => CursorIcon::NResize,
        BattlePointer::Edge(EdgeScrollCursor::Scroll(EdgeScrollDir::South)) => CursorIcon::SResize,
        BattlePointer::Edge(EdgeScrollCursor::Scroll(EdgeScrollDir::East)) => CursorIcon::EResize,
        BattlePointer::Edge(EdgeScrollCursor::Scroll(EdgeScrollDir::West)) => CursorIcon::WResize,
        BattlePointer::Edge(EdgeScrollCursor::Scroll(EdgeScrollDir::NorthEast)) => CursorIcon::NeResize,
        BattlePointer::Edge(EdgeScrollCursor::Scroll(EdgeScrollDir::NorthWest)) => CursorIcon::NwResize,
        BattlePointer::Edge(EdgeScrollCursor::Scroll(EdgeScrollDir::SouthEast)) => CursorIcon::SeResize,
        BattlePointer::Edge(EdgeScrollCursor::Scroll(EdgeScrollDir::SouthWest)) => CursorIcon::SwResize,
        BattlePointer::Edge(EdgeScrollCursor::Scroll(EdgeScrollDir::None)) => CursorIcon::Default,
        BattlePointer::Edge(EdgeScrollCursor::Blocked(_)) => CursorIcon::NotAllowed,
    }
}
