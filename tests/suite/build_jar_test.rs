use predicates::prelude::predicate;
use std::process::Command as StdCommand;

use crate::support::{lazy_java, Project};

#[test]
fn build_jar_creates_executable_jar() -> Result<(), Box<dyn std::error::Error>> {
    let p = Project::from_fixture("build-and-run")?;
    let dest = &p.dir;

    // Build first
    lazy_java(dest)?.args(["build"]).assert().success();

    // Create jar
    lazy_java(dest)?
        .args(["build", "jar", "--entry-point", "Main"])
        .assert()
        .success();

    assert!(
        dest.join("target").join("build.jar").exists(),
        "build.jar should exist"
    );

    // Run the jar with plain java
    let output = StdCommand::new("java")
        .arg("-jar")
        .arg(dest.join("target").join("build.jar"))
        .output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello world!"));
    assert!(stdout.contains("Welcome to your LazyJava project"));

    Ok(())
}

#[test]
fn build_fat_jar_self_contained() -> Result<(), Box<dyn std::error::Error>> {
    let p = Project::from_fixture("with-dependency")?;
    let dest = &p.dir;

    // Add a dependency and build
    lazy_java(dest)?
        .args(["add", "org.apache.commons", "commons-lang3", "3.20.0"])
        .assert()
        .success();

    lazy_java(dest)?.args(["build"]).assert().success();

    // Create fat jar
    lazy_java(dest)?
        .args(["build", "jar", "--entry-point", "Main", "--fat"])
        .assert()
        .success();

    assert!(
        dest.join("target").join("build.jar").exists(),
        "build.jar should exist"
    );

    // Remove lib to prove fat jar is self-contained
    std::fs::remove_dir_all(dest.join("target").join("lib"))?;
    std::fs::remove_dir_all(dest.join("target").join("lib-annotations"))?;

    // Run the fat jar — should work without any external libs
    let output = StdCommand::new("java")
        .arg("-jar")
        .arg(dest.join("target").join("build.jar"))
        .output()?;

    assert!(
        output.status.success(),
        "fat jar should run without external lib dir, stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello world from lazy-java"));

    Ok(())
}

#[test]
fn run_jar_using_lazy_java() -> Result<(), Box<dyn std::error::Error>> {
    let p = Project::from_fixture("build-and-run")?;
    let dest = &p.dir;

    // Build and create jar
    lazy_java(dest)?
        .args(["build", "jar", "--entry-point", "Main"])
        .assert()
        .success();

    // Run via lazy-java --jar
    lazy_java(dest)?
        .args(["run", "--jar", "--no-build"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello world!"));

    Ok(())
}

#[test]
fn run_jar_errors_when_not_built() -> Result<(), Box<dyn std::error::Error>> {
    let p = Project::from_fixture("build-and-run")?;
    let dest = &p.dir;

    // Try running jar without building it first
    lazy_java(dest)?
        .args(["run", "--jar", "--no-build"])
        .assert()
        .failure();

    Ok(())
}