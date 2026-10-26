use curado_audio::Action;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Player,
    Mix,
}
impl Mode {
    pub fn allows(self, action: &Action) -> bool {
        self == Self::Mix
            || action.kind == "panic"
            || action.kind == "master"
            || action.kind == "pause"
            || (action.slot == 0 && (action.kind != "crossfader" || action.value == 0.))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_policy_for_ui_keyboard_and_midi() {
        let a = |slot, kind: &str, value| Action {
            slot,
            kind: kind.into(),
            value,
        };
        assert!(Mode::Player.allows(&a(0, "toggle", 0.)));
        assert!(!Mode::Player.allows(&a(1, "toggle", 0.)));
        assert!(!Mode::Player.allows(&a(2, "trigger", 0.)));
        assert!(Mode::Mix.allows(&a(2, "trigger", 0.)));
        assert!(!Mode::Player.allows(&a(0, "crossfader", 0.5)));
        assert!(Mode::Player.allows(&a(0, "panic", 0.)));
    }
}
