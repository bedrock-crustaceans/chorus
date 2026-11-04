mod schema;

use chorus_block_schema::{Bundle, BundledSchema};
use flate2::read::GzDecoder;
use schema::RawSchema;
use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{env, fs};

const SCHEMA_REPOSITORY: &str = "opencollab-incubator/BedrockBlockUpgradeSchema";
const SCHEMA_OUTPUT: &str = "crates/block/resources/block_upgrade_schemas.json";
const SCHEMA_DIRECTORY: &str = "nbt_upgrade_schema/";

const GAMEDATA_REPOSITORY: &str = "PowerNukkitX/Gamedata";
const GAMEDATA_REFERENCE: &str = "r/26_u6";
const GAMEDATA_DIRECTORY: &str = "src/main/resources/gamedata/kaooot/";
const ITEM_RESOURCES: &str = "crates/item/resources";
const ITEM_FILES: [(&str, bool); 3] = [("item_palette.json", false), ("creative_items.json", false), ("item_components.nbt", true)];

#[derive(Deserialize)]
struct Commit {
    sha: String,
}

#[derive(Deserialize)]
struct Tree {
    tree: Vec<TreeEntry>,
}

#[derive(Deserialize)]
struct TreeEntry {
    path: String,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("xtask lives in the workspace").to_path_buf()
}

fn get_bytes(url: &str) -> Vec<u8> {
    ureq::get(url)
        .header("User-Agent", "chorus-xtask")
        .call()
        .unwrap_or_else(|error| panic!("{url}: {error}"))
        .body_mut()
        .read_to_vec()
        .unwrap_or_else(|error| panic!("{url}: {error}"))
}

fn get(url: &str) -> String {
    String::from_utf8(get_bytes(url)).unwrap_or_else(|error| panic!("{url}: {error}"))
}

fn latest_commit(repository: &str, reference: Option<&str>) -> String {
    match reference {
        None => {
            let commit: Commit = serde_json::from_str(&get(&format!("https://api.github.com/repos/{repository}/commits/HEAD"))).expect("the commit response parses");
            commit.sha
        }
        Some(reference) => {
            let commits: Vec<Commit> = serde_json::from_str(&get(&format!("https://api.github.com/repos/{repository}/commits?sha={reference}&per_page=1"))).expect("the commit response parses");
            commits.into_iter().next().unwrap_or_else(|| panic!("{repository} has no commits on {reference}")).sha
        }
    }
}

fn schema_files(commit: &str) -> Vec<(u32, String, String)> {
    let tree: Tree = serde_json::from_str(&get(&format!("https://api.github.com/repos/{SCHEMA_REPOSITORY}/git/trees/{commit}?recursive=1"))).expect("the tree response parses");
    let mut files: Vec<(u32, String, String)> = tree
        .tree
        .into_iter()
        .filter_map(|entry| {
            let file = entry.path.strip_prefix(SCHEMA_DIRECTORY)?.strip_suffix(".json")?.to_owned();
            let (id, name) = file.split_once('_')?;
            Some((id.parse().ok()?, name.to_owned(), entry.path))
        })
        .collect();
    files.sort_by_key(|(id, ..)| *id);
    files
}

fn bundle_schemas() -> Bundle {
    let commit = latest_commit(SCHEMA_REPOSITORY, None);
    let schemas: Vec<BundledSchema> = schema_files(&commit)
        .into_iter()
        .map(|(id, name, path)| {
            let document = get(&format!("https://raw.githubusercontent.com/{SCHEMA_REPOSITORY}/{commit}/{path}"));
            let raw: RawSchema = serde_json::from_str(&document).unwrap_or_else(|error| panic!("{path}: {error}"));
            BundledSchema {
                id,
                name,
                schema: schema::convert(raw),
            }
        })
        .collect();
    assert!(!schemas.is_empty(), "no block upgrade schemas in {SCHEMA_REPOSITORY} at {commit}");
    Bundle {
        source: format!("https://github.com/{SCHEMA_REPOSITORY}"),
        commit,
        schemas,
    }
}

fn block_upgrade_schemas() {
    let bundle = bundle_schemas();
    let output = workspace_root().join(SCHEMA_OUTPUT);
    fs::create_dir_all(output.parent().expect("the output has a directory")).expect("the resources directory is writable");
    fs::write(&output, serde_json::to_string_pretty(&bundle).expect("the bundle serializes") + "\n").expect("the output is writable");
    println!("wrote {} schemas ({}) to {}", bundle.schemas.len(), bundle.commit, output.display());
}

fn gunzip(bytes: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::new();
    GzDecoder::new(bytes).read_to_end(&mut decoded).expect("the item components are gzip");
    decoded
}

fn item_data(arguments: &[String]) {
    let reference = arguments.first().map_or(GAMEDATA_REFERENCE, String::as_str);
    let commit = latest_commit(GAMEDATA_REPOSITORY, Some(reference));
    let directory = workspace_root().join(ITEM_RESOURCES);
    fs::create_dir_all(&directory).expect("the resources directory is writable");
    for (file, compressed) in ITEM_FILES {
        let bytes = get_bytes(&format!("https://raw.githubusercontent.com/{GAMEDATA_REPOSITORY}/{commit}/{GAMEDATA_DIRECTORY}{file}"));
        let bytes = if compressed { gunzip(&bytes) } else { bytes };
        fs::write(directory.join(file), &bytes).unwrap_or_else(|error| panic!("{file}: {error}"));
        println!("wrote {file} ({} bytes)", bytes.len());
    }
    println!("item data of {GAMEDATA_REPOSITORY} {reference} at {commit}");
}

fn main() {
    let arguments: Vec<String> = env::args().skip(1).collect();
    match arguments.first().map(String::as_str) {
        Some("block-upgrade-schemas") => block_upgrade_schemas(),
        Some("item-data") => item_data(&arguments[1..]),
        _ => {
            eprintln!("usage: cargo run -p xtask -- <block-upgrade-schemas | item-data [gamedata branch]>");
            std::process::exit(2);
        }
    }
}
