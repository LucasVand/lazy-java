use predicates::prelude::{PredicateBooleanExt, predicate};

use crate::support::{lazy_java, Project};

#[test]
fn sync_build_run_with_maven_dependency() -> Result<(), Box<dyn std::error::Error>> {
    let p = Project::from_fixture("with-dependency")?;
    let dest = &p.dir;

    // Step 1: Add dependency — resolves from Maven, downloads JAR, creates lock file
    lazy_java(dest)?
        .args(["add", "org.apache.commons", "commons-lang3", "3.20.0"])
        .assert()
        .success();

    assert!(
        dest.join("lazy-java.lock").exists(),
        "lock file should exist after add"
    );

    assert!(
        dest.join("target").join("lib").is_dir(),
        "lib dir should exist after add"
    );

    // Step 2: Build — first build is a full build
    lazy_java(dest)?
        .args(["build"])
        .assert()
        .success()
        .stdout(predicate::str::contains("full build"));

    assert!(
        dest.join("target").join("bin").join("Main.class").exists(),
        "Main.class should exist after build"
    );

    // Step 3: A build with no changes must not trigger a full rebuild.
    lazy_java(dest)?
        .args(["build"])
        .assert()
        .success()
        .stdout(predicate::str::contains("full build").not());

    // Step 4: Adding a new remote dependency must force a full rebuild.
    lazy_java(dest)?
        .args(["add", "org.apache.commons", "commons-collections4", "4.4"])
        .assert()
        .success();

    lazy_java(dest)?
        .args(["build"])
        .assert()
        .success()
        .stdout(predicate::str::contains("full build"));

    // Step 5: Adding a remote dependency that bundles an annotation processor
    // (lands in lib-annotations) must also force a full rebuild.
    lazy_java(dest)?
        .args(["add", "org.immutables", "value", "2.10.1"])
        .assert()
        .success();

    assert!(
        dest.join("target").join("lib-annotations").is_dir(),
        "lib-annotations dir should exist after add"
    );

    lazy_java(dest)?
        .args(["build"])
        .assert()
        .success()
        .stdout(predicate::str::contains("full build"));

    // Step 6: Run and assert output from commons-lang3 usage
    lazy_java(dest)?
        .args(["run", "--no-build", "Main"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello world from lazy-java"))
        .stdout(predicate::str::contains("hello wor..."))
        .stdout(predicate::str::contains("avaj-yzal morf dlrow olleh"));

    Ok(())
}