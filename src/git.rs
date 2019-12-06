use path_slash::PathBufExt;
use std::{
    io,
    path::{Path, PathBuf},
};
use tokio::process::Command;

/// hooks that can be by-passed
pub const NOVERIFY_HOOKS: &[&str] = &["commit-msg", "pre-commit", "pre-rebase", "pre-push"];

/// client-side hooks
pub const HOOKS: &[&str] = &[
    "applypatch-msg",
    "pre-applypatch",
    "post-applypatch",
    "pre-commit",
    "prepare-commit-msg",
    "commit-msg",
    "post-commit",
    "pre-rebase",
    "post-checkout",
    "post-merge",
    "pre-push",
    "pre-receive",
    "update",
    "post-receive",
    "post-update",
    "push-to-checkout",
    "pre-auto-gc",
    "post-rewrite",
    "sendemail-validate",
];

#[derive(Debug, PartialEq)]
pub struct Files {
    pub ls: Vec<String>,
    pub staged: Vec<String>,
    pub push: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct Dir {
    pub top_level: PathBuf,
    pub git_dir: PathBuf,
}

pub async fn dir() -> io::Result<Option<Dir>> {
    Ok(
        match &lines("git rev-parse --show-toplevel --git-common-dir").await?[..] {
            [top_level, git_dir] => Some(Dir {
                top_level: PathBuf::from_slash(top_level),
                git_dir: PathBuf::from_slash(git_dir),
            }),
            _ => None,
        },
    )
}

pub async fn context() -> io::Result<Files> {
    let (ls, staged, push) = (ls().await?, staged().await?, push().await?);
    Ok(Files { ls, staged, push })
}

async fn ls() -> io::Result<Vec<String>> {
    files("git ls-files --cached").await
}

async fn staged() -> io::Result<Vec<String>> {
    files("git diff --diff-filter=ACMR --name-only --cached").await
}

async fn push() -> io::Result<Vec<String>> {
    files("git diff --diff-filter=ACMR --name-only HEAD @{push} || git diff --diff-filter=ACMR --name-only HEAD master").await
}

async fn lines(cmd: &str) -> io::Result<Vec<String>> {
    Command::new("sh")
        .args(&["-c", cmd])
        .output()
        .await
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(std::convert::Into::into)
                .collect()
        })
}

async fn files(cmd: &str) -> io::Result<Vec<String>> {
    lines(cmd).await.map(|lines| {
        lines
            .iter()
            .filter_map(|line| {
                if Path::new(line).is_file() {
                    Some(line.into())
                } else {
                    None
                }
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{error::Error, path::Path};

    #[tokio::test]
    async fn dir_provides_paths() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            dir().await?,
            // canonicalize returns a "UNC" path on windows
            // https://github.com/rust-lang/rust/issues/42869
            Some(Dir {
                top_level: Path::new(".")
                    .canonicalize()
                    .map(|path| { path.to_string_lossy().replace(r"\\?\", "").into() })?,
                git_dir: ".git".into()
            })
        );
        Ok(())
    }

    #[tokio::test]
    async fn context_profiles_files() {
        assert!(context().await.is_ok())
    }
}
