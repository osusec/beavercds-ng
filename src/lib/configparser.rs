use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::BTreeMap;
use std::{fs, io};

// Structs for rcds.yaml parsing
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RCDSConfig {
    flag_regex: String,
    registry: Registry,
    defaults: Defaults,
    profiles: BTreeMap<String, ProfileConfig>,
    points: Vec<ChallengePoints>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum Registry {
    TopLevel(RegistryOne),
    Nested(RegistryTwo),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct RegistryOne {
    domain: String,
    build: UserPass,
    cluster: UserPass,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct RegistryTwo {
    domain: String,
    user: String,
    pass: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct UserPass {
    user: String,
    pass: String,
}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Resource {
    cpu: i64, 
    memory: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Defaults {
    difficulty: i64,
    resources: Resource,
}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct ProfileConfig {
	// deployed_challenges: BTreeMap<String, bool>,
	frontend_url: String,
	frontend_token: Option<String>,
	challenges_domain: String,
	kubeconfig: String,
	kubecontext: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct ChallengePoints {
    difficulty: i64,
    min: i64,
    max: i64,
}

#[derive(Debug)]
pub enum ParseError {
    YAMLParseError(serde_yaml::Error),
    IOError(std::io::Error),
}

impl From<io::Error> for ParseError {
    fn from(err: io::Error) -> Self {
        ParseError::IOError(err)
    }
}

impl From<serde_yaml::Error> for ParseError {
    fn from(err: serde_yaml::Error) -> Self {
        ParseError::YAMLParseError(err)
    }
}

pub fn parse_rcds_config() -> Result<RCDSConfig,ParseError> {
    let my_stuff = fs::read_to_string("rcds.yaml")?;

    let application_data: RCDSConfig = serde_yaml::from_str(&my_stuff)?;
    println!("{:?}", application_data);
    return Ok(application_data);
}


// Structs for challenge.yaml parsing

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ChallengeConfig {
    name: String,
    author: String,
    description: String,
    difficulty: i64,
    flag: FlagType,
    provide: Vec<String>,
    pods: Vec<Pod>
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
    CONTEXT(String),
    MAP(BTreeMap<String, String>),
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
    LIST(Vec<String>),
    MAP(BTreeMap<String, String>),
}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct PortConfig {
    internal: i64,
    expose: PortType
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum PortType {
    TCP(TCPPort),
    HTTP(HTTPEndpoint),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct TCPPort {
    tcp: i64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct HTTPEndpoint {
    http: String,
}




pub fn parse_challenge_config(path: &str) -> Result<ChallengeConfig,ParseError> {
    let my_stuff = fs::read_to_string(path)?;

    let application_data: ChallengeConfig = serde_yaml::from_str(&my_stuff)?;
    println!("{:?}", application_data);
    return Ok(application_data);
}