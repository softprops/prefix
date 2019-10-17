use crate::{
    git,
    git::{dir, Dir},
};
use colored::Colorize;
use std::{
    error::Error,
    fs::{create_dir_all, OpenOptions, Permissions},
    io,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
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
    let mut file = OpenOptions::new().create(true).write(true).open(hook)?;
    let permissions = Permissions::from_mode(0o744);
    file.set_permissions(permissions)?;
    file.write_all(script.as_bytes())?;
    Ok(())
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
