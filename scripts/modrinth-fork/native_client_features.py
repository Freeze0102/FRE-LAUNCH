"""Compile FRE's reviewed client extensions against the pinned hook points."""
from pathlib import Path

SOURCES = {"game/fre_client_mods.rs": "native_client_mods.rs",
           "game/fre_mod_settings.rs": "fre_client_mods.rs"}

def apply(name, text, patch):
    if name == "crates/horizon-client/src/game.rs":
        text = patch(text, "mod advancements;", "mod advancements;\nmod fre_client_mods;\nmod fre_mod_settings;")
        text = patch(text, "                        && !self.native_wheel_event(d)",
                     "                        && !self.fre_client_scroll(d)\n                        && !self.native_wheel_event(d)")
    elif name == "crates/horizon-client/src/game/native_hooks.rs":
        text = patch(text, "        let _ = (batch, corners);", "        self.fre_client_hud(batch, corners);")
        text = patch(text, "        let _ = zoom;", "        self.fre_client_zoom(zoom);")
    elif name == "crates/horizon-client/src/game/control.rs":
        text = patch(text, 'status["lod"] = lod;',
            'status["freClientMods"] = self.fre_client_mod_status();\n        status["lod"] = lod;')
    return text
