use shared::{
    State,
    extensions::{Extension, ExtensionRouteBuilder, settings::ExtensionSettingsDeserializer},
};
use std::sync::Arc;

mod routes;
mod settings;

#[derive(Default)]
pub struct ExtensionStruct;

#[async_trait::async_trait]
impl Extension for ExtensionStruct {
    async fn initialize(&mut self, _state: State) {
        tracing::info!("clientxcms sso extension loaded");
    }

    /// Mounted under /api/auth: the issuing call authenticates itself, and the consuming one runs before any session exists.
    async fn initialize_router(
        &mut self,
        state: State,
        builder: ExtensionRouteBuilder,
    ) -> ExtensionRouteBuilder {
        let admin_state = state.clone();

        builder
            .add_auth_api_router(|router| router.nest("/ssotickets", routes::router(&state)))
            .add_admin_api_router(|router| {
                router.nest("/ssotickets/secret", routes::admin_router(&admin_state))
            })
    }

    async fn initialize_permissions(
        &mut self,
        _state: State,
        builder: shared::extensions::ExtensionPermissionsBuilder,
    ) -> shared::extensions::ExtensionPermissionsBuilder {
        let mut permissions = indexmap::IndexMap::new();
        permissions.insert("manage", "Configure the shared secret used to issue login tickets.");

        builder.add_admin_permission_group(
            "ssotickets",
            shared::permissions::PermissionGroup {
                description: "ClientXCMS SSO",
                permissions,
            },
        )
    }

    async fn settings_deserializer(&self, _state: State) -> ExtensionSettingsDeserializer {
        Arc::new(settings::SsoSettings::default())
    }
}
