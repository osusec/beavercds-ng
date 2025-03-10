use test_log::test;

use beavercds_ng::commands::check_access;

mod infra;
use infra::{k3s_ctr, registry_ctr, s3_ctr, setup_test_repo};

#[test]
fn test_check_kube_ok() {
    todo!("k3s container does not work rootless");

    figment::Jail::expect_with(|jail| {
        setup_test_repo(jail).unwrap();
        let _kube = k3s_ctr(jail);

        let result = check_access::run("testing", true, false, false, false);
        println!("{result:?}");

        assert!(result.is_ok());

        Ok(())
    });
}

#[test]
fn test_check_kube_missing() {
    figment::Jail::expect_with(|jail| {
        setup_test_repo(jail).unwrap();
        // no kube container

        let check = check_access::run("testing", true, false, false, false);
        assert!(check.is_err());

        Ok(())
    })
}

#[test]
fn test_check_bucket_ok() {
    figment::Jail::expect_with(|jail| {
        setup_test_repo(jail).unwrap();
        let _s3 = s3_ctr(jail);

        std::thread::sleep(std::time::Duration::from_secs(5));

        let check = check_access::run("testing", false, false, false, true);
        assert!(check.is_ok());

        Ok(())
    })
}

#[test]
fn test_check_bucket_missing() {
    figment::Jail::expect_with(|jail| {
        setup_test_repo(jail).unwrap();

        // no minio container

        let check = check_access::run("testing", false, false, false, true);
        assert!(check.is_err());

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
        assert!(check.is_err());

        Ok(())
    });
}
