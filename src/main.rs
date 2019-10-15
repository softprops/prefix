//#![feature(async_await)]
mod git;
use colored::Colorize;
use futures::future::join_all;
use git::*;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    env,
    error::Error,
    fs::File,
    io,
    process::ExitStatus,
    time::{Duration, Instant},
};
use structopt::StructOpt;
use tokio::process::Command;

#[derive(StructOpt)]
struct Run {
    /// name of git hook group to run
    ///
    /// see https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks#_client_side_hooks for a list of hooks
    hook: String,
}

#[derive(StructOpt)]
struct Install {}

#[derive(StructOpt)]
/// a managed githook runner
enum Options {
    /// Run hook group
    Run(Run),
    /// Install hook
    Install(Install),
}

#[derive(Deserialize, Debug, Clone)]
struct Action {
    /// name for display
    name: String,
    /// patterns to include
    include: Option<String>,
    /// patterns to exclude
    exclude: Option<String>,
    /// command to run (relative to git dir)
    run: String,
}

type Config = BTreeMap<String, Vec<Action>>;

fn parse_config<R>(reader: R) -> Result<Config, Box<dyn Error>>
where
    R: io::Read,
{
    Ok(serde_yaml::from_reader::<R, Config>(reader)?)
}

async fn act(
    action: Action,
    instant: Instant,
) -> io::Result<(Action, ExitStatus, Duration)> {
    println!(
        "{} {}",
        "  › Executing".to_string().bright_green(),
        action.name
    );
    Command::new("sh")
        .args(&["-c", &action.run])
        .status()
        .await
        .map(|result| (action, result, instant.elapsed()))
}

fn applies(
    _: &Action,
    _: &Context,
) -> bool {
    true
}

async fn exec(
    hook: &str,
    config: &mut Config,
    instant: Instant,
) -> io::Result<Vec<io::Result<(Action, ExitStatus, Duration)>>> {
    if env::var("PREFIX_SKIP").is_ok() {
        return Ok(Vec::default());
    }
    let group = config.remove(hook).unwrap_or_default();
    if group.is_empty() {
        return Ok(Vec::default());
    }
    println!(
        "{}",
        format!("›Running {} git hooks", hook.to_string().bold()).bright_green()
    );
    let ctx = git::context().await?;
    Ok(join_all(group.into_iter().filter_map(|action| {
        if applies(&action, &ctx) {
            Some(act(action, instant))
        } else {
            None
        }
    }))
    .await)
}

async fn run(args: Run) -> Result<(), Box<dyn Error>> {
    let Run { hook } = args;
    let mut config = parse_config(File::open("tests/data/config.yml")?)?;
    let start = Instant::now();
    for result in exec(&hook, &mut config, start).await? {
        match result {
            Ok((action, status, elapsed)) => println!(
                "complete with action {} {} in {:.2}",
                action.name,
                status.code().unwrap_or_default(),
                elapsed.as_secs_f64()
            ),
            Err(err) => eprintln!("error executing action {}", err),
        }
    }
    println!(
        "{}",
        format!(
            "›{} hooks complete in {:.2} seconds",
            hook,
            start.elapsed().as_secs_f64()
        )
        .bright_green()
    );
    Ok(())
}

async fn install(_: Install) -> Result<(), Box<dyn Error>> {
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    match Options::from_args() {
        Options::Run(args) => run(args).await?,
        Options::Install(args) => install(args).await?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hooks_are_parsable() -> Result<(), Box<dyn Error>> {
        let _ = parse_config(&include_bytes!("../tests/data/config.yml")[..])?;
        Ok(())
    }
}
