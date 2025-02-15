use minijinja::{context, escape_formatter, Environment, Value};

// Embed Kubernetes template files into binary.

pub static CHALLENGE_NAMESPACE: &str =
    include_str!("../../asset_files/challenge_templates/namespace.yaml.j2");

pub static CHALLENGE_DEPLOYMENT: &str =
    include_str!("../../asset_files/challenge_templates/deployment.yaml.j2");

pub static CHALLENGE_SERVICE_HTTP: &str =
    include_str!("../../asset_files/challenge_templates/http.yaml.j2");

pub static CHALLENGE_SERVICE_TCP: &str =
    include_str!("../../asset_files/challenge_templates/tcp.yaml.j2");

/// Build template environment with None as default
/// https://github.com/mitsuhiko/minijinja/tree/main/examples/none-as-undefined
pub fn template_env() -> Environment<'static> {
    let mut env = Environment::new();

    env.add_filter("default", none_default);
    env.set_formatter(|out, state, value| {
        escape_formatter(
            out,
            state,
            if value.is_none() {
                &Value::UNDEFINED
            } else {
                value
            },
        )
    });

    env
}

/// Similar to the regular `default` filter but also handles `none`.
pub fn none_default(value: Value, other: Option<Value>) -> Value {
    if value.is_undefined() || value.is_none() {
        other.unwrap_or_else(|| Value::from(""))
    } else {
        value
    }
}
