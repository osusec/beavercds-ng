use anyhow::{anyhow, Error, Result};
use docker_api::opts::{PullOpts, RegistryAuth};
use docker_api::Docker;
use futures::StreamExt;
use simplelog::*;
use std::env;

use crate::configparser::{config, get_config};

/// container registry / daemon access checks
#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn check(profile: &config::ProfileConfig) -> Result<()> {
    debug!("checking docker api & registry");
    let client = client()?;

    // is docker (or podman) reachable?
    let ping = client.ping().await?;
    debug!("  docker version: {:#?}", ping.server);

    check_credentials(client).await?;

    Ok(())
}

/// build docker client
fn client() -> Result<Docker> {
    // follow default convention
    let host = env::var("DOCKER_HOST").unwrap_or("unix://var/run/docker.sock".into());

    // coerce docker_api's Error to Anyhow::Error with ?
    // instead of directly returning result
    let client = Docker::new(host)?;
    Ok(client)
}

async fn check_credentials(client: Docker) -> Result<(), Error> {
    // do we have pull access to registry?
    // try pulling nonexistant image and see what error we get

    let auth = RegistryAuth::builder()
        .username(get_config()?.registry.build.user.clone())
        .password(get_config()?.registry.build.pass.clone())
        .server_address(get_config()?.registry.domain.clone())
        .build();
    let opts = PullOpts::builder()
        .image("bogusimage")
        .tag("doesntexist")
        .auth(auth)
        .build();

    let bad_auth_message: String = "unable to retrieve auth token: invalid username/password: unauthorized: incorrect username or password".into();
    match pull_image(client, opts).await {
        // did we get an authentication error?
        Err(e) => match e {
            docker_api::Error::Fault {
                code,
                message: bad_auth_message,
            } => return Err(anyhow!("invalid registry credentials")),

            _ => {}
        },
        Ok(_) => {} // somehow the image pulled...? sure i guess
    }

    Ok(())
}

async fn pull_image(client: Docker, opts: PullOpts) -> Result<(), docker_api::Error> {
    let images = client.images();
    let mut stream = images.pull(&opts);
    let mut auth_error: Option<Error> = None;
    while let Some(pull_result) = stream.next().await {
        match pull_result {
            Ok(output) => {}
            Err(e) => return Err(e),
        }
    }

    Ok(())
}
