use std::env;
use std::fmt::Write;
use std::fs;
use std::path::{MAIN_SEPARATOR, PathBuf};

use walkdir::WalkDir;

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    embed_resources(&out);
    generate_biomes(&out);
}

fn embed_resources(out: &std::path::Path) {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR")).join("resources");
    println!("cargo:rerun-if-changed=resources");
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for entry in WalkDir::new(&root).into_iter().map(|entry| entry.expect("readable resource")) {
        if !entry.file_type().is_file() || entry.file_name() == "README.md" {
            continue;
        }
        let name = entry.path().strip_prefix(&root).expect("file under resources").to_string_lossy().replace(MAIN_SEPARATOR, "/");
        files.push((name, entry.into_path()));
    }
    files.sort();
    let mut source = String::from(
        "pub static CHORUS_FILES: &[(&str, &[u8])] = &[
",
    );
    for (name, path) in files {
        writeln!(source, "    ({name:?}, include_bytes!({:?})),", path.to_string_lossy()).expect("table row");
    }
    source.push_str(
        "];
",
    );
    fs::write(out.join("chorus_files.rs"), source).expect("resource table");
}

fn generate_biomes(out: &std::path::Path) {
    println!("cargo:rerun-if-changed=resources/biomes.json");
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR")).join("resources").join("biomes.json");
    let document: serde_json::Value = serde_json::from_slice(&fs::read(&root).expect("resources/biomes.json")).expect("biomes.json is json");
    let names: Vec<&str> = document["generated"]
        .as_array()
        .expect("a generated list")
        .iter()
        .map(|row| row["name"].as_str().expect("a biome name"))
        .collect();
    let mut source = String::from(
        "impl Biome {
",
    );
    for name in &names {
        writeln!(source, "    pub const {}: Biome = Biome::new({name:?});", name.to_ascii_uppercase()).expect("constant");
    }
    let all: Vec<String> = names.iter().map(|name| format!("Biome::{}", name.to_ascii_uppercase())).collect();
    writeln!(
        source,
        "    pub const ALL: [Biome; {}] = [{}];
}}",
        names.len(),
        all.join(", ")
    )
    .expect("list");
    let keys: Vec<String> = names.iter().map(|name| format!("Biome::new({name:?})")).collect();
    writeln!(
        source,
        "
static BIOME_KEYS: [Biome; {}] = [{}];",
        names.len(),
        keys.join(", ")
    )
    .expect("keys");
    source.push_str("\npub mod pack_ids {\n");
    let generated = document["generated"].as_array().expect("a generated list").iter().map(|row| {
        (
            row["pack"].as_str().unwrap_or_else(|| row["name"].as_str().expect("a biome name")),
            row["id"].as_i64().expect("a biome id"),
        )
    });
    let other = document["other"]
        .as_array()
        .expect("an other list")
        .iter()
        .map(|row| (row["pack"].as_str().expect("a pack name"), row["id"].as_i64().expect("a biome id")));
    for (pack, id) in generated.chain(other) {
        writeln!(source, "    pub const {}: i32 = {id};", pack.to_ascii_uppercase()).expect("pack id");
    }
    source.push_str("}\n");
    fs::write(out.join("biome_constants.rs"), source).expect("biome constants");
}
