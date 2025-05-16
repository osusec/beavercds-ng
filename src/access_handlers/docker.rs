use anyhow::{anyhow, Context, Error, Result};
use bollard::{
    auth::DockerCredentials,
    image::{CreateImageOptions, PushImageOptions, TagImageOptions},
    Docker,
};
use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;
use minijinja;
use tokio;
use tracing::{debug, error, info, trace, warn};

use crate::configparser::{get_config, get_profile_config};
use crate::{clients::docker, utils::render_strict};

/// container registry / daemon access checks
#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn check(profile_name: &str) -> Result<()> {
    // docker / podman does not keep track of whether registry credentials are
    // valid or not. to check if we do have valid creds, we need to do something
    // to present creds, like pulling an image.

    let profile = get_profile_config(profile_name)?;
    let registry_config = &get_config()?.registry;

    let client = docker().await?;

    // build test image string
    // registry.example.com/somerepo/testimage:pleaseignore
    let test_image = render_strict(
        &registry_config.tag_format,
        minijinja::context! {
        domain => registry_config.domain,
        challenge => "accesscheck",
        container => "testimage",
        profile => profile_name
        },
    )
    .context("could not render tag format template")?;
    debug!("will push test image to {}", test_image);

    // push alpine image with build credentials
    check_build_credentials(client, &test_image)
        .await
        .with_context(|| "Could not push images to registry (bad build credentials?)")?;

    // try pulling that image with cluster credentials
    check_cluster_credentials(client, &test_image)
        .await
        .with_context(|| "Could not pull images from registry (bad cluster credentials?)")?;

    Ok(())
}

/// test build-time registry push credentials by pushing test image
async fn check_build_credentials(client: &Docker, test_image: &str) -> Result<(), Error> {
    // do we have push access to registry?
    // try pushing test image and see
    debug!("checking registry build push credentials");

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

    // rename alpine image as test imag
    let (repo, tag) = test_image
        .rsplit_once(':')
        .unwrap_or((test_image, "latest"));
    let tag_opts = TagImageOptions { repo, tag };
    client.tag_image("alpine", Some(tag_opts)).await?;

    // now push test iamge to configured repo
    debug!("pushing alpine to target registry as {}:{}", repo, tag);
    let options = PushImageOptions { tag };
    let build_creds = DockerCredentials {
        username: Some(registry_config.build.user.clone()),
        password: Some(registry_config.build.pass.clone()),
        serveraddress: Some(registry_config.domain.clone()),
        ..Default::default()
    };

    client
        .push_image(repo, Some(options), Some(build_creds))
        .try_collect::<Vec<_>>()
        .await?;

    Ok(())
}

/// test in-cluster registry credentials with test image
async fn check_cluster_credentials(client: &Docker, test_image: &str) -> Result<(), Error> {
    // do we have pull access from registry?
    // try pulling test image and see
    debug!("checking registry cluster pull credentials");

    let registry_config = &get_config()?.registry;

    // pull just-pushed alpine image from repo
    let (repo, tag) = test_image
        .rsplit_once(':')
        .unwrap_or((test_image, "latest"));
    let alpine_test_image = CreateImageOptions {
        from_image: [repo, tag].join(":"),
        ..Default::default()
    };
    let cluster_creds = DockerCredentials {
        username: Some(registry_config.cluster.user.clone()),
        password: Some(registry_config.cluster.pass.clone()),
        serveraddress: Some(registry_config.domain.clone()),
        ..Default::default()
    };

    client
        .create_image(Some(alpine_test_image), None, Some(cluster_creds))
        .try_collect::<Vec<_>>()
        .await?;

    Ok(())
}
