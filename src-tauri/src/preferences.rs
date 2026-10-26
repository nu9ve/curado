use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Theme {
    pub version: u32,
    pub mode: String,
    pub variant: String,
    pub base: String,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            version: 1,
            mode: "dark".into(),
            variant: "neutral".into(),
            base: "#d66154".into(),
        }
    }
}
impl Theme {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || !["dark", "light", "system"].contains(&self.mode.as_str())
            || !["neutral", "tinted", "contrast"].contains(&self.variant.as_str())
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
        assert!((t.rgb()[0] - 214. / 255.).abs() < 0.00001);
    }
}
