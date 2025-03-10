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

        assert!(result.is_ok());

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

        Ok(())
    });
}
