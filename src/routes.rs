use leptos::WriteSignal;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppRoute {
    Dashboard,
    Services,
    ServiceDetail { key: String },
    Settings,
}

impl AppRoute {
    pub fn from_hash(hash: &str) -> Self {
        let path = hash
            .strip_prefix('#')
            .unwrap_or(hash)
            .strip_prefix('/')
            .unwrap_or(hash.trim_start_matches('#'))
            .trim_matches('/');

        if path.is_empty() || path.eq_ignore_ascii_case("dashboard") {
            return Self::Dashboard;
        }

        if path.eq_ignore_ascii_case("services") {
            return Self::Services;
        }

        if path.eq_ignore_ascii_case("settings") {
            return Self::Settings;
        }

        if let Some(key) = path.strip_prefix("services/") {
            let key = key.trim();

            if !key.is_empty() {
                return Self::ServiceDetail {
                    key: key.to_owned(),
                };
            }
        }

        Self::Dashboard
    }

    pub fn current() -> Self {
        current_hash().map_or(Self::Dashboard, |hash| Self::from_hash(&hash))
    }

    pub fn href(&self) -> String {
        match self {
            Self::Dashboard => "#/dashboard".to_owned(),
            Self::Services => "#/services".to_owned(),
            Self::ServiceDetail { key } => format!("#/services/{key}"),
            Self::Settings => "#/settings".to_owned(),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Services => "Services",
            Self::ServiceDetail { .. } => "Service detail",
            Self::Settings => "Settings",
        }
    }

    pub fn is_services_section(&self) -> bool {
        matches!(self, Self::Services | Self::ServiceDetail { .. })
    }
}

pub fn install_hash_route_listener(set_route: WriteSignal<AppRoute>) {
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::SignalSet;
        use wasm_bindgen::{JsCast, closure::Closure};

        let Some(window) = web_sys::window() else {
            return;
        };

        let callback = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
            set_route.set(AppRoute::current());
        }));

        let _ = window
            .add_event_listener_with_callback("hashchange", callback.as_ref().unchecked_ref());
        callback.forget();
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = set_route;
    }
}

#[cfg(target_arch = "wasm32")]
fn current_hash() -> Option<String> {
    web_sys::window()
        .map(|window| window.location().hash().unwrap_or_default())
        .filter(|hash| !hash.is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
fn current_hash() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::AppRoute;

    #[test]
    fn parses_hash_routes() {
        assert_eq!(AppRoute::from_hash("#/dashboard"), AppRoute::Dashboard);
        assert_eq!(AppRoute::from_hash("#/services"), AppRoute::Services);
        assert_eq!(
            AppRoute::from_hash("#/services/s3"),
            AppRoute::ServiceDetail {
                key: "s3".to_owned()
            }
        );
        assert_eq!(AppRoute::from_hash("#/settings"), AppRoute::Settings);
        assert_eq!(AppRoute::from_hash("#/unknown"), AppRoute::Dashboard);
    }
}
