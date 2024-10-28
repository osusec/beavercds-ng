use crate::configparser::challenge::*;
// use crate::configparser::config::*;

#[test]
fn valid_challenge_yaml() {
    let parsed = serde_yml::from_str::<ChallengeConfig>(
        r#"
            name: test_chal
            author: "me! :)"
            description: >
                A description that spans multiple lines.
                This is for testing purposes.
            difficulty: 0
            flag: dam{is-this-your-flag?}
            provide:
                - test_file1
                - test_file2
            pods: []
        "#,
    );

    assert!(parsed.is_ok());
}

#[test]
fn invalid_challenge_yaml() {
    let parsed = serde_yml::from_str::<ChallengeConfig>(
        r#"
            name: there's nothing here
            difficulty: yes
        "#,
    );

    assert!(parsed.is_err());
}
