pub mod auth;

use crate::login::auth::Auth;
use crate::session::LoginConfig;
use crate::transport::SessionId;
use bedrock::network::compression::Compression;
use bedrock::network::login::{LoginOptions, NetworkSettings};
use chorus_core::config::NetworkCompression;
use chorus_core::config::{NetworkConfig, ServerConfig};

pub fn login_config(server: &ServerConfig, network: &NetworkConfig, oidc: Option<&Auth>, id: &SessionId) -> LoginConfig {
    let options = LoginOptions::default().require_authentication(server.authentication);
    let options = match oidc {
        Some(auth) => options.oidc(auth.0.clone()),
        None => options,
    };

    LoginConfig {
        options,
        settings: NetworkSettings::new(match network.compression {
            NetworkCompression::Zlib => Compression::Zlib {
                threshold: network.zlib.threshold,
                compression_level: network.zlib.level,
            },
            NetworkCompression::Snappy => Compression::Snappy { threshold: network.snappy.threshold },
            NetworkCompression::None => Compression::None,
        }),
        encryption: id.supports_encryption(),
    }
}
