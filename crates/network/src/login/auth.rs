use bedrock::auth::AuthOIDC;
use bedrock::auth::http::ureq_agent;
use bevy_app::{App, Plugin, Startup};
use bevy_ecs::prelude::{Commands, Res, Resource};
use chorus_core::config::ServerConfig;
use std::sync::Arc;
use tracing::debug;

#[derive(Resource)]
pub struct Auth(pub Arc<AuthOIDC>);

pub struct LoginAuthOIDC;

impl Plugin for LoginAuthOIDC {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::fetch_oidc);
    }
}

impl LoginAuthOIDC {
    pub fn fetch_oidc(server: Res<ServerConfig>, mut commands: Commands) {
        if !server.authentication {
            debug!("Skipping Auth OIDC fetch, authentication is disabled");
            return;
        }

        if let Ok(oidc) = AuthOIDC::fetch(&ureq_agent()) {
            debug!("Auth OIDC fetch succeeded");
            commands.insert_resource(Auth(Arc::new(oidc)))
        } else {
            debug!("Auth OIDC fetch failed")
        }
    }
}
