use std::str::FromStr;

use regex::Captures;
use strum::EnumString;

use crate::CLASS_REGEX;

#[derive(EnumString, Default, Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum ClassType {
    #[default]
    #[strum(serialize = "@interface")]
    AtInterface,
    #[strum(serialize = "class")]
    Class,
    #[strum(serialize = "record")]
    Record,
    #[strum(serialize = "enum")]
    Enum,
    #[strum(serialize = "interface")]
    Interface,
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub object_type: ClassType,
    pub sub_classes: Vec<ClassInfo>,
}

/// Finds an object's body: the text between the opening brace of its
/// declaration and the matching closing brace (exclusive), using brace counting
/// so nested types and method bodies are handled correctly.
pub fn class_body(content: &str, open_brace: usize) -> Option<String> {
    if content.as_bytes().get(open_brace) != Some(&b'{') {
        return None;
    }
    let mut depth = 0u32;
    for (i, b) in content.as_bytes()[open_brace..].iter().copied().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(content[open_brace + 1..open_brace + i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

impl ClassInfo {
    /// Parses the first type declaration in `content` into a `ClassInfo`,
    /// recursing into nested type declarations found inside its body.
    pub fn create(content: &str) -> Option<ClassInfo> {
        let class = CLASS_REGEX.captures(content)?;
        Self::from_captures(&class, content)
    }

    fn from_captures(class: &Captures<'_>, content: &str) -> Option<ClassInfo> {
        let implements: Vec<String> = class
            .name("implements")
            .map(|impls| {
                impls
                    .as_str()
                    .split(',')
                    .map(|i| i.trim().to_string())
                    .collect()
            })
            .unwrap_or_default();

        let sub_classes = class_body(content, class.get_match().end() - 1)
            .map(|body| {
                CLASS_REGEX
                    .captures_iter(&body)
                    .filter_map(|inner| Self::from_captures(&inner, &body))
                    .collect()
            })
            .unwrap_or_default();

        Some(ClassInfo {
            name: class.name("class")?.as_str().to_string(),
            extends: class.name("extends").map(|e| e.as_str().to_string()),
            implements,
            object_type: ClassType::from_str(class.name("type")?.as_str()).unwrap_or_default(),
            sub_classes,
        })
    }
    pub fn is_processor(&self) -> bool {
        if self.object_type == ClassType::AtInterface {
            return true;
        }
        if let Some(ext) = self.extends.as_ref()
            && ext.trim() == "AbstractProcessor"
        {
            return true;
        }
        return false;
    }
}

#[cfg(test)]
mod tests {
    use super::ClassInfo;

    #[test]
    fn create_builds_nested_class_tree() {
        let src = r#"package app;

public class Outer extends Base implements Runnable {

    public static class Inner {
        public void run() {}
    }

    void method() {
        class Local {}
    }
}
"#;

        let info = ClassInfo::create(src).unwrap();
        assert_eq!(info.name, "Outer");
        assert_eq!(info.extends.as_deref(), Some("Base"));
        assert_eq!(info.implements, vec!["Runnable"]);
        // Both the member class `Inner` and the method-local `Local` are found,
        // since any line-start declaration inside the body is discovered.
        assert_eq!(info.sub_classes.len(), 2);
        assert_eq!(info.sub_classes[0].name, "Inner");
        assert!(info.sub_classes[0].sub_classes.is_empty());
        assert_eq!(info.sub_classes[1].name, "Local");
    }

    #[test]
    fn create_handles_annotation_type() {
        let src = "public @interface Deprecated3 {\n    String value() default \"\";\n}\n";
        let info = ClassInfo::create(src).unwrap();
        assert_eq!(info.name, "Deprecated3");
        assert!(info.implements.is_empty());
    }
}

