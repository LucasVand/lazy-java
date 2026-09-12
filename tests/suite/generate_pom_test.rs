use crate::support::{lazy_java, Project};

#[test]
fn generate_pom_includes_deps_and_annotation_processors() -> Result<(), Box<dyn std::error::Error>>
{
    let p = Project::from_fixture("generate-pom")?;
    let dest = &p.dir;

    lazy_java(dest)?
        .args(["add", "org.apache.commons", "commons-lang3", "3.20.0"])
        .assert()
        .success();

    lazy_java(dest)?
        .args([
            "add",
            "com.google.auto.value",
            "auto-value-annotations",
            "1.11.1",
        ])
        .assert()
        .success();

    lazy_java(dest)?
        .args(["add", "com.google.auto.value", "auto-value", "1.11.1"])
        .assert()
        .success();

    assert!(
        dest.join("lazy-java.lock").exists(),
        "lock file should exist after add"
    );

    lazy_java(dest)?.args(["generate", "pom"]).assert().success();

    let pom_path = dest.join("pom.xml");
    assert!(pom_path.exists(), "pom.xml should exist");

    let pom_content = std::fs::read_to_string(pom_path)?;

    insta::assert_snapshot!("generate_pom_output", pom_content);

    Ok(())
}