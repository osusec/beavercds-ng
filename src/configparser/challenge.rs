use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Error, Result};
use rust_search::SearchBuilder;
use simplelog::*;

use crate::configparser::config::Resource;

pub fn parse_all_challenges() -> Vec<Result<ChallengeConfig, Error>> {
    // find all challenge.yaml files
    SearchBuilder::default()
        .location(".")
        .search_input("challenge.yaml")
        .build()
        // try to parse each one
        .map(|path| {
            parse_challenge(&path)
                .with_context(|| format!("failed to parse challenge config {}", path))
        })
        .collect()
}

pub fn parse_challenge(path: &str) -> Result<ChallengeConfig> {
    trace!("trying to parse {path}");

    // extract category from challenge path
    let contents = fs::read_to_string(path)?;
    let mut parsed: ChallengeConfig = serde_yaml::from_str(&contents)?;

    let category = Path::new(path)
        .components()
        .nth_back(2)
        .expect("could not find category from path");
    parsed.category = category.as_os_str().to_str().unwrap().to_owned();
    Ok(parsed)
}

//
// ==== Structs for challenge.yaml parsing ====
//

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ChallengeConfig {
    name: String,
    author: String,
    #[serde(default)]
    category: String,
    description: String,
    difficulty: i64,
    flag: FlagType,
    provide: Vec<String>,
    pods: Vec<Pod>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum FlagType {
    RawString(String),
    File(FilePath),
    Text(FileText),
    Regex(FileRegex),
    Verifier(FileVerifier),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct FilePath {
    file: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct FileText {
    text: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct FileRegex {
    regex: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct FileVerifier {
    verifier: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Pod {
    name: String,
    build: BuildSpec,
    image: String,
    env: Option<ListOrMap>,
    resources: Option<Resource>,
    replicas: i64,
    ports: Vec<PortConfig>,
    volume: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum BuildSpec {
    Context(String),
    Map(BTreeMap<String, String>),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct BuildObject {
    context: String,
    dockerfile: String,
    dockerfile_inline: String,
    args: ListOrMap,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum ListOrMap {
    List(Vec<String>),
    Map(BTreeMap<String, String>),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct PortConfig {
    internal: i64,
    expose: PortType,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum PortType {
    Tcp(TcpPort),
    Http(HttpEndpoint),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct TcpPort {
    tcp: i64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct HttpEndpoint {
    http: String,
}
