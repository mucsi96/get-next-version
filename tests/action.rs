use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

struct Repository {
    root: PathBuf,
    checkout: PathBuf,
}

impl Repository {
    fn new() -> Self {
        let root = env::temp_dir().join(format!(
            "get-next-version-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let checkout = root.join("checkout with spaces");
        fs::create_dir_all(&checkout).unwrap();
        let repo = Self { root, checkout };
        repo.git(&["init", "-b", "main"]);
        repo.write("server/source.txt", "server release\n");
        repo.write("client/source.txt", "client release\n");
        repo.commit("Initial source");
        repo
    }

    fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(["-c", "commit.gpgsign=false", "-c", "tag.gpgsign=false"])
            .args(args)
            .current_dir(&self.checkout)
            .env("GIT_AUTHOR_NAME", "Version action test")
            .env("GIT_AUTHOR_EMAIL", "version-test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Version action test")
            .env("GIT_COMMITTER_EMAIL", "version-test@example.invalid")
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?}: {output:?}");
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn write(&self, path: &str, content: &str) {
        let path = self.checkout.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn commit(&self, message: &str) {
        self.git(&["add", "."]);
        self.git(&["commit", "-m", message]);
    }

    fn invoke(&self, prefix: &str, src: &str, ignore: &str) -> (Output, String) {
        self.invoke_with_suffix(prefix, "", src, ignore)
    }

    fn invoke_with_suffix(
        &self,
        prefix: &str,
        suffix: &str,
        src: &str,
        ignore: &str,
    ) -> (Output, String) {
        let output_file = self.root.join("github-output");
        fs::write(&output_file, "existing=value\n").unwrap();
        let binary = env::var_os("ACTION_BINARY")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_get-next-version")));
        let output = Command::new(binary)
            .current_dir(&self.checkout)
            .env("INPUT_PREFIX", prefix)
            .env("INPUT_SUFFIX", suffix)
            .env("INPUT_SRC", src)
            .env("INPUT_IGNORE", ignore)
            .env("GITHUB_OUTPUT", &output_file)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .unwrap();
        (output, fs::read_to_string(output_file).unwrap())
    }

    fn expect_version(&self, prefix: &str, src: &str, ignore: &str, version: u64, next: bool) {
        let (output, values) = self.invoke(prefix, src, ignore);
        self.assert_version(
            output,
            values,
            version,
            &format!("{prefix}-{version}"),
            next,
        );
    }

    fn expect_suffixed_version(&self, suffix: &str, version: u64, tag: &str, next: bool) {
        let (output, values) = self.invoke_with_suffix("server", suffix, "server", "");
        self.assert_version(output, values, version, tag, next);
    }

    fn assert_version(&self, output: Output, values: String, version: u64, tag: &str, next: bool) {
        assert!(output.status.success(), "Action failed: {output:?}");
        let actual: BTreeMap<_, _> = values
            .lines()
            .map(|line| line.split_once('=').unwrap())
            .collect();
        let version = version.to_string();
        let next = next.to_string();
        assert_eq!(
            actual,
            BTreeMap::from([
                ("existing", "value"),
                ("version", version.as_str()),
                ("tag", tag),
                ("hasNextVersion", next.as_str()),
            ])
        );
    }

    fn expect_failure(&self, prefix: &str, src: &str, expected: &str) {
        let (output, values) = self.invoke(prefix, src, "");
        assert!(!output.status.success(), "Action unexpectedly succeeded");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expected), "Unexpected error: {stderr}");
        assert_eq!(
            values, "existing=value\n",
            "Failures must not publish outputs"
        );
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        // Windows Git creates read-only objects, so let Git's shell remove the fixture.
        #[cfg(windows)]
        let _ = Command::new("bash")
            .args(["-c", "rm -rf -- \"$1\"", "cleanup"])
            .arg(self.root.to_string_lossy().replace('\\', "/"))
            .status();
        #[cfg(not(windows))]
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn starts_at_one_without_release_tags_and_defaults_source() {
    let repo = Repository::new();
    repo.git(&["tag", "other-100"]);
    repo.expect_version("server", "", "", 1, true);
}

#[test]
fn unchanged_components_keep_their_current_versions() {
    let repo = Repository::new();
    repo.git(&["tag", "server-42"]);
    repo.git(&["tag", "client-17"]);
    repo.write("README.md", "Workflow-only maintenance\n");
    repo.commit("Documentation change");
    repo.expect_version("server", "server", "", 42, false);
    repo.expect_version("client", "client", "", 17, false);
}

#[test]
fn changed_server_increments_without_releasing_client() {
    let repo = Repository::new();
    repo.git(&["tag", "server-42"]);
    repo.git(&["tag", "client-17"]);
    repo.write("server/source.txt", "updated server\n");
    repo.commit("Server change");
    repo.expect_version("server", "server", "", 43, true);
    repo.expect_version("client", "client", "", 17, false);
}

#[test]
fn uses_reachable_release_when_unchanged_and_global_maximum_when_changed() {
    let repo = Repository::new();
    repo.git(&["tag", "server-42"]);
    repo.git(&["switch", "-c", "unrelated-release"]);
    repo.write("server/source.txt", "unrelated release\n");
    repo.commit("Unrelated release");
    repo.git(&["tag", "server-99"]);
    repo.git(&["switch", "main"]);
    repo.expect_version("server", "server", "", 42, false);
    repo.write("server/source.txt", "main server change\n");
    repo.commit("Main server change");
    repo.expect_version("server", "server", "", 100, true);
}

#[test]
fn uses_global_numbering_even_without_a_reachable_tag() {
    let repo = Repository::new();
    repo.git(&["switch", "-c", "unrelated-release"]);
    repo.write("server/source.txt", "other branch\n");
    repo.commit("Other branch");
    repo.git(&["tag", "server-10"]);
    repo.git(&["switch", "main"]);
    repo.expect_version("server", "server", "", 11, true);
}

#[test]
fn ignores_multiple_repository_relative_paths_but_not_actual_source_changes() {
    let repo = Repository::new();
    repo.git(&["tag", "server-42"]);
    repo.write("server/generated/one.txt", "generated\n");
    repo.write("server/cache/two.txt", "cached\n");
    repo.commit("Generated files");
    let ignore = "server/generated  server/cache";
    repo.expect_version("server", "server", ignore, 42, false);
    repo.write("server/source.txt", "real source change\n");
    repo.commit("Source change");
    repo.expect_version("server", "server", ignore, 43, true);
}

#[test]
fn handles_literal_prefixes_and_source_paths_with_spaces() {
    let repo = Repository::new();
    repo.write("source with spaces/main.txt", "initial\n");
    repo.commit("Additional component");
    repo.git(&["tag", "app.v-9"]);
    repo.git(&["tag", "appXv-100"]);
    repo.expect_version("app.v", "source with spaces", "", 9, false);
    repo.write("source with spaces/main.txt", "changed\n");
    repo.commit("Update spaced path");
    repo.expect_version("app.v", "source with spaces", "", 10, true);
}

#[test]
fn ignores_tags_outside_the_documented_positive_integer_format() {
    let repo = Repository::new();
    repo.git(&["tag", "server-9"]);
    repo.write("server/source.txt", "new source\n");
    repo.commit("Change source");
    for tag in [
        "server-99-beta",
        "server-0",
        "server-01",
        "server-100-extra",
    ] {
        repo.git(&["tag", tag]);
    }
    repo.expect_version("server", "server", "", 10, true);
}

#[test]
fn fetches_missing_tags_and_unshallows_checkout() {
    let mut repo = Repository::new();
    repo.git(&["tag", "server-42"]);
    repo.write("README.md", "docs only\n");
    repo.commit("Documentation change");
    let clone = repo.root.join("shallow checkout");
    let path = repo.checkout.to_string_lossy().replace('\\', "/");
    let url = if cfg!(windows) {
        format!("file:///{path}")
    } else {
        format!("file://{path}")
    };
    repo.git(&[
        "clone",
        "--depth=1",
        "--no-tags",
        &url,
        clone.to_str().unwrap(),
    ]);
    repo.checkout = clone;
    assert_eq!(repo.git(&["rev-parse", "--is-shallow-repository"]), "true");
    assert_eq!(repo.git(&["tag", "--list"]), "");
    repo.expect_version("server", "server", "", 42, false);
    assert_eq!(repo.git(&["rev-parse", "--is-shallow-repository"]), "false");
    // A complete checkout must fetch without attempting --unshallow again.
    repo.expect_version("server", "server", "", 42, false);
}

#[test]
fn invalid_inputs_fail_without_writing_outputs() {
    let repo = Repository::new();
    repo.expect_failure("", ".", "Input required");
    repo.expect_failure("bad*prefix", ".", "check-ref-format");
    repo.expect_failure("server", "missing", "Source directory does not exist");
}

#[test]
fn git_failures_do_not_report_an_unchanged_version() {
    let mut repo = Repository::new();
    repo.git(&[
        "remote",
        "add",
        "origin",
        repo.root.join("missing.git").to_str().unwrap(),
    ]);
    repo.expect_failure("server", "server", "fetch --tags failed");
    repo.checkout = repo.root.join("not a git repository");
    fs::create_dir_all(repo.checkout.join("server")).unwrap();
    repo.expect_failure("server", "server", "rev-parse");
}

#[test]
fn refuses_version_overflow() {
    let repo = Repository::new();
    repo.git(&["tag", "server-18446744073709551615"]);
    repo.write("server/source.txt", "changed\n");
    repo.commit("Source change");
    repo.expect_failure("server", "server", "Next version exceeds");
}

#[test]
fn starts_a_new_chart_with_a_suffixed_tag_and_integer_counter() {
    let repo = Repository::new();
    repo.expect_suffixed_version(".0.0", 1, "server-1.0.0", true);
}

#[test]
fn increments_published_chart_versions_instead_of_reusing_legacy_numbers() {
    let repo = Repository::new();
    repo.git(&["tag", "server-14"]);
    repo.write("server/source.txt", "published chart source\n");
    repo.commit("Published chart");
    repo.git(&["tag", "-a", "server-22.0.0", "-m", "Published chart 22"]);
    repo.expect_suffixed_version(".0.0", 22, "server-22.0.0", false);
    repo.write("server/source.txt", "new chart source\n");
    repo.commit("Chart change");
    repo.expect_suffixed_version(".0.0", 23, "server-23.0.0", true);
}

#[test]
fn unchanged_legacy_release_keeps_its_actual_existing_tag() {
    let repo = Repository::new();
    repo.git(&["tag", "server-14"]);
    repo.expect_suffixed_version(".0.0", 14, "server-14", false);
    repo.write("server/source.txt", "changed source\n");
    repo.commit("Migrate release format");
    repo.expect_suffixed_version(".0.0", 15, "server-15.0.0", true);
}

#[test]
fn default_integer_mode_does_not_change_when_suffixed_tags_exist() {
    let repo = Repository::new();
    repo.git(&["tag", "server-14"]);
    repo.git(&["tag", "server-22.0.0"]);
    repo.expect_version("server", "server", "", 14, false);
    repo.write("server/source.txt", "changed integer-mode source\n");
    repo.commit("Integer component change");
    repo.expect_version("server", "server", "", 15, true);
}

#[test]
fn suffix_mode_uses_reachable_release_and_global_maximum_across_both_formats() {
    let repo = Repository::new();
    repo.git(&["tag", "server-22.0.0"]);
    repo.git(&["switch", "-c", "other-release"]);
    repo.write("server/source.txt", "unrelated chart\n");
    repo.commit("Other release branch");
    repo.git(&["tag", "server-99.0.0"]);
    repo.git(&["tag", "server-100"]);
    repo.git(&["switch", "main"]);
    repo.expect_suffixed_version(".0.0", 22, "server-22.0.0", false);
    repo.write("server/source.txt", "main chart change\n");
    repo.commit("Main chart change");
    repo.expect_suffixed_version(".0.0", 101, "server-101.0.0", true);
}

#[test]
fn suffix_mode_ignores_other_formats_prereleases_and_leading_zeroes() {
    let repo = Repository::new();
    repo.git(&["tag", "server-22.0.0"]);
    repo.write("server/source.txt", "changed chart\n");
    repo.commit("Chart change");
    for tag in [
        "server-99.0.0-beta",
        "server-88.1.0",
        "server-77.0.0.0.0",
        "server-066.0.0",
        "server-0.0.0",
        "other-100.0.0",
    ] {
        repo.git(&["tag", tag]);
    }
    repo.expect_suffixed_version(".0.0", 23, "server-23.0.0", true);
}

#[test]
fn suffix_is_literal_and_not_a_regular_expression() {
    let repo = Repository::new();
    repo.git(&["tag", "server-22.0.0"]);
    repo.git(&["tag", "server-99x0x0"]);
    repo.expect_suffixed_version(".0.0", 22, "server-22.0.0", false);
}

#[test]
fn suffix_mode_preserves_source_ignore_rules() {
    let repo = Repository::new();
    repo.git(&["tag", "server-22.0.0"]);
    repo.write("server/generated/output.txt", "generated\n");
    repo.commit("Generated change");
    let (output, values) = repo.invoke_with_suffix("server", ".0.0", "server", "server/generated");
    repo.assert_version(output, values, 22, "server-22.0.0", false);
}

#[test]
fn fetches_suffixed_tags_from_a_shallow_checkout() {
    let mut repo = Repository::new();
    repo.git(&["tag", "server-22.0.0"]);
    repo.write("README.md", "documentation-only change\n");
    repo.commit("Documentation change");
    let clone = repo.root.join("shallow suffix checkout");
    let path = repo.checkout.to_string_lossy().replace('\\', "/");
    let url = if cfg!(windows) {
        format!("file:///{path}")
    } else {
        format!("file://{path}")
    };
    repo.git(&[
        "clone",
        "--depth=1",
        "--no-tags",
        &url,
        clone.to_str().unwrap(),
    ]);
    repo.checkout = clone;
    assert_eq!(repo.git(&["tag", "--list"]), "");
    repo.expect_suffixed_version(".0.0", 22, "server-22.0.0", false);
    assert_eq!(repo.git(&["rev-parse", "--is-shallow-repository"]), "false");
}

#[test]
fn invalid_suffixes_fail_without_publishing_outputs() {
    let repo = Repository::new();
    for suffix in ["0.0", "*", ".0.0\nforged=value", "..0"] {
        let (output, values) = repo.invoke_with_suffix("server", suffix, "server", "");
        assert!(!output.status.success());
        assert_eq!(values, "existing=value\n");
    }
}

#[test]
fn suffix_mode_refuses_counter_overflow() {
    let repo = Repository::new();
    repo.git(&["tag", "server-18446744073709551615.0.0"]);
    repo.write("server/source.txt", "changed chart\n");
    repo.commit("Chart change");
    let (output, values) = repo.invoke_with_suffix("server", ".0.0", "server", "");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Next version exceeds"));
    assert_eq!(values, "existing=value\n");
}
