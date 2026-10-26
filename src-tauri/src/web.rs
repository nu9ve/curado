//! Remote service viewers. No capabilities, native commands, PCM bridge or script injection.
use serde::Deserialize;
use std::sync::Mutex;
use tauri::{
    webview::NewWindowResponse, AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder,
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
    fn title(self) -> &'static str {
        match self {
            Self::Fractos => "curado · fractos radio · audio web independiente",
            Self::Soundcloud => "curado · SoundCloud · audio web independiente",
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

#[tauri::command]
pub async fn open_web_service(service: Service, app: AppHandle) -> Result<(), String> {
    crate::work(move || {
        let _guard = WINDOW_GATE
            .lock()
            .map_err(|_| "No se pudo coordinar la ventana web")?;
        if let Some(window) = app.get_webview_window(service.label()) {
            window.show().map_err(|e| e.to_string())?;
            return window.set_focus().map_err(|e| e.to_string());
        }
        WebviewWindowBuilder::new(
            &app,
            service.label(),
            WebviewUrl::External(
                service
                    .url()
                    .parse()
                    .map_err(|_| "URL de servicio inválida")?,
            ),
        )
        .title(service.title())
        .inner_size(1120., 800.)
        .min_inner_size(700., 500.)
        .incognito(true)
        .devtools(false)
        .disable_drag_drop_handler()
        .on_navigation(move |url| service.allows(url))
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .on_download(|_, _| false)
        .build()
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
            if let Some(window) = app.get_webview_window(service.label()) {
                // Explicit user action: destroy the webview instead of hiding playing audio.
                window.destroy().map_err(|e| e.to_string())?;
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
