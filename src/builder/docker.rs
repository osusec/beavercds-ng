use anyhow::{anyhow, bail, Context, Error, Result};
use bollard::auth::DockerCredentials;
use bollard::container::{
    Config, CreateContainerOptions, DownloadFromContainerOptions, RemoveContainerOptions,
};
use bollard::errors::Error as DockerError;
use bollard::image::{BuildImageOptions, PushImageOptions};
use bollard::Docker;
use core::fmt;
use futures::{StreamExt, TryStreamExt};
use simplelog::*;
use std::fs::File;
use std::io::{Seek, Write};
use std::path::PathBuf;
use std::sync::LazyLock;
use std::{fs, io};
use std::{io::Read, path::Path};
use tar;
use tempfile::Builder;
use tokio;

use crate::clients::docker;
use crate::configparser::challenge::BuildObject;
use crate::configparser::UserPass;

pub struct ContainerInfo {
    pub name: String,
    id: String,
}

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn build_image(context: &Path, options: &BuildObject, tag: &str) -> Result<String> {
    trace!("building image in directory {context:?} to tag {tag:?}");
    let client = docker().await?;

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
    let client = docker().await?;

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

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn create_container(image_tag: &str, name: &str) -> Result<ContainerInfo> {
    debug!("creating container {name:?} from image {image_tag:?}");
    let client = docker().await?;

    let opts = CreateContainerOptions {
        name: name.to_string(),
        ..Default::default()
    };
    let config = Config {
        image: Some(image_tag),
        ..Default::default()
    };

    let container = client.create_container(Some(opts), config).await?;
    Ok(ContainerInfo {
        id: container.id,
        name: name.to_string(),
    })
}

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn remove_container(container: ContainerInfo) -> Result<()> {
    debug!("removing container {}", &container.name);
    let client = docker().await?;

    let opts = RemoveContainerOptions {
        force: true,
        ..Default::default()
    };
    client.remove_container(&container.name, Some(opts)).await?;

    Ok(())
}

pub async fn copy_file(container: &ContainerInfo, from: &Path, to: &Path) -> Result<PathBuf> {
    trace!("copying {}:{from:?} to {to:?}", container.name);

    let client = docker().await?;

    let from_basename = from.file_name().unwrap();

    // Download single file from container in an archive
    let opts = DownloadFromContainerOptions {
        path: from.to_string_lossy(),
    };
    let mut dl_stream = client
        .download_from_container(&container.id, Some(opts))
        .map(|c| {
            c.with_context(|| {
                format!(
                    "could not copy file {}:{} to {}",
                    &container.name,
                    from.to_string_lossy(),
                    to.to_string_lossy()
                )
            })
        });

    // collect byte stream chunks into full file
    let mut temptar = Builder::new().suffix(".tar").tempfile_in(".")?;
    while let Some(chunk) = dl_stream.next().await {
        temptar.as_file_mut().write_all(&chunk?)?;
    }

    // unpack file retrieved to temp dir
    trace!("extracting download tar to {:?}", temptar.path());
    // need to create new File here since tar library chokes when rewinding existing `tarfile` File
    let mut tar = tar::Archive::new(File::open(temptar.path())?);

    // extract single file from archive to disk
    // we only copied out one file, so this tar should only have one file
    if let Some(mut entry_r) = tar.entries()?.next() {
        let mut entry = entry_r?;
        trace!("got entry: {:?}", entry.path());
        let mut target = File::create(to)?;
        io::copy(&mut entry, &mut target)?;
    } else {
        bail!(
            "downloaded archive for {}:{from:?} has no files in it!",
            container.name
        );
    }

    Ok(to.to_path_buf())
}
