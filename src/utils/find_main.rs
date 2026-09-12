use std::{
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
};

use globset::{Glob, GlobSet, GlobSetBuilder};
use walkdir::{DirEntry, WalkDir};

use crate::{CLASS_REGEX, MAIN_REGEX, PACKAGE_REGEX, utils::fs};

#[derive(Debug)]
pub struct MainClass {
    pub path: PathBuf,
    pub classname: String,
    pub full_package_name: String,
}

pub fn find_main_classes(src: &Path, excluded: &[String]) -> Result<Vec<MainClass>, io::Error> {
    log::debug!("Scanning for main classes in {:?}", src);
    let mut main_classes: Vec<MainClass> = Vec::new();

    let java_files = find_java_files(src, excluded);
    log::debug!("Found {} Java files to scan", java_files.len());

    for file in java_files {
        let content = fs::read_to_string(&file)?;
        let package_captures = PACKAGE_REGEX.captures(&content);

        let package = match package_captures {
            Some(cap) => {
                let package = cap.name("package").unwrap();
                package.as_str()
            }
            None => "",
        };

        if let Some(class) = CLASS_REGEX.captures(&content) {
            let classname = class.name("class").unwrap().as_str().to_string();
            if let Some(body) = class_body(&content, class.get_match().end() - 1) {
                let mut found_classes = find_main_class(&classname, &body, package, &file)?;

                if !found_classes.is_empty() {
                    log::debug!(
                        "Found {} main class(es) in {:?}",
                        found_classes.len(),
                        file
                    );
                }
                main_classes.append(&mut found_classes);
            }
        }
    }
    log::debug!("Total main classes found: {}", main_classes.len());
    Ok(main_classes)
}

/// Returns the text between the opening brace at `open_brace` and its matching
/// closing brace (exclusive), using brace counting so nested types and method
/// bodies are handled correctly.
fn class_body(src: &str, open_brace: usize) -> Option<String> {
    if src.as_bytes().get(open_brace) != Some(&b'{') {
        return None;
    }
    let mut depth = 0u32;
    for (i, b) in src.as_bytes()[open_brace..].iter().copied().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(src[open_brace + 1..open_brace + i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn find_main_class(
    classname: &str,
    content: &str,
    package: &str,
    file: &PathBuf,
) -> Result<Vec<MainClass>, io::Error> {
    let mut removed_inner_content = content.to_string();
    let main = MAIN_REGEX.captures(content);

    let mut main_vec: Vec<MainClass> = Vec::new();
    if let Some(cap) = main {
        removed_inner_content.replace_range(cap.get_match().range(), "");
        let full_package = if !package.is_empty() {
            format!("{}.{}", package, classname)
        } else {
            classname.to_string()
        };

        let class = MainClass {
            path: file.to_path_buf(),
            classname: classname.to_string(),
            full_package_name: full_package,
        };

        main_vec.push(class);
    }

    for inner in CLASS_REGEX.captures_iter(&removed_inner_content) {
        let inner_name = inner.name("class").unwrap().as_str().to_string();
        if let Some(inner_body) = class_body(
            &removed_inner_content,
            inner.get_match().end() - 1,
        ) {
            let mut pack = package.to_string();
            if !pack.is_empty() {
                pack.push('.');
            }
            pack.push_str(classname);

            let mut classes = find_main_class(&inner_name, &inner_body, &pack, file)?;
            main_vec.append(&mut classes);
        }
    }

    Ok(main_vec)
}

pub fn find_java_files(root: &Path, excluded: &[String]) -> Vec<PathBuf> {
    log::debug!("Recursively searching for Java files in {:?}", root);
    let mut java_files: Vec<PathBuf> = Vec::new();

    let set = build_globset(excluded);

    for entry in WalkDir::new(root) {
        if let Ok(file) = entry
            && file.file_type().is_file()
            && file.path().extension() == Some(OsStr::new("java"))
            && !is_excluded(&file, &set)
        {
            java_files.push(file.into_path());
        }
    }

    java_files
}
pub fn find_java_files_glob(root: &Path, set: &GlobSet) -> Vec<PathBuf> {
    log::debug!("Recursively searching for Java files in {:?}", root);
    let mut java_files: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(root) {
        if let Ok(file) = entry
            && file.file_type().is_file()
            && file.path().extension() == Some(OsStr::new("java"))
            && !is_excluded(&file, &set)
        {
            java_files.push(file.into_path());
        }
    }

    java_files
}

fn is_excluded(file: &DirEntry, glob: &GlobSet) -> bool {
    glob.is_match(file.path()) || glob.is_match(file.file_name())
}

fn build_globset(excluded: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for rule in excluded {
        if let Ok(glob_rule) = Glob::new(rule) {
            builder.add(glob_rule);
        } else {
            log::warn!("Invalid glob rule, \"{}\" is not a valid rule", rule);
        }
    }

    builder.build().unwrap()
}

#[cfg(test)]
mod tests {
    use std::{env, fs, io, path::PathBuf};

    use crate::utils::find_main::{MainClass, find_main_classes};

    #[test]
    fn find_main_test() -> Result<(), io::Error> {
        let mut current = env::current_dir()?;
        current.push("test_filesystem");
        current.push("find_main_classes_test");

        let classes = find_main_classes(&current, &vec![])?;

        let expect1 = MainClass {
            path: PathBuf::from("./test_filesystem/find_main_classes_test/Test1.java"),
            classname: "Test1".to_string(),
            full_package_name: "Test1".to_string(),
        };
        let expect2 = MainClass {
            path: PathBuf::from("./test_filesystem/find_main_classes_test/Test2.java"),
            classname: "Test2".to_string(),
            full_package_name: "Test2".to_string(),
        };
        let expect3 = MainClass {
            path: PathBuf::from("./test_filesystem/find_main_classes_test/Test3.java"),
            classname: "Test3".to_string(),
            full_package_name: "Test3".to_string(),
        };
        let expect4 = MainClass {
            path: PathBuf::from("./test_filesystem/find_main_classes_test/Test4.java"),
            classname: "Test4".to_string(),
            full_package_name: "Test4".to_string(),
        };
        let expect5 = MainClass {
            path: PathBuf::from("./test_filesystem/find_main_classes_test/dir1/Test5.java"),
            classname: "Test5".to_string(),
            full_package_name: "dir1.Test5".to_string(),
        };

        let expected = vec![expect3, expect2, expect4, expect1, expect5];

        dbg!(&classes);
        for index in 0..5 {
            let main = &classes[index];
            let expect = expected.iter().find(|m| {
                return m.full_package_name == main.full_package_name;
            });

            assert!(
                expect.is_some(),
                "Couldn't find expected main class matching a class found"
            );
            let expect = expect.unwrap();
            let con_main = fs::canonicalize(&main.path)?;
            let con_expec = fs::canonicalize(&expect.path)?;

            assert_eq!(con_main, con_expec, "Paths did not match");
            assert_eq!(main.classname, expect.classname, "Classnames did not match");
            assert_eq!(
                main.full_package_name, expect.full_package_name,
                "Package names did not match"
            );
            println!("Passed Test {}", index);
        }

        return Ok(());
    }
}
