use std::env;

use pretty_assertions::assert_eq;
use test_log::test;
use testcontainers_modules::{
    cncf_distribution::CncfDistribution,
    minio::MinIO,
    testcontainers::{core::ExecCommand, runners::SyncRunner, Container, ImageExt},
};

fn registry_ctr() -> Container<CncfDistribution> {
    let registry = CncfDistribution.start().unwrap();

    env::set_var(
        "BEAVERCDS_REGISTRY_DOMAIN",
        format!(
            "{}:{}/testimages",
            registry.get_host().unwrap(),
            registry.get_host_port_ipv4(5000).unwrap()
        ),
    );

    registry
}

fn s3_ctr() -> Container<MinIO> {
    let minio = MinIO::default()
        .with_env_var("MINIO_ROOT_USER", "testuser")
        .with_env_var("MINIO_ROOT_PASSWORD", "notsecure")
        .start()
        .unwrap();

    minio
        .exec(
            // create bucket and permissions
            ExecCommand::new([
                "/bin/sh",
                "-xec",
                "
                    mc alias set self http://localhost:9000 testuser notsecure;
                    mc mb self/testbucket;
                    mc anonymous set download self/testbucket;
                ",
            ]),
        )
        .unwrap();

    // set envvars to point at this container
    env::set_var(
        "BEAVERCDS_PROFILES_TESTING_S3_ENDPOINT",
        format!(
            "http://{}:{}",
            minio.get_host().unwrap(),
            minio.get_host_port_ipv4(9000).unwrap()
        ),
    );
    env::set_var("BEAVERCDS_PROFILES_TESTING_S3_REGION", "");
    env::set_var("BEAVERCDS_PROFILES_TESTING_S3_ACCESS_KEY", "minioadmin");
    env::set_var("BEAVERCDS_PROFILES_TESTING_S3_SECRET_KEY", "minioadmin");

    minio
}

#[test]
fn test_challenge_build() {
    let _registry = registry_ctr();
    env::set_current_dir("./tests/repo").unwrap();

    // build and push but don't extract
    let result = beavercds_ng::commands::build::run("testing", &true, &false);

    assert!(result.is_ok())

    // TODO: check for images on registry (shell out to docker? bring in bollard?)
}

#[test]
fn test_artifact_extract() {
    let _registry = registry_ctr();
    let _s3 = s3_ctr();
    env::set_current_dir("./tests/repo").unwrap();

    // build and extract but don't push
    let result = beavercds_ng::commands::build::run("testing", &false, &true);

    assert!(result.is_ok());

    // check extracted files are present disk and approx correct size
    assert!(std::fs::metadata("pwn/notsh/libc.so.6").is_ok());
    assert_eq!(
        std::fs::metadata("pwn/notsh/libc.so.6").unwrap().len(),
        2030928
    );

    assert!(std::fs::metadata("pwn/notsh/notsh").is_ok());
    assert_eq!(std::fs::metadata("pwn/notsh/notsh").unwrap().len(), 8744);

    assert!(std::fs::metadata("pwn/notsh/notsh.zip").is_ok());
    assert_eq!(
        std::fs::metadata("pwn/notsh/notsh.zip").unwrap().len(),
        888175
    );
}
