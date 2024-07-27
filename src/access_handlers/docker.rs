use anyhow::{anyhow, Context, Error, Result};
use bollard::{auth::DockerCredentials, image::CreateImageOptions, Docker};
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

    Ok(())
}

async fn client() -> Result<Docker> {
    debug!("connecting to docker...");
    let client = Docker::connect_with_defaults()?;
    client.ping().await?;

    Ok(client)
}

async fn check_credentials(client: Docker) -> Result<(), Error> {
    // do we have pull access to registry?
    // try pulling nonexistant image and see what error we get
    debug!("checking registry credentials");

    let options = CreateImageOptions {
        from_image: "bogusimage",
        tag: "doesntexist",
        ..Default::default()
    };
    let auth = DockerCredentials {
        username: Some(get_config()?.registry.build.user.clone()),
        password: Some(get_config()?.registry.build.pass.clone()),
        serveraddress: Some(get_config()?.registry.domain.clone()),
        ..Default::default()
    };

    let result = client
        .create_image(Some(options.clone()), None, None)
        .try_collect::<Vec<_>>()
        .await;

    debug!("result: {:?}", result);

    let err = match result {
        Ok(info) => {
            // somehow the image pulled...?
            warn!(
                "successfully pulled '{}:{}'! how did that happen?",
                options.from_image, options.tag
            );
            // that's fine, I suppose; return Ok
            return Ok(());
        }
        Err(e) => e,
    };

    let expected_error_message: String = "image not found".into();
    match err {
        // if image does not exist, that is expected (and credentials worked)
        bollard::errors::Error::DockerResponseServerError {
            message: expected_error_message,
            status_code: 403,
        } => Ok(()),
        // any other error means something else is wrong (with credentials or otherwise), pass it up
        others => Err(others.into()),
    }
}
