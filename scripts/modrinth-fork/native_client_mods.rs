//! FRE LAUNCH's reviewed client mods, compiled into the native game.
use super::{Game, fre_mod_settings::{self, ClientMods, ZoomKey}, native_hooks::NativeZoom};
use fre_client::{extras::Corner, extras_hud::Corners, hud::HudBatch};
use std::sync::OnceLock;
use winit::keyboard::KeyCode;

static SETTINGS: OnceLock<ClientMods> = OnceLock::new();
fn settings() -> &'static ClientMods {
    SETTINGS.get_or_init(|| fre_mod_settings::read(&Game::game_dir()).unwrap_or_else(|error| {
        log::warn!("FRE LAUNCH client mods disabled: {error}");
        ClientMods::disabled()
    }))
}
impl Game<'_> {
    pub(super) fn fre_client_zoom(&self, zoom: &mut Option<NativeZoom>) {
        if zoom.is_some() { return; }
        let config = settings();
        let key = match config.zoom_key { ZoomKey::C => KeyCode::KeyC,
            ZoomKey::V => KeyCode::KeyV, ZoomKey::Z => KeyCode::KeyZ };
        if let Some(factor) = config.zoom_now(self.physical_keys.contains(&key), self.native_playing(), self.scoping()) {
            *zoom = Some(NativeZoom { factor, smooth: true, turn: 1.0 / factor });
        }
    }
    pub(super) fn fre_client_hud(&self, batch: &mut HudBatch, corners: &mut Corners) {
        let config = settings();
        let Some(player) = self.native_player() else { return; };
        let corner = Corner::TopLeft;
        if !self.native_window() || !self.native_corner_shown(corner) { return; }
        let mut lines = Vec::new();
        if config.fps_counter { lines.push(format!("FPS: {}", self.native_frame_rate().0)); }
        if config.coordinates {
            lines.push(format!("XYZ: {} / {} / {}", player.x.floor() as i64,
                player.y.floor() as i64, player.z.floor() as i64));
        }
        if lines.is_empty() { return; }
        let width = lines.iter().map(|line| self.native_text_width(line)).fold(0.0, f32::max) + 8.0;
        let height = lines.len() as f32 * 11.0 + 6.0;
        let (x, y) = corners.place(corner, width, height);
        self.native_fill(batch, [x, y, width, height], 0x78000000);
        for (row, text) in lines.iter().enumerate() {
            self.native_text(batch, text, [x + 4.0, y + 3.0 + row as f32 * 11.0], 0xffffff, true);
        }
    }
    pub(super) fn fre_client_mod_status(&self) -> serde_json::Value {
        serde_json::json!({"settings": settings(),
            "zoomFactor": self.native_zoom_now().map_or(1.0, |zoom| zoom.factor)})
    }
}
