use super::document::ParsedDocument;

#[derive(Debug, thiserror::Error)]
pub enum FrontmatterError {
    #[error("missing frontmatter delimiter")]
    MissingDelimiter,
    #[error("yaml parse error: {0}")]
    YamlError(#[from] serde_yaml::Error),
}

/// Parse a document with `---` delimited YAML frontmatter.
pub fn parse(raw: &str) -> Result<ParsedDocument, FrontmatterError> {
    let trimmed = raw.trim_start();

    if !trimmed.starts_with("---") {
        return Ok(ParsedDocument {
            frontmatter: serde_yaml::Mapping::new(),
            body: raw.to_string(),
            raw: raw.to_string(),
        });
    }

    let after_first = &trimmed[3..];
    let end = after_first
        .find("\n---")
        .or_else(|| after_first.find("\r\n---"))
        .ok_or(FrontmatterError::MissingDelimiter)?;

    let yaml_str = &after_first[..end];
    let body_start = 3 + end + 4; // skip `\n---`
    let body = trimmed[body_start..].trim_start_matches('\n').trim_start_matches('\r');

    let frontmatter: serde_yaml::Mapping = serde_yaml::from_str(yaml_str)?;

    Ok(ParsedDocument {
        frontmatter,
        body: body.to_string(),
        raw: raw.to_string(),
    })
}

/// Serialize a document back to raw string.
pub fn serialize(frontmatter: &serde_yaml::Mapping, body: &str) -> String {
    let without_top_level_nulls: serde_yaml::Mapping = frontmatter
        .iter()
        .filter(|(_, value)| !value.is_null())
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();

    if without_top_level_nulls.is_empty() {
        return body.to_string();
    }

    let yaml = serde_yaml::to_string(&without_top_level_nulls).unwrap_or_default();

    format!("---\n{}---\n{}", yaml, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_frontmatter() {
        let raw = "---\ntitle: Hello\ndate: 2024-01-01\n---\nBody text";
        let doc = parse(raw).unwrap();
        assert_eq!(doc.body, "Body text");
        assert!(doc.frontmatter.contains_key(&serde_yaml::Value::String("title".into())));
    }

    #[test]
    fn parse_no_frontmatter() {
        let raw = "Just plain markdown";
        let doc = parse(raw).unwrap();
        assert_eq!(doc.body, "Just plain markdown");
        assert!(doc.frontmatter.is_empty());
    }

    #[test]
    fn serialize_roundtrip() {
        let raw = "---\ntitle: Hello\n---\nBody";
        let doc = parse(raw).unwrap();
        let serialized = serialize(&doc.frontmatter, &doc.body);
        let doc2 = parse(&serialized).unwrap();
        assert_eq!(doc.body.trim(), doc2.body.trim());
    }

    #[test]
    fn serialize_removes_nulls() {
        let mut fm = serde_yaml::Mapping::new();
        fm.insert(
            serde_yaml::Value::String("title".into()),
            serde_yaml::Value::String("Hello".into()),
        );
        fm.insert(
            serde_yaml::Value::String("draft".into()),
            serde_yaml::Value::Null,
        );
        let result = serialize(&fm, "body");
        assert!(!result.contains("null"));
    }
    #[test]
    fn serialize_removes_only_top_level_nulls_and_preserves_nested_values() {
        let mut frontmatter = serde_yaml::Mapping::new();
        frontmatter.insert(
            serde_yaml::Value::String("top_level".into()),
            serde_yaml::Value::Null,
        );
        frontmatter.insert(
            serde_yaml::Value::String("nested".into()),
            serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter([
                (serde_yaml::Value::String("keep".into()), serde_yaml::Value::Null),
                (
                    serde_yaml::Value::String("title".into()),
                    serde_yaml::Value::String("Nested".into()),
                ),
            ])),
        );
        frontmatter.insert(
            serde_yaml::Value::String("items".into()),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::Null]),
        );

        let serialized = serialize(&frontmatter, "body");
        assert!(serialized.contains("keep: null"));
        let reparsed = parse(&serialized).unwrap();
        assert!(!reparsed.frontmatter.contains_key(&serde_yaml::Value::String("top_level".into())));
        assert_eq!(
            reparsed.frontmatter[&serde_yaml::Value::String("nested".into())]
                [&serde_yaml::Value::String("keep".into())],
            serde_yaml::Value::Null
        );
        assert_eq!(
            reparsed.frontmatter[&serde_yaml::Value::String("items".into())],
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::Null])
        );
    }
}
