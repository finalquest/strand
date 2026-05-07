use super::GitLabError;
use std::process::Output;

pub trait Transport {
    fn call(&self, endpoint: &str) -> Result<(u16, String), GitLabError>;
}

pub struct ReqwestTransport {
    auth: Box<dyn crate::auth::Auth>,
    base_url: String,
    client: reqwest::blocking::Client,
}

impl ReqwestTransport {
    pub fn new(auth: Box<dyn crate::auth::Auth>, base_url: String) -> Self {
        Self {
            auth,
            base_url,
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl Transport for ReqwestTransport {
    fn call(&self, endpoint: &str) -> Result<(u16, String), GitLabError> {
        let token = self
            .auth
            .get_token()
            .map_err(|e| GitLabError::AuthError(e.to_string()))?;

        let url = format!("{}{}", self.base_url.trim_end_matches('/'), endpoint);
        let resp = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", token)
            .send()?;
        let status = resp.status().as_u16();
        let body = resp.text()?;
        Ok((status, body))
    }
}

pub struct GlabTransport {
    hostname: String,
    runner: Box<dyn Fn(&str, &[&str]) -> Result<Output, std::io::Error>>,
}

impl GlabTransport {
    pub fn new(hostname: String) -> Self {
        Self {
            hostname,
            runner: Box::new(|cmd, args| std::process::Command::new(cmd).args(args).output()),
        }
    }

    #[cfg(test)]
    pub fn with_runner<F>(hostname: String, runner: F) -> Self
    where
        F: Fn(&str, &[&str]) -> Result<Output, std::io::Error> + 'static,
    {
        Self {
            hostname,
            runner: Box::new(runner),
        }
    }
}

impl Transport for GlabTransport {
    fn call(&self, endpoint: &str) -> Result<(u16, String), GitLabError> {
        // glab api automatically prefixes with /api/v4, so strip it if present
        let glab_endpoint = endpoint.strip_prefix("/api/v4").unwrap_or(endpoint);
        let output =
            (self.runner)("glab", &["api", "--hostname", &self.hostname, glab_endpoint])?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok((200, stdout))
        } else {
            let status = output.status.code().unwrap_or(1) as u16;
            // glab prints HTTP status in stderr (e.g. "404 Not Found")
            if stderr.contains("404") {
                return Err(GitLabError::NotFound(glab_endpoint.to_string()));
            }
            Err(GitLabError::HttpError {
                status,
                message: stderr,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glab_transport_success() {
        let transport =
            GlabTransport::with_runner("gitlab.example.com".to_string(), |cmd, args| {
                assert_eq!(cmd, "glab");
                assert_eq!(
                    args,
                    &["api", "--hostname", "gitlab.example.com", "/projects/123"]
                );
                Ok(Output {
                    status: std::process::ExitStatus::default(),
                    stdout: b"[{\"name\":\"foo\"}]".to_vec(),
                    stderr: vec![],
                })
            });

        let (status, body) = transport.call("/projects/123").unwrap();
        assert_eq!(status, 200);
        assert_eq!(body, "[{\"name\":\"foo\"}]");
    }

    #[test]
    fn test_glab_transport_command_failure() {
        use std::os::unix::process::ExitStatusExt;
        let transport =
            GlabTransport::with_runner("gitlab.example.com".to_string(), |_cmd, _args| {
                Ok(Output {
                    status: std::process::ExitStatus::from_raw(1),
                    stdout: vec![],
                    stderr: b"error: 500 Internal Server Error".to_vec(),
                })
            });

        let result = transport.call("/projects/123");
        assert!(matches!(
            result,
            Err(GitLabError::HttpError { status: 1, .. })
        ));
    }

    #[test]
    fn test_glab_transport_404_maps_to_not_found() {
        use std::os::unix::process::ExitStatusExt;
        let transport =
            GlabTransport::with_runner("gitlab.example.com".to_string(), |_cmd, _args| {
                Ok(Output {
                    status: std::process::ExitStatus::from_raw(1),
                    stdout: vec![],
                    stderr: b"glab: 404 Not Found".to_vec(),
                })
            });

        let result = transport.call("/projects/123");
        assert!(matches!(result, Err(GitLabError::NotFound(_))));
    }

    #[test]
    fn test_glab_transport_not_installed() {
        let transport =
            GlabTransport::with_runner("gitlab.example.com".to_string(), |_cmd, _args| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "No such file or directory",
                ))
            });

        let result = transport.call("/projects/123");
        assert!(matches!(result, Err(GitLabError::Io(_))));
    }

    #[test]
    fn test_glab_transport_command_construction() {
        let transport = GlabTransport::with_runner("gitlab.com".to_string(), |cmd, args| {
            assert_eq!(cmd, "glab");
            assert_eq!(args[0], "api");
            assert_eq!(args[1], "--hostname");
            assert_eq!(args[2], "gitlab.com");
            assert_eq!(args[3], "/projects/123/repository/tree?path=skills");
            Ok(Output {
                status: std::process::ExitStatus::default(),
                stdout: b"[]".to_vec(),
                stderr: vec![],
            })
        });

        let (status, body) = transport
            .call("/api/v4/projects/123/repository/tree?path=skills")
            .unwrap();
        assert_eq!(status, 200);
        assert_eq!(body, "[]");
    }
}
