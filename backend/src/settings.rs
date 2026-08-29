use shared::extensions::settings::{
    ExtensionSettings, SettingsDeserializeExt, SettingsDeserializer, SettingsSerializeExt,
    SettingsSerializer,
};

pub const PACKAGE: &str = "net.cerbonix.ssotickets";

#[derive(Default, Clone)]
pub struct SsoSettings {
    pub shared_secret: Option<compact_str::CompactString>,
}

#[async_trait::async_trait]
impl SettingsSerializeExt for SsoSettings {
    async fn serialize(
        &self,
        serializer: SettingsSerializer,
    ) -> Result<SettingsSerializer, anyhow::Error> {
        match &self.shared_secret {
            Some(secret) => {
                serializer
                    .write_raw_encrypted_setting("shared_secret", secret.clone())
                    .await
            }
            None => Ok(serializer.write_raw_setting("shared_secret", "")),
        }
    }
}

#[async_trait::async_trait]
impl SettingsDeserializeExt for SsoSettings {
    async fn deserialize_boxed(
        &self,
        deserializer: SettingsDeserializer<'_>,
    ) -> Result<ExtensionSettings, anyhow::Error> {
        let secret = match deserializer.read_raw_encrypted_setting("shared_secret").await {
            Ok(value) => value.filter(|value| !value.is_empty()),
            Err(err) => {
                tracing::warn!("could not read the sso shared secret, treating it as unset: {err}");
                None
            }
        };

        Ok(Box::new(SsoSettings {
            shared_secret: secret,
        }))
    }
}

/// Compares without an early exit, so a caller cannot learn the secret one byte at a time from timing.
pub fn secret_matches(expected: Option<&compact_str::CompactString>, provided: &str) -> bool {
    let Some(expected) = expected else {
        return false;
    };

    let (a, b) = (expected.as_bytes(), provided.as_bytes());

    if a.len() != b.len() {
        return false;
    }

    let mut difference = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        difference |= x ^ y;
    }

    difference == 0
}
