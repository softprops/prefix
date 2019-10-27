use crate::{
    git,
    git::{dir, Dir},
};
use colored::Colorize;
use std::{
    error::Error,
    fs::{create_dir_all, File},
    io::{self, Write},
    path::{Path, PathBuf},
};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Install {}

fn add_hook(
    hook: PathBuf,
    script: &str,
) -> io::Result<()> {
    println!(
        "creating hook {}",
        hook.display().to_string().bright_green()
    );
    create_file(hook)?.write_all(script.as_bytes())?;
    Ok(())
}

fn create_file<P>(hook: P) -> io::Result<File>
where
    P: AsRef<Path>,
{
    #[cfg(target_family = "unix")]
    {
        use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt};
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o755)
            .open(hook)
    }
    #[cfg(not(target_family = "unix"))]
    {
        File::create(hook)
    }
}

pub async fn install(_: Install) -> Result<(), Box<dyn Error>> {
    if let Some(Dir { git_dir, .. }) = dir().await? {
        let hooks_dir = git_dir.join("hooks");
        if !hooks_dir.exists() {
            create_dir_all(&hooks_dir)?;
        }
        let data = script();
        for hook in git::HOOKS.iter().map(|hook| hooks_dir.join(hook)) {
            add_hook(hook, &data)?;
        }
    }
    Ok(())
}

fn script() -> String {
    r#"
#!/bin/sh
hook_name=$(basename "$0")
git_args="$*"
echo "hook_name '$hook_name' git args '$git_args'"
"#
    .into()
}
