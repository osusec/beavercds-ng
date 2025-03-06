use pretty_assertions::assert_eq;
use serial_test::serial;
use test_log::test;

use beavercds_ng::commands::build;

mod infra;
use infra::*;

#[test]
#[serial]
fn test_challenge_build() {
    let _registry = registry_ctr();
    cd_to_repo();

    // build and push but don't extract
    let result = build::run("testing", true, false);

    assert!(result.is_ok())

    // TODO: check for images on registry (shell out to docker? bring in bollard?)
}

#[test]
#[serial]
fn test_artifact_extract() {
    cd_to_repo();

    // build and extract but don't push
    let result = build::run("testing", false, true);

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
