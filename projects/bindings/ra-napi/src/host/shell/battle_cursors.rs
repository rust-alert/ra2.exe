//! 对局软件光标：`mouse.shp` 帧 → winit `CustomCursor`。
//!
//! 覆盖默认 / 点选 / 移动 / 禁止移动 / 攻击 / 部署 / 禁止部署 / 边缘滚屏。

use std::time::{SystemTime, UNIX_EPOCH};

use ra_widgets::battle_order_icons::{
    DecodedBattleEdgeCursors, DecodedMouseCursorFrame, MOUSE_CURSOR_ANIM_MS, MOUSE_SCROLL_DIR_COUNT, load_battle_edge_cursors,
};
use winit::{
    event_loop::ActiveEventLoop,
    window::{Cursor, CursorIcon, CustomCursor},
};

use crate::host::battle_input::{BattlePointer, EdgeScrollCursor, EdgeScrollDir};

use super::Shell;

/// 已上传到平台的对局 `mouse.shp` 软件光标。
pub(crate) struct BattleMouseCursorSet {
    default: CustomCursor,
    select: Vec<CustomCursor>,
    move_ok: Vec<CustomCursor>,
    no_move: CustomCursor,
    attack: Vec<CustomCursor>,
    deploy: Vec<CustomCursor>,
    no_deploy: CustomCursor,
    scroll: [CustomCursor; MOUSE_SCROLL_DIR_COUNT],
    blocked: [CustomCursor; MOUSE_SCROLL_DIR_COUNT],
}

impl Shell {
    /// 从安装资源装入对局软件光标（每进程一次；失败则继续用系统占位）。
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
                tracing::info!(
                    "对局软件光标 · select#{} move#{} attack#{} deploy#{} + scroll/blocked · mouse.shp",
                    set.select.len(),
                    set.move_ok.len(),
                    set.attack.len(),
                    set.deploy.len()
                );
                self.battle_mouse_cursors = Some(set);
                self.battle_pointer = BattlePointer::Default;
                self.battle_pointer_anim_frame = u32::MAX;
            }
            None => tracing::warn!("对局软件光标 · CustomCursor 创建失败"),
        }
    }

    pub(super) fn sync_battle_edge_cursor(&mut self) {
        let Some(window) = self.window.clone()
        else {
            return;
        };
        let cur = if self.screen == ra_widgets::original_screen::OriginalScreen::Battle {
            self.battle_controller.as_ref().map(|c| c.battle_pointer(&self.renderer, &window)).unwrap_or(BattlePointer::Default)
        }
        else {
            BattlePointer::Default
        };
        self.apply_battle_pointer(cur);
    }

    pub(super) fn apply_battle_pointer(&mut self, cur: BattlePointer) {
        let anim = anim_frame_index(cur, self.battle_mouse_cursors.as_ref());
        if cur == self.battle_pointer && anim == self.battle_pointer_anim_frame {
            return;
        }
        let Some(window) = self.window.as_ref()
        else {
            return;
        };
        let cursor = self.resolve_battle_cursor(cur, anim);
        window.set_cursor(cursor);
        self.battle_pointer = cur;
        self.battle_pointer_anim_frame = anim;
    }

    fn resolve_battle_cursor(&self, cur: BattlePointer, anim: u32) -> Cursor {
        if let Some(set) = self.battle_mouse_cursors.as_ref() {
            if let Some(c) = set.cursor_for(cur, anim as usize) {
                return Cursor::Custom(c);
            }
        }
        Cursor::Icon(system_battle_cursor_fallback(cur))
    }
}

impl BattleMouseCursorSet {
    fn from_decoded(event_loop: &ActiveEventLoop, decoded: &DecodedBattleEdgeCursors) -> Option<Self> {
        Some(Self {
            default: create_custom_cursor(event_loop, &decoded.default)?,
            select: create_seq_cursors(event_loop, &decoded.select)?,
            move_ok: create_seq_cursors(event_loop, &decoded.move_ok)?,
            no_move: create_custom_cursor(event_loop, &decoded.no_move)?,
            attack: create_seq_cursors(event_loop, &decoded.attack)?,
            deploy: create_seq_cursors(event_loop, &decoded.deploy)?,
            no_deploy: create_custom_cursor(event_loop, &decoded.no_deploy)?,
            scroll: create_dir_cursors(event_loop, &decoded.scroll)?,
            blocked: create_dir_cursors(event_loop, &decoded.blocked)?,
        })
    }

    fn cursor_for(&self, cur: BattlePointer, anim: usize) -> Option<CustomCursor> {
        match cur {
            BattlePointer::Default | BattlePointer::Edge(EdgeScrollCursor::Default) => Some(self.default.clone()),
            BattlePointer::Select => pick_anim(&self.select, anim),
            BattlePointer::Move => pick_anim(&self.move_ok, anim),
            BattlePointer::NoMove => Some(self.no_move.clone()),
            BattlePointer::Attack => pick_anim(&self.attack, anim),
            BattlePointer::Deploy => pick_anim(&self.deploy, anim),
            BattlePointer::NoDeploy => Some(self.no_deploy.clone()),
            BattlePointer::Edge(EdgeScrollCursor::Scroll(dir)) => dir_index(dir).map(|i| self.scroll[i].clone()),
            BattlePointer::Edge(EdgeScrollCursor::Blocked(dir)) => dir_index(dir).map(|i| self.blocked[i].clone()),
        }
    }

    fn anim_len(&self, cur: BattlePointer) -> usize {
        match cur {
            BattlePointer::Select => self.select.len().max(1),
            BattlePointer::Move => self.move_ok.len().max(1),
            BattlePointer::Attack => self.attack.len().max(1),
            BattlePointer::Deploy => self.deploy.len().max(1),
            _ => 1,
        }
    }
}

fn pick_anim(frames: &[CustomCursor], anim: usize) -> Option<CustomCursor> {
    if frames.is_empty() {
        return None;
    }
    Some(frames[anim % frames.len()].clone())
}

fn anim_frame_index(cur: BattlePointer, set: Option<&BattleMouseCursorSet>) -> u32 {
    let len = set.map(|s| s.anim_len(cur)).unwrap_or(1).max(1);
    if len <= 1 {
        return 0;
    }
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    ((ms / MOUSE_CURSOR_ANIM_MS) as usize % len) as u32
}

fn create_seq_cursors(event_loop: &ActiveEventLoop, frames: &[DecodedMouseCursorFrame]) -> Option<Vec<CustomCursor>> {
    let mut out = Vec::with_capacity(frames.len());
    for frame in frames {
        out.push(create_custom_cursor(event_loop, frame)?);
    }
    if out.is_empty() { None } else { Some(out) }
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

fn create_custom_cursor(event_loop: &ActiveEventLoop, frame: &DecodedMouseCursorFrame) -> Option<CustomCursor> {
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
        BattlePointer::Default | BattlePointer::Edge(EdgeScrollCursor::Default) => CursorIcon::Default,
        BattlePointer::Select => CursorIcon::Pointer,
        BattlePointer::Move => CursorIcon::Crosshair,
        BattlePointer::NoMove | BattlePointer::NoDeploy => CursorIcon::NotAllowed,
        BattlePointer::Attack => CursorIcon::Crosshair,
        BattlePointer::Deploy => CursorIcon::Cell,
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
