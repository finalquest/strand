use super::{ResolverError, SkillSource};
use std::path::PathBuf;

pub struct LocalSkillSource {
    base_path: PathBuf,
}

impl LocalSkillSource {
    pub fn new(base_path: impl Into<PathBuf>) -> Result<Self, ResolverError> {
        let path = base_path.into();
        if !path.is_dir() {
            return Err(ResolverError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("not a directory: {}", path.display()),
            )));
        }
        Ok(Self { base_path: path })
    }
}

impl SkillSource for LocalSkillSource {
    fn read_file(&self, path: &str) -> Result<String, ResolverError> {
        let full_path = self.base_path.join(path);
        std::fs::read_to_string(&full_path).map_err(ResolverError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_local_skill_source_read_file() {
        let temp_dir = std::env::temp_dir().join("strand_test_local_source");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let test_file = temp_dir.join("skill.json");
        let mut file = std::fs::File::create(&test_file).unwrap();
        file.write_all(b"skill content").unwrap();

        let source = LocalSkillSource::new(&temp_dir).unwrap();
        let content = source.read_file("skill.json").unwrap();
        assert_eq!(content, "skill content");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_local_skill_source_not_found() {
        let temp_dir = std::env::temp_dir().join("strand_test_local_source_missing");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let source = LocalSkillSource::new(&temp_dir);
        assert!(source.is_err());
    }
}
