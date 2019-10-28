use crate::{
    git,
    git::{dir, Dir},
};
use colored::Colorize;
use std::{
    error::Error,
    fs::{copy, create_dir_all, File},
    io::{self, Write},
    path::{Path, PathBuf},
};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Install {
    #[structopt(short, long)]
    /// When encounting a hook script of the same name, force override it
    force: bool,
}

fn add_hook(
    hook: PathBuf,
    script: &str,
    force: bool,
) -> io::Result<()> {
    if !force && hook.exists() {
        println!("warning: a hook script with the name {} already exists. Saving it with a .bak extension. To override these files, pass the --force flag", hook.display());
        copy(&hook, hook.with_extension(".bak"))?;
    }
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

pub async fn install(args: Install) -> Result<(), Box<dyn Error>> {
    let Install { force } = args;
    if let Some(Dir { git_dir, .. }) = dir().await? {
        let hooks_dir = git_dir.join("hooks");
        if !hooks_dir.exists() {
            create_dir_all(&hooks_dir)?;
        }
        let data = script();
        for hook in git::HOOKS.iter().map(|hook| hooks_dir.join(hook)) {
            add_hook(hook, &data, force)?;
        }
    }
    Ok(())
}

fn script() -> String {
    r#"
#!/bin/sh
hook_name=$(basename "$0")
git_args="$*"
cargo +nightly run -q -- run $hook_name -- $git_args
"#
    .into()
}
