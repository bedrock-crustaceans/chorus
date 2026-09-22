use bedrock::auth::auth_oidc::AuthOIDC;
use bevy_app::{App, Plugin, Startup};
use bevy_ecs::prelude::{Commands, Resource};
use tracing::debug;

#[derive(Resource)]
pub struct Auth(pub AuthOIDC);

pub struct LoginAuthOIDC;

impl Plugin for LoginAuthOIDC {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::fetch_oidc);
    }
}

impl LoginAuthOIDC {
    pub fn fetch_oidc(mut commands: Commands) {
        if let Ok(oidc) = AuthOIDC::fetch() {
            debug!("Auth OIDC fetch succeeded");
            commands.insert_resource(Auth(oidc))
        } else {
            debug!("Auth OIDC fetch failed")
        }
    }
}
