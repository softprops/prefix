use std::{io, path::Path};
use tokio::process::Command;

#[derive(Debug, PartialEq)]
pub struct Context {
    pub ls: Vec<String>,
    pub staged: Vec<String>,
    pub push: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct Dir {
    top_level: String,
    git_dir: String,
}

pub async fn dir() -> io::Result<Option<Dir>> {
    Ok(
        match &lines("git rev-parse --show-toplevel --git-common-dir").await?[..] {
            [top_level, git_dir] => Some(Dir {
                top_level: top_level.into(),
                git_dir: git_dir.into(),
            }),
            _ => None,
        },
    )
}

pub async fn context() -> io::Result<Context> {
    let (ls, staged, push) = (ls().await?, staged().await?, push().await?);
    Ok(Context { ls, staged, push })
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
            Some(Dir {
                top_level: Path::new(".").canonicalize()?.display().to_string(),
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
