use std::process::Output;

pub struct GlabAuth {
    runner: Box<dyn Fn(&str, &[&str]) -> Result<Output, std::io::Error>>,
}

impl GlabAuth {
    pub fn new() -> Self {
        Self {
            runner: Box::new(|cmd, args| std::process::Command::new(cmd).args(args).output()),
        }
    }

    #[cfg(test)]
    pub fn with_runner<F>(runner: F) -> Self
    where
        F: Fn(&str, &[&str]) -> Result<Output, std::io::Error> + 'static,
    {
        Self {
            runner: Box::new(runner),
        }
    }

    pub fn is_installed(&self) -> bool {
        match (self.runner)("glab", &["--version"]) {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    pub fn is_authenticated_for(&self, hostname: &str) -> bool {
        let output = match (self.runner)("glab", &["auth", "status", "--hostname", hostname]) {
            Ok(output) => output,
            Err(_) => return false,
        };
        output.status.success()
    }

    /// Returns the list of hostnames configured in glab auth status output.
    /// Parses lines that are not indented (hostnames) vs indented (details).
    pub fn configured_hosts(&self) -> Vec<String> {
        let output = match (self.runner)("glab", &["auth", "status"]) {
            Ok(output) => output,
            Err(_) => return vec![],
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}\n{}", stdout, stderr);
        combined
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim_start();
                // Hostname lines have no leading whitespace; detail lines are indented
                if !line.starts_with(' ') && !line.starts_with('\t') && !trimmed.is_empty() {
                    // Looks like a hostname (e.g. gitlab.com, gitlab.example.com)
                    // Must contain a dot to be a hostname, not an error banner
                    if trimmed.contains('.') {
                        Some(trimmed.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for GlabAuth {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_installed_success() {
        let auth = GlabAuth::with_runner(|cmd, args| {
            assert_eq!(cmd, "glab");
            assert_eq!(args, &["--version"]);
            Ok(Output {
                status: std::process::ExitStatus::default(),
                stdout: vec![],
                stderr: vec![],
            })
        });
        assert!(auth.is_installed());
    }

    #[test]
    fn test_is_installed_failure() {
        let auth = GlabAuth::with_runner(|_cmd, _args| {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No such file or directory",
            ))
        });
        assert!(!auth.is_installed());
    }

    #[test]
    fn test_is_authenticated_for_success() {
        let auth = GlabAuth::with_runner(|cmd, args| {
            assert_eq!(cmd, "glab");
            assert_eq!(args, &["auth", "status", "--hostname", "gitlab.com"]);
            Ok(Output {
                status: std::process::ExitStatus::default(),
                stdout: vec![],
                stderr: vec![],
            })
        });
        assert!(auth.is_authenticated_for("gitlab.com"));
    }

    #[test]
    fn test_is_authenticated_for_failure() {
        use std::os::unix::process::ExitStatusExt;
        let auth = GlabAuth::with_runner(|_cmd, _args| {
            Ok(Output {
                status: std::process::ExitStatus::from_raw(1),
                stdout: vec![],
                stderr: b"not authenticated".to_vec(),
            })
        });
        assert!(!auth.is_authenticated_for("gitlab.com"));
    }

    #[test]
    fn test_is_authenticated_for_not_installed() {
        let auth = GlabAuth::with_runner(|_cmd, _args| {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No such file or directory",
            ))
        });
        assert!(!auth.is_authenticated_for("gitlab.com"));
    }
}
