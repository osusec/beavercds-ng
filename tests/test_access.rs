use serial_test::serial;
use test_log::test;

use beavercds_ng::commands::check_access;

mod infra;
use infra::{cd_to_repo, k3s_ctr, s3_ctr};

#[test]
#[serial]
fn test_check_kube_ok() {
    let _kube = k3s_ctr();
    cd_to_repo();

    let check = check_access::run("testing", true, false, false, false);
    assert!(check.is_ok())
}

#[test]
#[serial]
fn test_check_kube_missing() {
    // no kube container
    cd_to_repo();

    let check = check_access::run("testing", true, false, false, false);
    assert!(check.is_err())
}

#[test]
#[serial]
fn test_check_bucket_ok() {
    let _s3 = s3_ctr();
    cd_to_repo();

    let check = check_access::run("testing", false, false, false, true);
    assert!(check.is_ok())
}

#[test]
#[serial]
fn test_check_bucket_missing() {
    // no minio container
    cd_to_repo();

    let check = check_access::run("testing", false, false, false, true);
    assert!(check.is_err())
}

#[test]
#[serial]
fn test_check_bucket_badcreds() {
    let _s3 = s3_ctr();
    cd_to_repo();

    std::env::set_var("BEAVERCDS_PROFILES_TESTING_S3_ACCESS_KEY", "baduser");
    std::env::set_var("BEAVERCDS_PROFILES_TESTING_S3_SECRET_KEY", "doesntexist");

    let check = check_access::run("testing", false, false, false, true);
    assert!(check.is_err())
}
