use colored::*;

#[derive(Debug, Clone)]
pub struct SkillReport {
    pub name: String,
    pub valid: bool,
    pub issues: Vec<String>,
}

pub fn format_report(reports: &[SkillReport], valid_count: usize, invalid_count: usize) -> String {
    let skill_col_width = reports.iter().map(|r| r.name.len()).max().unwrap_or(0).max(20);
    let table_width = (skill_col_width + 1 + 10 + 1 + 6).max(60);

    let mut lines = Vec::new();
    lines.push(format!("{}", "─".repeat(table_width)));
    lines.push(format!(
        "{}",
        format!(
            "{:<width$} {:<10} {}",
            "Skill",
            "Status",
            "Issues",
            width = skill_col_width
        )
        .bold()
    ));
    lines.push(format!("{}", "─".repeat(table_width)));

    for report in reports {
        let status_text = if report.valid { "valid" } else { "invalid" };
        let status = format!("{:<10}", status_text);
        let colored_status = if report.valid {
            status.green()
        } else {
            status.red()
        };

        let issues_str = if report.issues.is_empty() {
            "-".dimmed().to_string()
        } else {
            report.issues.join(", ")
        };

        lines.push(format!(
            "{:<width$} {} {}",
            report.name,
            colored_status,
            issues_str,
            width = skill_col_width
        ));
    }

    lines.push(format!("{}", "─".repeat(table_width)));
    lines.push(format!(
        "Summary: {} valid, {} invalid",
        valid_count.to_string().green().bold(),
        invalid_count.to_string().red().bold()
    ));

    lines.join("\n") + "\n"
}

pub fn print_report(reports: &[SkillReport], valid_count: usize, invalid_count: usize) {
    print!("{}", format_report(reports, valid_count, invalid_count));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_format_matches_spec() {
        colored::control::set_override(false);

        let reports = vec![
            SkillReport {
                name: "valid_skill".to_string(),
                valid: true,
                issues: vec![],
            },
            SkillReport {
                name: "invalid_empty_name".to_string(),
                valid: false,
                issues: vec!["name is empty".to_string()],
            },
        ];

        let output = format_report(&reports, 1, 1);
        let expected = r#"────────────────────────────────────────────────────────────
Skill                Status     Issues
────────────────────────────────────────────────────────────
valid_skill          valid      -
invalid_empty_name   invalid    name is empty
────────────────────────────────────────────────────────────
Summary: 1 valid, 1 invalid
"#;
        assert_eq!(output, expected);
    }

    #[test]
    fn test_report_multiple_issues() {
        colored::control::set_override(false);

        let reports = vec![SkillReport {
            name: "bad_skill".to_string(),
            valid: false,
            issues: vec![
                "name is empty".to_string(),
                "version 'x' is not valid semver".to_string(),
            ],
        }];

        let output = format_report(&reports, 0, 1);
        assert!(output.contains("bad_skill"));
        assert!(output.contains("name is empty, version 'x' is not valid semver"));
        assert!(output.contains("Summary: 0 valid, 1 invalid"));
    }
}
