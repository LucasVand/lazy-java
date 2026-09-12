use std::{
    collections::HashSet,
    ffi::OsStr,
    fs::Metadata,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use globset::GlobSet;

use crate::{
    IMPORT_REGEX, PACKAGE_REGEX,
    build::graph::{class_info::ClassInfo, package::Package},
    utils::fs,
};

#[derive(Debug)]
pub struct NodeFile {
    pub name: String,
    pub package: Package,
    pub path: PathBuf,
    pub dependencies: HashSet<Package>,
    pub meta: Metadata,
    pub info: Option<ClassInfo>,
}

#[derive(Debug)]
pub enum Node {
    File(NodeFile),
    Directory {
        name: String,
        files: Vec<Node>,
        package: Package,
    },
}

impl Node {
    pub fn from_path(path: &Path, excluded: &GlobSet) -> Result<Node, io::Error> {
        if path.is_dir() {
            let Some(p) = path.file_name() else {
                return Err(io::Error::new(ErrorKind::Other, "No name"));
            };
            let mut files = Vec::new();
            for dir in fs::read_dir(&path)? {
                let dir = dir?;
                let dir_path = dir.path();
                let ex = excluded.is_match(&dir_path) || excluded.is_match(dir.file_name());
                let is_java = dir_path.extension() == Some(OsStr::new("java")) || dir_path.is_dir();

                if !ex && is_java {
                    files.push(Node::from_path(&dir_path, excluded)?);
                }
            }
            let package =
                if let Some(Node::File(file)) = files.iter().find(|f| matches!(f, Node::File(_))) {
                    let mut p = file.package.clone();
                    p.pop();
                    p.join("*")
                } else {
                    Package::empty()
                };

            // resolving the interpackage dependencies
            // ISSUE: this should become more comprehensive but to ensure its the same as before
            // this is how it is for now
            let list: HashSet<Package> = files
                .iter()
                .filter_map(|f| match f {
                    Node::File(file) => Some(file.package.clone()),
                    Node::Directory { .. } => None,
                })
                .collect();

            for file in files.iter_mut() {
                match file {
                    Node::Directory { .. } => continue,
                    Node::File(file) => {
                        file.dependencies.extend(list.clone());
                        file.dependencies.remove(&file.package);
                    }
                }
            }

            Ok(Node::Directory {
                name: p.to_string_lossy().to_string(),
                files: files,
                package,
            })
        } else {
            let Some(file_name) = path.file_name() else {
                return Err(io::Error::new(ErrorKind::Other, "No name"));
            };
            let Some(stem) = path.file_stem() else {
                return Err(io::Error::new(ErrorKind::Other, "No stem"));
            };

            let meta = path.metadata()?;
            let contents = fs::read_to_string(&path)?;

            let package_str = if let Some(p) = PACKAGE_REGEX.captures(&contents)
                && let Some(name) = p.name("package")
            {
                name.as_str()
            } else {
                ""
            };
            let info = ClassInfo::create(&contents);

            let dependencies: HashSet<Package> = IMPORT_REGEX
                .captures_iter(&contents)
                .filter_map(|cap| {
                    if let Some(import) = cap.name("import") {
                        let stat = cap.name("static");
                        let package = Package::from_string(import.as_str(), stat.is_some());

                        return Some(package);
                    }
                    None
                })
                .collect();

            let mut package = Package::from_string(package_str, false);
            package.push(stem.to_string_lossy());

            let name = file_name.to_string_lossy().to_string();

            log::trace!(
                "Creating node name: {}, dependencies: {:?}, path: {}",
                &name,
                &dependencies,
                &path.display(),
            );
            Ok(Node::File(NodeFile {
                info,
                name,
                package,
                path: path.to_path_buf(),
                dependencies,
                meta,
            }))
        }
    }

    /// Returns a depth-first iterator over all nodes in the tree,
    /// visiting a directory before its children.
    pub fn iter(&self) -> NodeIter<'_> {
        NodeIter { stack: vec![self] }
    }

    /// Returns a depth-first iterator over the file nodes only,
    /// yielding the extracted file data and skipping directories.
    pub fn files(&self) -> NodeFileIter<'_> {
        NodeFileIter { stack: vec![self] }
    }

    /// Returns the file node whose path matches `path`.
    pub fn find_path(&self, path: &Path) -> Option<&NodeFile> {
        self.files().find(|file| file.path == path)
    }

    /// Returns the file node whose path matches `path`.
    pub fn find_package(&self, search_package: &Package) -> Option<&Node> {
        self.iter().find(|file| match file {
            Node::File(file) => file.package == *search_package,
            Node::Directory { package, .. } => search_package == package,
        })
    }
}

pub struct NodeIter<'a> {
    stack: Vec<&'a Node>,
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<&'a Node> {
        let node = self.stack.pop()?;
        if let Node::Directory { files, .. } = node {
            self.stack.extend(files.iter().map(|f| f));
        }
        Some(node)
    }
}

pub struct NodeFileIter<'a> {
    stack: Vec<&'a Node>,
}

impl<'a> Iterator for NodeFileIter<'a> {
    type Item = &'a NodeFile;

    fn next(&mut self) -> Option<&'a NodeFile> {
        while let Some(node) = self.stack.pop() {
            match node {
                Node::File(file) => return Some(file),
                Node::Directory { files, .. } => {
                    self.stack.extend(files.iter().map(|f| f));
                }
            }
        }
        None
    }
}

impl<'a> IntoIterator for &'a Node {
    type Item = &'a Node;
    type IntoIter = NodeIter<'a>;

    fn into_iter(self) -> NodeIter<'a> {
        self.iter()
    }
}
