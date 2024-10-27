use anyhow::{anyhow, bail, Context, Error, Result};
use bollard::auth::DockerCredentials;
use bollard::errors::Error as DockerError;
use bollard::image::{BuildImageOptions, PushImageOptions};
use bollard::Docker;
use core::fmt;
use futures_util::{StreamExt, TryStreamExt};
use simplelog::*;
use std::{io::Read, path::Path};
use tar;
use tempfile::tempfile;
use tokio;

use crate::configparser::challenge::BuildObject;
use crate::configparser::UserPass;

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn build_image(context: &Path, options: &BuildObject, tag: &str) -> Result<String> {
    trace!("building image in directory {context:?} to tag {tag:?}");
    let client = client()
        .await
        // truncate error chain with new error (returned error is way too verbose)
        .map_err(|_| anyhow!("could not talk to Docker daemon (is DOCKER_HOST correct?)"))?;

    let build_opts = BuildImageOptions {
        dockerfile: options.dockerfile.clone(),
        buildargs: options.args.clone(),
        t: tag.to_string(),
        forcerm: true,
        ..Default::default()
    };

    // tar up image context
    // TODO: dont store the tarball in memory...
    // let mut tar = tar::Builder::new(tempfile()?);
    let mut tar = tar::Builder::new(Vec::new());
    tar.append_dir_all("", context.join(&options.context))
        .with_context(|| "could not create image context tarball")?;
    let tarball = tar.into_inner()?;

    // send to docker daemon
    let mut build_stream = client.build_image(build_opts, None, Some(tarball.into()));

    // stream output to stdout
    while let Some(item) = build_stream.next().await {
        match item {
            // error from stream?
            Err(e) => match e {
                DockerError::DockerStreamError { error } => bail!("build error: {error}"),
                other => bail!("build error: {other:?}"),
            },
            Ok(msg) => {
                // error from daemon?
                if let Some(e) = msg.error_detail {
                    bail!(
                        "error building image: {}",
                        e.message.unwrap_or("".to_string())
                    )
                }

                if let Some(log) = msg.stream {
                    info!(
                        "building {}: <bright-black>{}</>",
                        context.to_string_lossy(),
                        // tag,
                        log.trim()
                    )
                }
            }
        }
    }

    Ok(tag.to_string())
}

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn push_image(image_tag: &str, creds: &UserPass) -> Result<String> {
    info!("pushing image {image_tag:?} to registry");
    let client = client()
        .await
        // truncate error chain with new error (returned error is way too verbose)
        .map_err(|_| anyhow!("could not talk to Docker daemon (is DOCKER_HOST correct?)"))?;

    let (image, tag) = image_tag
        .rsplit_once(":")
        .context("failed to get tag from full image string")?;

    let opts = PushImageOptions { tag };
    let creds = DockerCredentials {
        username: Some(creds.user.clone()),
        password: Some(creds.pass.clone()),
        ..Default::default()
    };

    let mut push_stream = client.push_image(image, Some(opts), Some(creds));

    // stream output to stdout
    while let Some(item) = push_stream.next().await {
        match item {
            // error from stream?
            Err(DockerError::DockerResponseServerError {
                status_code,
                message,
            }) => bail!("error from daemon: {message}"),
            Err(e) => bail!("{e:?}"),
            Ok(msg) => {
                debug!("{msg:?}");
                if let Some(progress) = msg.progress_detail {
                    info!("progress: {:?}/{:?}", progress.current, progress.total);
                }
            }
        }
    }
    Ok(tag.to_string())
}

//
// helper functions
//
pub async fn client() -> Result<Docker> {
    debug!("connecting to docker...");
    let client = Docker::connect_with_defaults()?;
    client.ping().await?;

    Ok(client)
}

#[derive(Debug)]
pub enum EngineType {
    Docker,
    Podman,
}
pub async fn engine_type() -> EngineType {
    let c = client().await.unwrap();
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
