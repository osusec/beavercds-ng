use anyhow::{anyhow, Context, Error, Result};
use bollard::{
    auth::DockerCredentials,
    image::{CreateImageOptions, PushImageOptions, TagImageOptions},
    Docker,
};
use futures_util::{StreamExt, TryStreamExt};
use itertools::Itertools;
use simplelog::*;
use tokio;

use crate::configparser::{config, get_config};

/// container registry / daemon access checks
#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn check(profile: &config::ProfileConfig) -> Result<()> {
    // docker / podman does not keep track of whether registry credentials are
    // valid or not. to check if we do have valid creds, we need to do something
    // to present creds, like pulling an image.

    let client = client()
        .await
        // truncate error chain with new error (returned error is way too verbose)
        .map_err(|_| anyhow!("could not talk to Docker daemon (is DOCKER_HOST correct?)"))?;

    // try pulling a non-existent image and see what error we get back
    check_credentials(client)
        .await
        .with_context(|| "Could not access registry (bad credentials?)")?;

    info!("  registry ok!");
    Ok(())
}

async fn client() -> Result<Docker> {
    debug!("connecting to docker...");
    let client = Docker::connect_with_defaults()?;
    client.ping().await?;

    Ok(client)
}

async fn check_credentials(client: Docker) -> Result<(), Error> {
    // do we have push access to registry?
    // try pushing test image and see
    debug!("checking registry credentials");

    // pull Alpine as test image
    debug!("pulling alpine test image from docker.io");
    let alpine = CreateImageOptions {
        from_image: "alpine",
        tag: "latest",
        ..Default::default()
    };
    let docker_public = DockerCredentials {
        serveraddress: Some("docker.io".to_string()),
        ..Default::default()
    };
    client
        .create_image(Some(alpine), None, Some(docker_public))
        .try_collect::<Vec<_>>()
        .await?;

    let registry_config = &get_config()?.registry;

    // rename alpine image
    let tag_opts = TagImageOptions {
        repo: registry_config.domain.clone(),
        tag: "ignore".to_string(),
    };
    client.tag_image("alpine", Some(tag_opts));

    // alpine image has been pulled, now push it to configured repo
    debug!("pushing alpine to target registry");
    let options = PushImageOptions { tag: "latest" };
    let creds = DockerCredentials {
        username: Some(registry_config.build.user.clone()),
        password: Some(registry_config.build.pass.clone()),
        serveraddress: Some(registry_config.domain.clone()),
        ..Default::default()
    };

    client.push_image("alpine", Some(options), Some(creds));

    Ok(())
}
