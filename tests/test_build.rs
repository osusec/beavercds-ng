use pretty_assertions::assert_eq;
use test_log::test;

use beavercds_ng::commands::build;

mod infra;
use infra::*;

#[test]
fn test_challenge_build() {
    figment::Jail::expect_with(|jail| {
        setup_test_repo(jail).unwrap();
        let _r = registry_ctr(jail);

        // build and push but don't extract
        let result = build::run("testing", true, false);
        println!("{result:?}");

        assert!(result.is_ok(), "challenges did not build correctly");

        Ok(())
    });
}

#[test]
fn test_artifact_extract() {
    figment::Jail::expect_with(|jail| {
        setup_test_repo(jail).unwrap();
        let _r = registry_ctr(jail);

        // build and extract but don't push
        let result = build::run("testing", false, true);
        println!("{result:?}");

        assert!(result.is_ok(), "challenges did not build correctly");

        // check extracted files are present disk and approx correct size
        for (file, expected_size) in [
            ("pwn/notsh/libc.so.6", 2030928),
            ("pwn/notsh/notsh", 8744),
            ("pwn/notsh/notsh.zip", 888175),
        ] {
            assert!(
                std::fs::metadata(file).is_ok(),
                "extracted file {file:?} should exist"
            );
            let size = std::fs::metadata(file).unwrap().len();
            assert_eq!(
                size, expected_size,
                "extracted file {file:?} should have filesize {expected_size}, but got {size}"
            );
        }

        Ok(())
    });
}
