use std::collections::HashMap;

use exports::edgee::components::consent_management::{Consent, Dict, Guest};

wit_bindgen::generate!({
    path: ".edgee/wit",
    world: "consent-management",
    generate_all,
});

struct Component;
export!(Component);

impl Guest for Component {
    fn map(cookies: Dict, _settings: Dict) -> Option<Consent> {
        let cookies = match Cookies::try_from(cookies) {
            Ok(cookies) => cookies,
            Err(err) => {
                eprintln!("Could not get cookies: {err}");
                return Some(Consent::Pending);
            }
        };

        if !cookies.completed.unwrap_or(false) {
            return Some(Consent::Pending);
        }

        if let Some(gcm) = cookies.google_consent_mode {
            for (key, value) in gcm.iter() {
                if key == "version" || key == "wait_for_update" {
                    continue;
                }
                let Some(value) = value.as_str() else {
                    continue;
                };
                if value != "granted" {
                    return Some(Consent::Denied);
                }
            }
        }

        for (key, value) in cookies.extra_fields.iter() {
            if key.starts_with("$$") {
                continue;
            }
            let Some(value) = value.as_bool() else {
                continue;
            };
            if !value {
                return Some(Consent::Denied);
            }
        }

        Some(Consent::Granted)
    }
}

#[derive(serde::Deserialize)]
struct Cookies {
    #[serde(rename = "$$completed")]
    completed: Option<bool>,
    #[serde(rename = "$$googleConsentMode")]
    google_consent_mode: Option<HashMap<String, serde_json::Value>>,
    #[serde(flatten)]
    extra_fields: HashMap<String, serde_json::Value>,
}

impl TryFrom<Dict> for Cookies {
    type Error = String;

    fn try_from(value: Dict) -> Result<Self, Self::Error> {
        let dict: std::collections::HashMap<_, _> = value.into_iter().collect();

        let value = dict
            .get("axeptio_cookies")
            .filter(|s| !s.is_empty())
            .and_then(|s| urlencoding::decode(s).ok())
            .ok_or_else(|| "Cookie not found: axeptio_cookies".to_string())?;

        serde_json::from_str(value.as_ref()).map_err(|err| err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! dict {
        {
            $($key:literal: $value:expr),*$(,)?
        } => {
            vec![
                $(($key.to_string(), $value.to_string()),)*
            ]
        };
    }

    fn make_axeptio_cookie(value: serde_json::Value) -> String {
        let s = serde_json::to_string(&value).unwrap();
        urlencoding::encode(&s).to_string()
    }

    #[test]
    fn test_consent_pending() {
        let cookies = dict! {};

        assert_eq!(
            Component::map(cookies, Default::default()),
            Some(Consent::Pending)
        );
    }

    #[test]
    fn test_consent_granted() {
        let cookies = dict! {
            "axeptio_cookies": make_axeptio_cookie(serde_json::json!({
                "$$completed": true,
            })),
        };

        assert_eq!(
            Component::map(cookies, Default::default()),
            Some(Consent::Granted)
        );
    }

    #[test]
    fn test_consent_denied() {
        let cookies = dict! {
            "axeptio_cookies": make_axeptio_cookie(serde_json::json!({
                "$$completed": true,
                "granted": false,
            })),
        };

        assert_eq!(
            Component::map(cookies, Default::default()),
            Some(Consent::Denied)
        );
    }
}
