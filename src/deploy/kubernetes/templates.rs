// Embed Kubernetes template files into binary.

pub static CHALLENGE_NAMESPACE: &str =
    include_str!("../../asset_files/challenge_templates/namespace.yaml.j2");

pub static CHALLENGE_DEPLOYMENT: &str =
    include_str!("../../asset_files/challenge_templates/deployment.yaml.j2");

pub static CHALLENGE_SERVICE_HTTP: &str =
    include_str!("../../asset_files/challenge_templates/http.yaml.j2");

pub static CHALLENGE_SERVICE_TCP: &str =
    include_str!("../../asset_files/challenge_templates/tcp.yaml.j2");

pub static IMAGE_PULL_CREDS_SECRET: &str =
    include_str!("../../asset_files/challenge_templates/pull-secret.yaml.j2");
