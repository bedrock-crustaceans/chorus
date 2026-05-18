use std::fmt;
use std::fs;
use std::io::Read;
use std::path::Path;

use bedrock::addon::manifest::AddonManifest;
use bedrock::addon::version::AddonSemanticVersion;
use uuid::Uuid;

pub const CHUNK_SIZE: u64 = 1024 * 1024;

#[derive(Clone, Debug)]
pub struct ResourcePack {
    pub uuid: Uuid,
    pub name: String,
    pub version: String,
    pub has_scripts: bool,
    data: Vec<u8>,
}

#[derive(Debug)]
pub enum ResourcePackError {
    Io(std::io::Error),
    Zip(zip::result::ZipError),
    Json(serde_json::Error),
    MissingManifest,
}

impl fmt::Display for ResourcePackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Zip(e) => write!(f, "zip error: {e}"),
            Self::Json(e) => write!(f, "JSON error: {e}"),
            Self::MissingManifest => write!(f, "manifest.json not found in archive"),
        }
    }
}

impl ResourcePack {
    pub fn from_zip(path: impl AsRef<Path>) -> Result<Self, ResourcePackError> {
        let data = fs::read(path.as_ref()).map_err(ResourcePackError::Io)?;

        let cursor = std::io::Cursor::new(&data);
        let mut archive = zip::ZipArchive::new(cursor).map_err(ResourcePackError::Zip)?;

        let manifest_str = {
            let mut file = archive.by_name("manifest.json").map_err(|_| ResourcePackError::MissingManifest)?;
            let mut s = String::new();
            file.read_to_string(&mut s).map_err(ResourcePackError::Io)?;
            s
        };

        let manifest: AddonManifest = serde_json::from_str(&manifest_str).map_err(ResourcePackError::Json)?;

        let version = format_version(&manifest.header.version);
        let has_scripts = manifest.modules.iter().any(|m| m.module_type == "script");

        Ok(Self {
            uuid: manifest.header.uuid,
            name: manifest.header.name,
            version,
            has_scripts,
            data,
        })
    }

    /// Returns the identifier used in protocol chunk requests: `{uuid}_{version}`.
    pub fn pack_id(&self) -> String {
        format!("{}_{}", self.uuid, self.version)
    }

    pub fn size(&self) -> u64 {
        self.data.len() as u64
    }

    pub fn chunk_count(&self) -> u32 {
        self.size().div_ceil(CHUNK_SIZE) as u32
    }

    pub fn get_chunk(&self, chunk_id: u32) -> &[u8] {
        let start = (chunk_id as u64 * CHUNK_SIZE) as usize;
        let end = (start + CHUNK_SIZE as usize).min(self.data.len());
        &self.data[start..end]
    }
}

fn format_version(ver: &AddonSemanticVersion) -> String {
    match ver {
        AddonSemanticVersion::Vector([a, b, c]) => format!("{a}.{b}.{c}"),
        AddonSemanticVersion::SemVer(v) => format!("{}.{}.{}", v.major, v.minor, v.patch),
    }
}
