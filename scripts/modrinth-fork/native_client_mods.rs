//! FRE LAUNCH's reviewed client mods, compiled into the native game.
use super::{Game, fre_mod_settings::{self, ClientMods, ZoomKey}, native_hooks::NativeZoom};
use fre_client::{extras::Corner, extras_hud::Corners, hud::HudBatch};
use std::{sync::{Mutex, OnceLock}, time::{Duration, Instant}};
use winit::keyboard::KeyCode;

struct LiveSettings {
    config: ClientMods,
    checked: Instant,
    factor: f32,
    held: bool,
}
static SETTINGS: OnceLock<Mutex<LiveSettings>> = OnceLock::new();
fn settings() -> std::sync::MutexGuard<'static, LiveSettings> {
    let mut state = SETTINGS.get_or_init(|| {
        let config = fre_mod_settings::read(&Game::game_dir()).unwrap_or_default();
        Mutex::new(LiveSettings { factor: config.zoom_factor.into(), config, checked: Instant::now(), held: false })
    }).lock().unwrap_or_else(|e| e.into_inner());
    if state.checked.elapsed() >= Duration::from_millis(250) {
        state.checked = Instant::now();
        // Keep the last valid settings during an atomic replacement or a bad external edit.
        if let Ok(config) = fre_mod_settings::read(&Game::game_dir()) {
            if state.config != config { state.factor = config.zoom_factor.into(); state.config = config; }
        }
    }
    state
}
impl Game<'_> {
    pub(super) fn fre_client_zoom(&self, zoom: &mut Option<NativeZoom>) {
        if zoom.is_some() { return; }
        let mut state = settings();
        let held = self.fre_zoom_held(&state.config);
        if !held { state.factor = state.config.zoom_factor.into(); }
        state.held = held;
        if state.config.zoom_now(held, self.native_playing(), self.scoping()).is_some() {
            let factor = state.factor;
            *zoom = Some(NativeZoom { factor, smooth: true, turn: 1.0 / factor });
        }
    }
    fn fre_zoom_held(&self, config: &ClientMods) -> bool {
        let key = match config.zoom_key { ZoomKey::C => KeyCode::KeyC,
            ZoomKey::V => KeyCode::KeyV, ZoomKey::Z => KeyCode::KeyZ };
        self.physical_keys.contains(&key)
    }
    pub(super) fn fre_client_scroll(&self, delta: f32) -> bool {
        let mut state = settings();
        if state.config.zoom_now(self.fre_zoom_held(&state.config), self.native_playing(), self.scoping()).is_none() {
            return false;
        }
        state.factor = fre_mod_settings::scroll_zoom(state.factor, delta);
        true // Consume the wheel: zooming must not change the selected hotbar slot.
    }
    pub(super) fn fre_client_hud(&self, batch: &mut HudBatch, corners: &mut Corners) {
        let state = settings();
        let config = &state.config;
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
        let config = settings().config.clone();
        serde_json::json!({"settings": config,
            "zoomFactor": self.native_zoom_now().map_or(1.0, |zoom| zoom.factor)})
    }
}
