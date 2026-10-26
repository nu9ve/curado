//! Remote service viewers. No capabilities, native commands, PCM bridge or script injection.
use serde::Deserialize;
use std::sync::Mutex;
use tauri::{
    webview::{NewWindowResponse, WebviewBuilder},
    AppHandle, LogicalPosition, LogicalSize, Manager, Rect, Url, WebviewUrl,
};

static WINDOW_GATE: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Service {
    Fractos,
    Soundcloud,
}
impl Service {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fractos => "web-fractos",
            Self::Soundcloud => "web-soundcloud",
        }
    }
    pub fn url(self) -> &'static str {
        match self {
            Self::Fractos => "https://www.fractos.app/es/radio",
            Self::Soundcloud => "https://soundcloud.com/discover",
        }
    }
    pub fn allows(self, url: &Url) -> bool {
        // Embedded players may initialize an empty frame. Never allow local app URLs.
        if matches!(url.as_str(), "about:blank" | "about:srcdoc") {
            return true;
        }
        if url.scheme() != "https"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_some_and(|port| port != 443)
        {
            return false;
        }
        let Some(host) = url.host_str() else {
            return false;
        };
        match self {
            Self::Fractos => matches!(
                host,
                "fractos.app"
                    | "www.fractos.app"
                    | "w.soundcloud.com"
                    | "soundcloud.com"
                    | "www.soundcloud.com"
                    | "youtube.com"
                    | "www.youtube.com"
                    | "youtube-nocookie.com"
                    | "www.youtube-nocookie.com"
                    | "consent.youtube.com"
            ),
            Self::Soundcloud => matches!(
                host,
                "soundcloud.com"
                    | "www.soundcloud.com"
                    | "secure.soundcloud.com"
                    | "w.soundcloud.com"
            ),
        }
    }
}

/// Defense in depth: app commands are also restricted to main by the generated ACL.
pub fn allows_native_commands(label: &str) -> bool {
    label == "main"
}
pub fn hide_on_close(label: &str, has_tray: bool) -> bool {
    label == "main" && has_tray
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Bounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    visible: bool,
}
impl Bounds {
    fn validate(self) -> Result<Self, String> {
        if [self.x, self.y, self.width, self.height]
            .iter()
            .any(|v| !v.is_finite())
            || self.x < 0.
            || self.y < 0.
            || self.width < 1.
            || self.height < 1.
            || self.x + self.width > 32768.
            || self.y + self.height > 32768.
        {
            return Err("Área del navegador inválida".into());
        }
        Ok(self)
    }
    fn rect(self) -> Rect {
        Rect {
            position: LogicalPosition::new(self.x, self.y).into(),
            size: LogicalSize::new(self.width, self.height).into(),
        }
    }
}

#[tauri::command]
pub async fn open_web_service(
    service: Service,
    bounds: Bounds,
    app: AppHandle,
) -> Result<(), String> {
    let bounds = bounds.validate()?;
    crate::work(move || {
        let _guard = WINDOW_GATE
            .lock()
            .map_err(|_| "No se pudo coordinar el navegador")?;
        for other in [Service::Fractos, Service::Soundcloud] {
            if other != service {
                if let Some(view) = app.get_webview(other.label()) {
                    view.close().map_err(|e| e.to_string())?;
                }
            }
        }
        if let Some(view) = app.get_webview(service.label()) {
            view.set_bounds(bounds.rect()).map_err(|e| e.to_string())?;
            return if bounds.visible {
                view.show()
            } else {
                view.hide()
            }
            .map_err(|e| e.to_string());
        }
        if !bounds.visible {
            return Ok(());
        }
        let window = app
            .get_window("main")
            .ok_or("Ventana principal no disponible")?;
        let view = WebviewBuilder::new(
            service.label(),
            WebviewUrl::External(service.url().parse().map_err(|_| "URL inválida")?),
        )
        .incognito(true)
        .devtools(false)
        .disable_drag_drop_handler()
        .on_navigation(move |url| service.allows(url))
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .on_download(|_, _| false);
        window
            .add_child(
                view,
                LogicalPosition::new(bounds.x, bounds.y),
                LogicalSize::new(bounds.width, bounds.height),
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn close_web_services(app: AppHandle) -> Result<usize, String> {
    crate::work(move || {
        let _guard = WINDOW_GATE
            .lock()
            .map_err(|_| "No se pudo coordinar la ventana web")?;
        let mut closed = 0;
        for service in [Service::Fractos, Service::Soundcloud] {
            if let Some(window) = app.get_webview(service.label()) {
                // Removing the child webview stops its independent playback.
                window.close().map_err(|e| e.to_string())?;
                closed += 1;
            }
        }
        Ok(closed)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn service_policy_accepts_canonical_redirects_and_required_embedded_players() {
        for service in [Service::Fractos, Service::Soundcloud] {
            assert!(service.allows(&service.url().parse().unwrap()));
            assert!(service.allows(&"about:blank".parse().unwrap()));
        }
        for url in [
            "https://fractos.app/es/radio",
            "https://www.fractos.app/en/radio",
            "https://w.soundcloud.com/player/",
            "https://www.youtube-nocookie.com/embed/id",
        ] {
            assert!(Service::Fractos.allows(&url.parse().unwrap()), "{url}");
        }
        assert!(
            Service::Soundcloud.allows(&"https://secure.soundcloud.com/sign-in".parse().unwrap())
        );
    }
    #[test]
    fn remote_navigation_rejects_local_schemes_spoofed_hosts_credentials_and_other_ports() {
        for service in [Service::Fractos, Service::Soundcloud] {
            for url in [
                "http://fractos.app/es/radio",
                "https://fractos.app.evil.test/",
                "https://fractos.app@evil.test/",
                "https://user:pass@fractos.app/",
                "https://fractos.app:444/",
                "https://example.com/",
                "file:///tmp/audio.wav",
                "tauri://localhost/",
                "https://tauri.localhost/",
                "http://localhost:1420",
                "data:text/html,test",
                "javascript:alert(1)",
                "blob:https://fractos.app/id",
            ] {
                assert!(!service.allows(&url.parse().unwrap()), "{url}");
            }
        }
    }
    #[test]
    fn embedded_bounds_reject_nonfinite_negative_empty_and_oversized_rects() {
        let valid = Bounds {
            x: 200.,
            y: 140.,
            width: 800.,
            height: 500.,
            visible: true,
        };
        assert!(valid.validate().is_ok());
        for invalid in [
            Bounds { x: -1., ..valid },
            Bounds { width: 0., ..valid },
            Bounds {
                height: f64::NAN,
                ..valid
            },
            Bounds {
                y: f64::INFINITY,
                ..valid
            },
            Bounds {
                width: 40000.,
                ..valid
            },
        ] {
            assert!(invalid.validate().is_err());
        }
    }
    #[test]
    fn only_the_main_window_can_invoke_commands_or_hide_to_tray() {
        assert!(allows_native_commands("main"));
        assert!(hide_on_close("main", true));
        assert!(!hide_on_close("main", false));
        for label in ["web-fractos", "web-soundcloud", "main-child", ""] {
            assert!(!allows_native_commands(label));
            assert!(!hide_on_close(label, true));
        }
    }
}
