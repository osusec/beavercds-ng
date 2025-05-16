// Builders for the various client structs for Docker/Kube etc.

use std::{collections::HashMap, sync::OnceLock};

use anyhow::{anyhow, bail, Context, Result};
use bollard;
use k8s_openapi::api::core::v1::Service;
use kube::{
    self,
    api::{DynamicObject, GroupVersionKind, Patch, PatchParams},
    core::ResourceExt,
    discovery::{ApiCapabilities, ApiResource},
    runtime::{conditions, wait::await_condition},
};
use s3;
use tracing::{debug, error, info, trace, warn};

use crate::configparser::config;

//
// Docker stuff
//

static DOCKER_CLIENT: OnceLock<bollard::Docker> = OnceLock::new();

/// Return existing or create new Docker client
pub async fn docker() -> Result<&'static bollard::Docker> {
    match DOCKER_CLIENT.get() {
        Some(d) => Ok(d),
        None => {
            debug!("connecting to docker...");
            let client = bollard::Docker::connect_with_defaults()?;
            client
                .ping()
                .await
                // truncate error chain with new error (returned error is way too verbose)
                .map_err(|_| {
                    anyhow!("could not talk to Docker daemon (is DOCKER_HOST correct?)")
                })?;

            Ok(DOCKER_CLIENT.get_or_init(|| client))
        }
    }
}

/// Fetch registry login credentials from ~/.docker/config.json or $DOCKER_CONFIG
///
/// For now, this is only `docker.io` credentials, as it is the only registry
/// that effectively requires auth for public images. We don't intend for
/// challenge images to be built from private images.
///
/// If lookup fails, return empty hashmap as anonymous user.
pub fn docker_creds() -> Result<HashMap<String, bollard::auth::DockerCredentials>> {
    let cred_r = docker_credential::get_credential("docker.io");

    let cred = match cred_r {
        Ok(cred) => cred,
        Err(e) => {
            // dont die if the credentials could not be found. Warn and continue as anonymous
            warn!("could not fetch docker.io registry credentials from Docker config (are you logged in?)");
            // log full error for debug
            trace!("credentials error: {e:?}");

            warn!("continuing as with anonymous build credentials");
            return Ok(HashMap::new());
        }
    };

    // convert docker_credential enum to bollad
    let converted = match cred {
        docker_credential::DockerCredential::IdentityToken(token) => {
            bollard::auth::DockerCredentials {
                identitytoken: Some(token),
                ..Default::default()
            }
        }
        docker_credential::DockerCredential::UsernamePassword(u, p) => {
            bollard::auth::DockerCredentials {
                username: Some(u),
                password: Some(p),
                ..Default::default()
            }
        }
    };

    Ok(std::collections::HashMap::from([(
        "docker.io".to_string(),
        converted,
    )]))
}

// /// wip to pull all docker creds from json
// pub async fn all_docker_creds() -> Result<HashMap<String, bollard::auth::DockerCredentials>> {
//     let auth_path = dirs::home_dir()
//         .expect("could not fetch homedir")
//         .join(".docker")
//         .join("config.json");
//     let auth_file = File::open(auth_path).context("could not read docker auth config.json")?;
//     // json is technically yaml so use the dependency we already bring in
//     let auth_json: serde_yml::Value = serde_yml::from_reader(auth_file).unwrap();

//     let mut map = HashMap::new();
//     for (raw_reg, _raw_auth) in auth_json.get("auths").unwrap().as_mapping().unwrap() {
//         let reg = raw_reg.as_str().unwrap();
//         let cred = match engine_type().await {
//             EngineType::Docker => docker_credential::get_credential(reg),
//             EngineType::Podman => docker_credential::get_podman_credential(reg),
//         }
//         .context("could not fetch Docker registry credentials from Docker config")?;

//         let creds = match cred {
//             docker_credential::DockerCredential::IdentityToken(token) => {
//                 bollard::auth::DockerCredentials {
//                     identitytoken: Some(token),
//                     ..Default::default()
//                 }
//             }
//             docker_credential::DockerCredential::UsernamePassword(u, p) => {
//                 bollard::auth::DockerCredentials {
//                     username: Some(u),
//                     password: Some(p),
//                     ..Default::default()
//                 }
//             }
//         };

//         map.insert(reg.to_string(), creds);
//     }

//     Ok(map)
// }

