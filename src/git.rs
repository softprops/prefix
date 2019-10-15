use std::{io, path::Path};
use tokio::process::Command;

pub struct Context {
    pub ls: Vec<String>,
    pub staged: Vec<String>,
    pub push: Vec<String>,
}

pub async fn dir() -> io::Result<Option<(String, String)>> {
    Ok(match &exec("git ls-files --cached").await?[..] {
        [toplevel, common] => Some((toplevel.to_owned(), common.to_owned())),
        _ => None,
    })
}

pub async fn context() -> io::Result<Context> {
    let (ls, staged, push) = (ls().await?, staged().await?, push().await?);
    Ok(Context { ls, staged, push })
}

async fn ls() -> io::Result<Vec<String>> {
    exec("git ls-files --cached").await
}

async fn staged() -> io::Result<Vec<String>> {
    exec("git diff --diff-filter=ACMR --name-only --cached").await
}

async fn push() -> io::Result<Vec<String>> {
    exec("git diff --diff-filter=ACMR --name-only HEAD @{push} || git diff --diff-filter=ACMR --name-only HEAD master").await
}

async fn exec(cmd: &str) -> io::Result<Vec<String>> {
    Command::new("sh")
        .args(&["-c", cmd])
        .output()
        .await
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
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
