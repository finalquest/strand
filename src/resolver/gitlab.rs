use super::{ResolverError, SkillSource};
use crate::gitlab::GitLabClient;

pub struct GitLabSkillSource {
    client: GitLabClient,
}

impl GitLabSkillSource {
    pub fn new(client: GitLabClient) -> Self {
        Self { client }
    }
}

impl SkillSource for GitLabSkillSource {
    fn read_file(&self, path: &str) -> Result<String, ResolverError> {
        self.client.fetch_file(path).map_err(ResolverError::GitLab)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gitlab::{GitLabError, Transport};

    struct MockTransport {
        handler: Box<dyn Fn(&str) -> Result<(u16, String), GitLabError>>,
    }

    impl Transport for MockTransport {
        fn call(&self, endpoint: &str) -> Result<(u16, String), GitLabError> {
            (self.handler)(endpoint)
        }
    }

    #[test]
    fn test_gitlab_skill_source_read_file() {
        let client = GitLabClient::with_transport(
            Box::new(MockTransport {
                handler: Box::new(|_endpoint| Ok((200, "remote skill content".to_string()))),
            }),
            "123".to_string(),
        );

        let source = GitLabSkillSource::new(client);
        let content = source.read_file("skills/test/skill.json").unwrap();
        assert_eq!(content, "remote skill content");
    }

    #[test]
    fn test_gitlab_skill_source_not_found() {
        let client = GitLabClient::with_transport(
            Box::new(MockTransport {
                handler: Box::new(|_endpoint| Ok((404, "Not Found".to_string()))),
            }),
            "123".to_string(),
        );

        let source = GitLabSkillSource::new(client);
        let result = source.read_file("skills/test/skill.json");
        assert!(matches!(result, Err(ResolverError::GitLab(_))));
    }
}
