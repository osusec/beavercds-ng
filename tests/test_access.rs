use test_log::test;

use beavercds_ng::commands::check_access;

mod infra;
use infra::{k3s_ctr, registry_ctr, s3_ctr, setup_test_repo};

#[test]
#[should_panic]
fn test_check_kube_ok() {
    todo!("k3s container does not work rootless");

    figment::Jail::expect_with(|jail| {
        setup_test_repo(jail).unwrap();
        let _kube = k3s_ctr(jail);

        let result = check_access::run("testing", true, false, false, false);
        println!("{result:?}");

        assert!(result.is_ok(), "kube access check failed");

        Ok(())
    });
}

#[test]
fn test_check_kube_missing() {
    figment::Jail::expect_with(|jail| {
        setup_test_repo(jail).unwrap();
        // no kube container

        let check = check_access::run("testing", true, false, false, false);
        assert!(check.is_err(), "kube access check should have failed");

        Ok(())
    })
}

#[test]
fn test_check_bucket_ok() {
    figment::Jail::expect_with(|jail| {
        setup_test_repo(jail).unwrap();
        let _s3 = s3_ctr(jail);

        let check = check_access::run("testing", false, false, false, true);
        assert!(check.is_ok(), "bucket access check failed");

        Ok(())
    })
}

#[test]
fn test_check_bucket_missing() {
    figment::Jail::expect_with(|jail| {
        setup_test_repo(jail).unwrap();

        // no minio container

        let check = check_access::run("testing", false, false, false, true);
        assert!(check.is_err(), "bucket access check should have failed");

        Ok(())
    })
}

#[test]
fn test_check_bucket_badcreds() {
    figment::Jail::expect_with(|jail| {
        setup_test_repo(jail).unwrap();
        let _s3 = s3_ctr(jail);

        jail.set_env("BEAVERCDS_PROFILES_TESTING_S3_ACCESS_KEY", "baduser");
        jail.set_env("BEAVERCDS_PROFILES_TESTING_S3_SECRET_KEY", "doesntexist");

        let check = check_access::run("testing", false, false, false, true);
        assert!(check.is_err(), "bucket access check should have failed");

        Ok(())
    });
}

#[test]
fn test_check_registry_ok() {
    figment::Jail::expect_with(|jail| {
        setup_test_repo(jail).unwrap();
        let _r = registry_ctr(jail);

        let check = check_access::run("testing", false, false, true, false);
        assert!(check.is_ok(), "registry access check failed");

        Ok(())
    })
}

#[test]
fn test_check_registry_missing() {
    figment::Jail::expect_with(|jail| {
        setup_test_repo(jail).unwrap();
        // no registry container

        let check = check_access::run("testing", false, false, true, false);
        assert!(check.is_err(), "registry access check should have failed");

        Ok(())
    })
}

#[test]
#[should_panic] // TODO: no way to test credentials with local registry
fn test_check_registry_badcreds() {
    figment::Jail::expect_with(|jail| {
        setup_test_repo(jail).unwrap();
        let _r = registry_ctr(jail);

        jail.set_env("BEAVERCDS_REGISTRY_BUILD_USER", "baduser");
        jail.set_env("BEAVERCDS_REGISTRY_BUILD_PASS", "doesntexist");

        let check = check_access::run("testing", false, false, true, false);
        assert!(check.is_err(), "registry access check should have failed");

        Ok(())
    });
}
