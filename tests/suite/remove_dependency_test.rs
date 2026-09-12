use crate::support::{lazy_java, sanitize_toml, Project};

#[test]
fn remove_dependency_cleans_config_and_lock() -> Result<(), Box<dyn std::error::Error>> {
    let p = Project::from_fixture("with-dependency")?;
    let dest = &p.dir;

    // Add spring-boot-starter-jdbc (pinned to avoid latest drift)
    lazy_java(dest)?
        .args([
            "add",
            "org.springframework.boot",
            "spring-boot-starter-jdbc",
            "4.1.0",
        ])
        .assert()
        .success();

    let toml_after_add = std::fs::read_to_string(dest.join("lazy-java.toml"))?;
    insta::assert_snapshot!("add_dependency_config", toml_after_add);

    let lock_after_add = sanitize_toml(
        &std::fs::read_to_string(dest.join("lazy-java.lock"))?,
        dest,
    );
    insta::assert_snapshot!("add_dependency_lock", lock_after_add);

    // Remove commons-lang3
    lazy_java(dest)?
        .args([
            "remove",
            "org.springframework.boot",
            "spring-boot-starter-jdbc",
        ])
        .assert()
        .success();

    let toml_after_remove = std::fs::read_to_string(dest.join("lazy-java.toml"))?;
    insta::assert_snapshot!("remove_dependency_config", toml_after_remove);

    let lock_after_remove = sanitize_toml(
        &std::fs::read_to_string(dest.join("lazy-java.lock"))?,
        dest,
    );
    insta::assert_snapshot!("remove_dependency_lock", lock_after_remove);

    Ok(())
}