use super::{GitLabError, Transport, GlabTransport, ReqwestTransport};
use crate::auth::{AuthBackend, AuthError, TokenAuth};

#[derive(Debug, serde::Deserialize)]
pub struct TreeEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    #[allow(dead_code)]
    pub path: String,
}

pub struct GitLabClient {
    transport: Box<dyn Transport>,
    project_id: String,
    branch: String,
}

impl GitLabClient {
    pub fn for_project(base_url: String, project_id: String) -> Result<Self, AuthError> {
        let hostname = extract_hostname(&base_url);
        let backend = crate::auth::authenticate(&hostname)?;
        let transport: Box<dyn Transport> = match backend {
            AuthBackend::Glab { hostname } => Box::new(GlabTransport::new(hostname)),
            AuthBackend::Token { token } => {
                Box::new(ReqwestTransport::new(Box::new(TokenAuth::new(token)), base_url))
            }
        };
        Ok(Self::with_transport(transport, project_id))
    }

    pub fn with_transport(transport: Box<dyn Transport>, project_id: String) -> Self {
        Self {
            transport,
            project_id,
            branch: "main".to_string(),
        }
    }

    pub fn with_branch(mut self, branch: &str) -> Self {
        self.branch = branch.to_string();
        self
    }

    pub fn list_tree(&self, path: &str) -> Result<Vec<TreeEntry>, GitLabError> {
        let encoded_project = urlencoding::encode(&self.project_id);
        let encoded_path = urlencoding::encode(path);
        let endpoint = format!(
            "/api/v4/projects/{}/repository/tree?path={}&ref={}",
            encoded_project,
            encoded_path,
            self.branch
        );

        let (status, body) = self.transport.call(&endpoint)?;

        match status {
            200 => serde_json::from_str(&body)
                .map_err(|e| GitLabError::ParseError(e.to_string())),
            401 | 403 => Err(GitLabError::AuthError(format!(
                "GitLab returned HTTP {}",
                status
            ))),
            404 => Err(GitLabError::NotFound(path.to_string())),
            status => Err(GitLabError::HttpError {
                status,
                message: body,
            }),
        }
    }

    pub fn fetch_file(&self, path: &str) -> Result<String, GitLabError> {
        let encoded_path = urlencoding::encode(path);
        let encoded_project = urlencoding::encode(&self.project_id);
        let endpoint = format!(
            "/api/v4/projects/{}/repository/files/{}/raw?ref={}",
            encoded_project,
            encoded_path,
            self.branch
        );

        let (status, body) = self.transport.call(&endpoint)?;

        match status {
            200 => Ok(body),
            401 | 403 => Err(GitLabError::AuthError(format!(
                "GitLab returned HTTP {}",
                status
            ))),
            404 => Err(GitLabError::NotFound(path.to_string())),
            status => Err(GitLabError::HttpError {
                status,
                message: body,
            }),
        }
    }
}

fn extract_hostname(base_url: &str) -> String {
    base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("//")
        .split('/')
        .next()
        .unwrap_or(base_url)
        .split(':')
        .next()
        .unwrap_or(base_url)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTransport {
        handler: Box<dyn Fn(&str) -> Result<(u16, String), GitLabError>>,
    }

    impl Transport for MockTransport {
        fn call(&self, endpoint: &str) -> Result<(u16, String), GitLabError> {
            (self.handler)(endpoint)
        }
    }

    struct FailAuth;

    impl crate::auth::Auth for FailAuth {
        fn get_token(&self) -> Result<String, crate::auth::AuthError> {
            Err(crate::auth::AuthError::EnvVarNotSet)
        }
    }

    #[test]
    fn test_fetch_file_success() {
        let client = GitLabClient::with_transport(
            Box::new(MockTransport {
                handler: Box::new(|_endpoint| Ok((200, "file contents".to_string()))),
            }),
            "123".to_string(),
        );

        let result = client.fetch_file("skills/test/skill.json").unwrap();
        assert_eq!(result, "file contents");
    }

    #[test]
    fn test_fetch_file_auth_failure() {
        let auth = Box::new(FailAuth);
        let transport = Box::new(ReqwestTransport::new(auth, "https://gitlab.example.com".to_string()));
        let client = GitLabClient::with_transport(transport, "123".to_string());

        let result = client.fetch_file("skills/test/skill.json");
        assert!(matches!(result, Err(GitLabError::AuthError(_))));
    }

    #[test]
    fn test_fetch_file_not_found() {
        let client = GitLabClient::with_transport(
            Box::new(MockTransport {
                handler: Box::new(|_endpoint| Ok((404, "Not Found".to_string()))),
            }),
            "123".to_string(),
        );

        let result = client.fetch_file("skills/test/skill.json");
        assert!(matches!(result, Err(GitLabError::NotFound(_))));
    }

    #[test]
    fn test_fetch_file_http_error() {
        let client = GitLabClient::with_transport(
            Box::new(MockTransport {
                handler: Box::new(|_endpoint| Ok((500, "Internal Server Error".to_string()))),
            }),
            "123".to_string(),
        );

        let result = client.fetch_file("skills/test/skill.json");
        assert!(
            matches!(result, Err(GitLabError::HttpError { status: 500, .. })),
            "expected HttpError with status 500, got {:?}",
            result
        );
    }

    #[test]
    fn test_fetch_file_gitlab_auth_error() {
        let client = GitLabClient::with_transport(
            Box::new(MockTransport {
                handler: Box::new(|_endpoint| Ok((401, "Unauthorized".to_string()))),
            }),
            "123".to_string(),
        );

        let result = client.fetch_file("skills/test/skill.json");
        assert!(matches!(result, Err(GitLabError::AuthError(_))));
    }

    #[test]
    fn test_fetch_file_with_custom_branch() {
        let client = GitLabClient::with_transport(
            Box::new(MockTransport {
                handler: Box::new(|endpoint| {
                    assert!(endpoint.contains("ref=develop"));
                    Ok((200, "branch content".to_string()))
                }),
            }),
            "mygroup/myproject".to_string(),
        )
        .with_branch("develop");

        let result = client.fetch_file("skills/test/skill.json").unwrap();
        assert_eq!(result, "branch content");
    }

    #[test]
    fn test_fetch_file_url_encoding() {
        let client = GitLabClient::with_transport(
            Box::new(MockTransport {
                handler: Box::new(|endpoint| {
                    assert!(endpoint.contains("projects/mygroup%2Fmyproject"));
                    assert!(endpoint.contains("files/skills%2Ftest%2Fskill.json"));
                    Ok((200, "encoded".to_string()))
                }),
            }),
            "mygroup/myproject".to_string(),
        );

        let result = client.fetch_file("skills/test/skill.json").unwrap();
        assert_eq!(result, "encoded");
    }

    #[test]
    fn test_extract_hostname() {
        assert_eq!(extract_hostname("https://gitlab.com"), "gitlab.com");
        assert_eq!(extract_hostname("https://gitlab.example.com:8443"), "gitlab.example.com");
        assert_eq!(extract_hostname("http://gitlab.local/path"), "gitlab.local");
        assert_eq!(extract_hostname("gitlab.local"), "gitlab.local");
    }
}
