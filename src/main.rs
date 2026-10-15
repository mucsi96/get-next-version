use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::{self, Command, Output};

type Result<T> = std::result::Result<T, String>;

fn git(args: &[&str]) -> Result<Output> {
    Command::new("git")
        .args(args)
        .output()
        .map_err(|error| format!("Could not run git: {error}"))
}

fn git_checked(args: &[&str]) -> Result<String> {
    let output = git(args)?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("Invalid Git output: {error}"))
}

fn fetch_history() -> Result<()> {
    let shallow = git_checked(&["rev-parse", "--is-shallow-repository"])?;
    git_checked(&["rev-parse", "--verify", "HEAD"])?;
    // Local repositories without a remote are useful for standalone execution and tests.
    if git_checked(&["remote"])?.trim().is_empty() {
        if shallow.trim() == "true" {
            return Err("Cannot fetch full history: shallow repository has no remote".into());
        }
        return Ok(());
    }
    if shallow.trim() == "true" {
        git_checked(&["fetch", "--tags", "--unshallow"])?;
    } else {
        git_checked(&["fetch", "--tags"])?;
    }
    Ok(())
}

fn tag_version(prefix: &str, suffix: &str, tag: &str) -> Result<Option<u64>> {
    let Some(value) = tag.strip_prefix(&format!("{prefix}-")) else {
        return Ok(None);
    };
    // Configured formats also recognize legacy integer releases during migration.
    let number = if suffix.is_empty() {
        value
    } else {
        value.strip_suffix(suffix).unwrap_or(value)
    };
    if number.is_empty()
        || number.starts_with('0')
        || !number.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Ok(None);
    }
    number
        .parse()
        .map(Some)
        .map_err(|_| format!("Version in tag {tag} exceeds the supported integer range"))
}

fn get_version(prefix: &str, suffix: &str, src: &str, ignore: &str) -> Result<(u64, String, bool)> {
    let pattern = format!("{prefix}-[1-9]*");
    let tags = git_checked(&["tag", "--list", "--", &pattern])?;
    let mut latest = 0;
    let mut describe_args = vec![
        "describe".to_owned(),
        "--tags".to_owned(),
        format!("--match={pattern}"),
        "--abbrev=0".to_owned(),
    ];
    for tag in tags.lines() {
        if let Some(version) = tag_version(prefix, suffix, tag)? {
            latest = latest.max(version);
        } else {
            // Git's glob also matches tags outside the configured release formats.
            describe_args.push(format!("--exclude={tag}"));
        }
    }

    if latest > 0 {
        let args: Vec<_> = describe_args.iter().map(String::as_str).collect();
        let previous = git(&args)?;
        if previous.status.success() {
            let tag = String::from_utf8(previous.stdout)
                .map_err(|error| format!("Invalid Git output: {error}"))?;
            let tag = tag.trim();
            println!("Detecting changes in {src} since {tag}");
            let excludes: Vec<_> = ignore
                .split_whitespace()
                .map(|path| format!(":(exclude){path}"))
                .collect();
            let mut diff_args = vec!["diff", "--name-only", "HEAD", tag, "--", src];
            diff_args.extend(excludes.iter().map(String::as_str));
            let changes = git_checked(&diff_args)?;
            if changes.is_empty() {
                let version = tag_version(prefix, suffix, tag)?
                    .ok_or_else(|| format!("Invalid release tag: {tag}"))?;
                println!("No changes detected since {prefix}:{version} in {src}.");
                return Ok((version, tag.to_owned(), false));
            }
            print!("{changes}");
        } else if previous.status.code() != Some(128) {
            return Err(format!(
                "git describe failed: {}",
                String::from_utf8_lossy(&previous.stderr).trim()
            ));
        }
        // describe exits 128 when all matching release tags are on unrelated branches.
    }

    let version = latest
        .checked_add(1)
        .ok_or_else(|| "Next version exceeds the supported integer range".to_owned())?;
    println!("Changes detected for {prefix}. New version: {version}");
    Ok((version, format!("{prefix}-{version}{suffix}"), true))
}

fn run() -> Result<()> {
    let prefix = env::var("INPUT_PREFIX").unwrap_or_default();
    let prefix = prefix.trim();
    if prefix.is_empty() {
        return Err("Input required and not supplied: prefix".into());
    }
    let suffix = env::var("INPUT_SUFFIX").unwrap_or_default();
    if suffix
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_digit())
    {
        return Err(
            "Suffix must not start with a digit: release counters must remain unambiguous".into(),
        );
    }
    // Also prevents special glob characters and newlines in tags and output values.
    git_checked(&["check-ref-format", &format!("refs/tags/{prefix}-1{suffix}")])?;
    let src = env::var("INPUT_SRC").unwrap_or_default();
    let src = if src.trim().is_empty() {
        "."
    } else {
        src.trim()
    };
    if !Path::new(src).is_dir() {
        return Err(format!("Source directory does not exist: {src}"));
    }
    let ignore = env::var("INPUT_IGNORE").unwrap_or_default();
    let output_path = env::var_os("GITHUB_OUTPUT")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "GITHUB_OUTPUT must point to an output file".to_owned())?;

    fetch_history()?;
    let (version, tag, has_next_version) = get_version(prefix, &suffix, src, &ignore)?;
    let mut output = OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_path)
        .map_err(|error| format!("Could not open GITHUB_OUTPUT: {error}"))?;
    writeln!(
        output,
        "version={version}\ntag={tag}\nhasNextVersion={has_next_version}"
    )
    .map_err(|error| format!("Could not write GITHUB_OUTPUT: {error}"))
}

fn main() {
    if let Err(error) = run() {
        let escaped = error
            .replace('%', "%25")
            .replace('\r', "%0D")
            .replace('\n', "%0A");
        eprintln!("::error::{escaped}");
        process::exit(1);
    }
}