#[derive(Debug)]
pub enum EngineType {
    Docker,
    Podman,
}
pub async fn engine_type() -> EngineType {
    let c = docker().await.unwrap();
    let version = c.version().await.unwrap();

    if version
        .components
        .unwrap()
        .iter()
        .any(|c| c.name == "Podman Engine")
    {
        EngineType::Podman
    } else {
        EngineType::Docker
    }
}

//
// S3 stuff
//

// this does need to be a OnceLock instead of a LazyLock, even though how this
// is used is more inline with a LazyLock. Lazy does not allow for passing
// anything into the init function, and this needs a parameter to know what
// profile to fetch creds for.
static BUCKET_CLIENT: OnceLock<Box<s3::Bucket>> = OnceLock::new();

/// return existing or create new bucket client for passed profile config
pub fn bucket_client(config: &config::S3Config) -> Result<&s3::Bucket> {
    match BUCKET_CLIENT.get() {
        Some(b) => Ok(b),
        None => {
            trace!("creating bucket client");
            let region = s3::Region::Custom {
                region: config.region.clone(),
                endpoint: config.endpoint.clone(),
            };
            let creds = s3::creds::Credentials::new(
                Some(&config.access_key),
                Some(&config.secret_key),
                None,
                None,
                None,
            )?;
            let bucket = s3::Bucket::new(&config.bucket_name, region, creds)?.with_path_style();

            Ok(BUCKET_CLIENT.get_or_init(|| bucket))
        }
    }
}

/// create public/anonymous bucket client for passed profile config
// this does not need a oncelock and can be created on-demand, as this is not used in very many places
pub fn bucket_client_anonymous(config: &config::S3Config) -> Result<Box<s3::Bucket>> {
    trace!("creating anon bucket client");
    // TODO: once_cell this so it reuses the same bucket?
    let region = s3::Region::Custom {
        region: config.region.clone(),
        endpoint: config.endpoint.clone(),
    };
    let creds = s3::creds::Credentials::anonymous()?;
    let bucket = s3::Bucket::new(&config.bucket_name, region, creds)?.with_path_style();

    Ok(bucket)
}

//
// Kubernetes stuff
//

// no OnceLock caching for K8S client. Some operations with the client require
// their own owned `kube::Client`, so always returning a borrowed client from
// the OnceLock would not work.

/// Returns Kubernetes Client for selected profile
pub async fn kube_client(profile: &config::ProfileConfig) -> Result<kube::Client> {
    debug!("building kube client");

    // read in kubeconfig from given kubeconfig (or default)
    // (use kube::Config to specify context)
    let options = kube::config::KubeConfigOptions {
        context: Some(profile.kubecontext.to_owned()),
        cluster: None,
        user: None,
    };

    let client_config = match &profile.kubeconfig {
        Some(kc_path) => {
            let kc = kube::config::Kubeconfig::read_from(kc_path)?;
            kube::Config::from_custom_kubeconfig(kc, &options).await
        }
        None => kube::Config::from_kubeconfig(&options).await,
    }?;

    let client = kube::Client::try_from(client_config)?;

    // check kube api readiness endpoint to make sure its reachable
    let ready_req = http::Request::get("/readyz").body(vec![]).unwrap();
    let ready_resp = client
        .request_text(ready_req)
        .await
        // change 'Connection refused' error into something more helpful
        .map_err(|e| {
            anyhow!("could not connect to Kubernetes (is KUBECONFIG or KUBECONTEXT correct?)")
        })?;

    if ready_resp != "ok" {
        bail!("Kubernetes is not ready")
    };

    Ok(client)
}

pub async fn kube_resource_for(
    kube_object: &DynamicObject,
    client: &kube::Client,
) -> Result<(ApiResource, ApiCapabilities)> {
    let gvk = if let Some(tm) = &kube_object.types {
        GroupVersionKind::try_from(tm)?
    } else {
        bail!(
            "cannot apply object without valid TypeMeta {:?}",
            kube_object
        );
    };

    let name = kube_object.name_any();

    kube::discovery::pinned_kind(client, &gvk)
        .await
        .with_context(|| {
            format!(
                "could not find resource type {:?} on cluster",
                kube_object.types.clone().unwrap_or_default()
            )
        })
}

/// Create a Kube API client for the passed object's resource type
pub async fn kube_api_for(
    kube_object: &DynamicObject,
    client: kube::Client,
) -> Result<kube::Api<DynamicObject>> {
    let ns = kube_object.metadata.namespace.as_deref();

    let (resource, caps) = kube_resource_for(kube_object, &client).await?;

    if caps.scope == kube::discovery::Scope::Cluster {
        Ok(kube::Api::all_with(client, &resource))
    } else if let Some(namespace) = ns {
        Ok(kube::Api::namespaced_with(client, namespace, &resource))
    } else {
        Ok(kube::Api::default_namespaced_with(client, &resource))
    }
}

