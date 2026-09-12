#![deny(clippy::disallowed_methods)]
#![cfg_attr(test, allow(clippy::disallowed_methods))]

use std::sync::LazyLock;

use regex::{Regex, RegexBuilder};

pub mod args;
pub mod build;
pub mod clean;
pub mod config;
pub mod create;
pub mod find;
pub mod generate;
pub mod import;
mod lazy_java;
pub mod lazy_java_error;
pub mod lock_file;
pub mod lsp;
pub mod maven_central;
pub mod packages;
pub mod run;
pub mod utils;

pub use lazy_java::LazyJava;
pub use utils::{Context, ContextNoConfig, ContextNoConfigExcluded};

pub const BUILD_FOLDER: &str = "bin";
pub const SRC_FOLDER: &str = "src";
pub const LIB_FOLDER: &str = "lib";
pub const TARGET_FOLDER: &str = "target";

pub const MAVEN_URL: &str = "https://repo1.maven.org/maven2/";

pub const LOCK_FILE_NAME: &str = "lazy-java.lock";
pub const CONFIG_FILE_NAME: &str = "lazy-java.toml";
pub const BUILD_METADATA_NAME: &str = ".lazy-java-build";

pub const JAVAC_SEPARATOR: char = if cfg!(target_os = "windows") {
    ';'
} else {
    ':'
};

pub fn create_maven_url(group: &str, artifact: &str) -> String {
    format!("{}{}/{}/", MAVEN_URL, group.replace(".", "/"), artifact)
}

pub static IMPORT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*import\s*(?<static>static)?\s*(?<import>\S+);").unwrap());
pub static PACKAGE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"^\s*package\s*(?<package>.*);")
        .unicode(true)
        .build()
        .unwrap()
});

pub static MAIN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"public static void main(.*) \{(?<content>[\s\S]*)\}")
        .unicode(true)
        .multi_line(true)
        .build()
        .unwrap()
});
pub static CLASS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    let re = RegexBuilder::new(
        r"(?m)^\s*(?:(?:public|protected|private|static|abstract|final|sealed|non-sealed|default|strictfp|synchronized)\s+|@[\w$][\w$.]*(?:\s*\([^)]*\))?\s+)*(?<type>class|interface|record|enum|@interface)\s+(?<class>[A-Za-z_$][A-Za-z0-9_$]*)(?<generics><[^>]*>)?(?<recordParams>\(\s*(?:[^()]|\([^()]*\))*\s*\))?(?:\s+extends\s+(?<extends>[^{\n]+?))?(?:\s+implements\s+(?<implements>[^{\n]+?))?(?:\s+permits\s+(?<permits>[^{\n]+?))?\s*\{",
    )
    .multi_line(true)
    .unicode(true)
    .build();
    re.unwrap()
});
