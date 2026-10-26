use serde::{Deserialize, Serialize};
fn style() -> String {
    "curado".into()
}
fn auto() -> String {
    "auto".into()
}
fn bands() -> String {
    "bands".into()
}
/// Mirrors `ThemeSettings` in `src/lib/theme.ts`. v1 rows (pre-styles) deserialize with defaults.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Theme {
    pub version: u32,
    pub mode: String,
    pub variant: String,
    pub base: String,
    #[serde(default = "style")]
    pub style: String,
    #[serde(default = "auto")]
    pub typography: String,
    #[serde(default = "auto")]
    pub texture: String,
    #[serde(default = "bands")]
    pub waveform: String,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            version: 2,
            mode: "dark".into(),
            variant: "neutral".into(),
            base: "#d66154".into(),
            style: style(),
            typography: auto(),
            texture: auto(),
            waveform: bands(),
        }
    }
}
impl Theme {
    pub fn validate(&self) -> Result<(), String> {
        let one_of = |value: &str, allowed: &[&str]| allowed.contains(&value);
        if !(1..=2).contains(&self.version)
            || !one_of(&self.mode, &["dark", "light", "system"])
            || !one_of(&self.variant, &["neutral", "tinted", "contrast"])
            || !one_of(
                &self.style,
                &["curado", "pixel", "gothic", "y2k", "minimal"],
            )
            || !one_of(
                &self.typography,
                &["auto", "display", "pixel", "system", "mono"],
            )
            || !one_of(&self.texture, &["auto", "dither", "smooth", "flat"])
            || !one_of(&self.waveform, &["bands", "rgb", "mono"])
            || self.base.len() != 7
            || !self.base.starts_with('#')
            || !self.base.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
        {
            return Err("Configuración de tema inválida".into());
        }
        Ok(())
    }
    pub fn rgb(&self) -> [f32; 3] {
        let bytes = self.base.as_bytes();
        std::array::from_fn(|i| {
            u8::from_str_radix(
                std::str::from_utf8(&bytes[1 + i * 2..3 + i * 2]).unwrap_or("00"),
                16,
            )
            .unwrap_or(0) as f32
                / 255.
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_and_untrusted_preferences() {
        let t = Theme::default();
        assert_eq!(t.base, "#d66154");
        t.validate().unwrap();
        let mut bad = t.clone();
        bad.base = "#💥54".into();
        assert!(bad.validate().is_err());
        bad.base = "#d66154".into();
        bad.mode = "unknown".into();
        assert!(bad.validate().is_err());
        let mut bad = t.clone();
        bad.style = "vaporwave".into();
        assert!(bad.validate().is_err());
        assert!((t.rgb()[0] - 214. / 255.).abs() < 0.00001);
    }
    #[test]
    fn v1_preferences_upgrade_with_defaults() {
        let v1: Theme = serde_json::from_str(
            r##"{"version":1,"mode":"light","variant":"tinted","base":"#548bbf"}"##,
        )
        .unwrap();
        v1.validate().unwrap();
        assert_eq!(v1.style, "curado");
        assert_eq!(v1.waveform, "bands");
        assert_eq!(v1.base, "#548bbf");
    }
}