/// Apply multi-document manifest file, return created resources
pub async fn apply_manifest_yaml(
    client: &kube::Client,
    manifest: &str,
) -> Result<Vec<DynamicObject>> {
    // set ourself as the owner for managed fields
    // https://kubernetes.io/docs/reference/using-api/server-side-apply/#managers
    let pp = PatchParams::apply("beavercds").force();

    let mut results = vec![];

    // this manifest has multiple documents (crds, deployment)
    for yaml in multidoc_deserialize(manifest)? {
        let obj: DynamicObject = serde_yml::from_value(yaml)?;
        trace!(
            "applying resource {} {}",
            obj.types.clone().unwrap_or_default().kind,
            obj.name_any()
        );

        let obj_api = kube_api_for(&obj, client.clone()).await?;
        match obj_api
            // patch is idempotent and will create if not present
            .patch(&obj.name_any(), &pp, &Patch::Apply(&obj))
            .await
        {
            Ok(d) => {
                results.push(d);
                Ok(())
            }
            // if error is from cluster api, mark it as such
            Err(kube::Error::Api(ae)) => {
                // Err(kube::Error::Api(ae).into())
                Err(anyhow!(ae).context("error from cluster when deploying"))
            }
            // other errors could be anything
            Err(e) => Err(anyhow!(e)).context("unknown error when deploying"),
        }?;
    }

    Ok(results)
}

/// Deserialize multi-document yaml string into a Vec of the documents
fn multidoc_deserialize(data: &str) -> Result<Vec<serde_yml::Value>> {
    use serde::Deserialize;

    let mut docs = vec![];
    for de in serde_yml::Deserializer::from_str(data) {
        match serde_yml::Value::deserialize(de)? {
            // discard any empty documents (e.g. from trailing ---)
            serde_yml::Value::Null => (),
            not_null => docs.push(not_null),
        };
    }
    Ok(docs)

    // // deserialize all chunks
    // serde_yml::Deserializer::from_str(data)
    //     .map(serde_yml::Value::deserialize)
    //     // discard any empty documents (e.g. from trailing ---)
    //     .filter_ok(|val| val != &serde_yml::Value::Null)
    //     // coerce errors to Anyhow
    //     .map(|r| r.map_err(|e| e.into()))
    //     .collect()
}

/// Check the status of the passed object and wait for it to become ready.
///
/// This function does not provide a timeout. Callers will need to wrap this with a timeout instead.
pub async fn wait_for_status(client: &kube::Client, object: &DynamicObject) -> Result<()> {
    debug!(
        "waiting for ok status for {} {}",
        object.types.clone().unwrap_or_default().kind,
        object.name_any()
    );

    // handle each separate object type differently
    match object.types.clone().unwrap_or_default().kind.as_str() {
        // wait for Pod to become running
        "Pod" => {
            let api = kube::Api::namespaced(client.clone(), &object.namespace().unwrap());
            let x = await_condition(api, &object.name_any(), conditions::is_pod_running()).await?;
        }

        // wait for Deployment to complete rollout
        "Deployment" => {
            let api = kube::Api::namespaced(client.clone(), &object.namespace().unwrap());
            await_condition(
                api,
                &object.name_any(),
                conditions::is_deployment_completed(),
            )
            .await?;
        }

        // wait for Ingress to get IP from ingress controller
        "Ingress" => {
            let api = kube::Api::namespaced(client.clone(), &object.namespace().unwrap());
            await_condition(
                api,
                &object.name_any(),
                conditions::is_ingress_provisioned(),
            )
            .await?;
        }

        // wait for LoadBalancer service to get IP
        "Service" => {
            let api = kube::Api::namespaced(client.clone(), &object.namespace().unwrap());
            let svc: Service = api.get(&object.name_any()).await?;

            // we only care about checking LoadBalancer-type services, return Ok
            // for any non-LB services
            //
            // TODO: do we care about NodePorts? don't need to check any atm
            if svc.spec.unwrap_or_default().type_ != Some("LoadBalancer".to_string()) {
                trace!(
                    "not checking status for internal service {}",
                    object.name_any()
                );
                return Ok(());
            }

            await_condition(
                api,
                &object.name_any(),
                conditions::is_service_loadbalancer_provisioned(),
            )
            .await?;
        }

        other => trace!("not checking status for resource type {other}"),
    };

    Ok(())
}
