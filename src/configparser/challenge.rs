use anyhow::{Context, Error, Result};
use fully_pub::fully_pub;
use rust_search::SearchBuilder;
use serde::{Deserialize, Serialize};
use simplelog::*;
use std::collections::HashMap as Map;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use void::Void;

use crate::configparser::config::Resource;
use crate::configparser::field_coersion::string_or_struct;

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
    debug!("trying to parse {path}");

    // extract category from challenge path
    let contents = fs::read_to_string(path)?;
    let mut parsed: ChallengeConfig = serde_yaml::from_str(&contents)?;

    // safe to unwrap here since path from find() always has the challenge yaml
    let pathobj = Path::new(path).parent().unwrap();
    parsed.directory = pathobj.strip_prefix("./").unwrap_or(pathobj).to_path_buf();

    let category = parsed
        .directory
        .components()
        .nth_back(1)
        .expect("could not find category from path");
    category
        .as_os_str()
        .to_str()
        .unwrap()
        .clone_into(&mut parsed.category);

    trace!("got chal: {parsed:#?}");

    Ok(parsed)
}

//
// ==== Structs for challenge.yaml parsing ====
//

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
pub struct ChallengeConfig {
    name: String,
    author: String,
    description: String,

    #[serde(default)]
    directory: PathBuf,

    #[serde(default)]
    category: String,

    #[serde(default = "default_difficulty")]
    difficulty: i64,

    flag: FlagType,

    #[serde(default)]
    provide: Vec<String>, // optional if no files provided

    #[serde(default)]
    pods: Vec<Pod>, // optional if no containers used
}

fn default_difficulty() -> i64 {
    1
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

    #[serde(flatten)]
    image_source: ImageSource,

    env: Option<ListOrMap>,
    resources: Option<Resource>,
    replicas: i64,
    ports: Vec<PortConfig>,
    volume: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[fully_pub]
enum ImageSource {
    #[serde(deserialize_with = "string_or_struct")]
    Build(BuildObject),
    Image(String),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[fully_pub]
struct BuildObject {
    context: String,
    #[serde(default = "default_dockerfile")]
    dockerfile: String,
    // dockerfile_inline: String,
    #[serde(default)]
    args: Map<String, String>,
}
impl FromStr for BuildObject {
    type Err = Void;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(BuildObject {
            context: s.to_string(),
            dockerfile: default_dockerfile(),
            args: Default::default(),
        })
    }
}
fn default_dockerfile() -> String {
    "Dockerfile".to_string()
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[fully_pub]
enum ListOrMap {
    List(Vec<String>),
    Map(Map<String, String>),
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
