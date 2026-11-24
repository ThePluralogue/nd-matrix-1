//! Link to the project's GitHub Sponsors page, shown as a title-bar pill.

/// ASSUMED: the Sponsors page of the repository owner (`ymatrix1`, the only account named in
/// `.agents/rules/git-workflow.md`); confirm by opening the URL once Sponsors is enabled.
pub const SPONSOR_URL: &str = "https://github.com/sponsors/ymatrix1";

/// Title-bar pill text.
pub const SPONSOR_LABEL: &str = "SPONSOR";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sponsor_url_points_at_github_sponsors() {
        assert!(SPONSOR_URL.starts_with("https://github.com/sponsors/"));
        assert_eq!(SPONSOR_LABEL, "SPONSOR");
    }
}
