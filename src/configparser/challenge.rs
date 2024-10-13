use anyhow::{Context, Error, Result};
use fully_pub::fully_pub;
use rust_search::SearchBuilder;
use serde::{Deserialize, Serialize};
use simplelog::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::configparser::config::Resource;

pub fn parse_all() -> Vec<Result<ChallengeConfig, Error>> {
    // find all challenge.yaml files
    SearchBuilder::default()
        .location(".")
        .search_input("challenge.yaml")
        .build()
        // try to parse each one
        .map(|path| {
            parse_one(&path).with_context(|| format!("failed to parse challenge config {}", path))
        })
        .collect()
}

pub fn parse_one(path: &str) -> Result<ChallengeConfig> {
    trace!("trying to parse {path}");

    // extract category from challenge path
    let contents = fs::read_to_string(path)?;
    let mut parsed: ChallengeConfig = serde_yaml::from_str(&contents)?;

    let category = Path::new(path)
        .components()
        .nth_back(2)
        .expect("could not find category from path");
    category
        .as_os_str()
        .to_str()
        .unwrap()
        .clone_into(&mut parsed.category);
    Ok(parsed)
}

//
// ==== Structs for challenge.yaml parsing ====
//

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct ChallengeConfig {
    name: String,
    author: String,

    #[serde(default)]
    category: String,

    description: String,
    difficulty: i64,
    flag: FlagType,

    #[serde(default)]
    provide: Vec<String>, // optional if no files provided

    #[serde(default)]
    pods: Vec<Pod>, // optional if no containers used
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[fully_pub]
enum FlagType {
    RawString(String),
    File(FilePath),
    Text(FileText),
    Regex(FileRegex),
    Verifier(FileVerifier),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct FilePath {
    file: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct FileText {
    text: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct FileRegex {
    regex: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct FileVerifier {
    verifier: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
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
#[fully_pub]
enum BuildSpec {
    Context(String),
    Map(BTreeMap<String, String>),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct BuildObject {
    context: String,
    dockerfile: String,
    dockerfile_inline: String,
    args: ListOrMap,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[fully_pub]
enum ListOrMap {
    List(Vec<String>),
    Map(BTreeMap<String, String>),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct PortConfig {
    internal: i64,
    expose: PortType,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[fully_pub]
enum PortType {
    Tcp(TcpPort),
    Http(HttpEndpoint),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct TcpPort {
    tcp: i64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct HttpEndpoint {
    http: String,
}
