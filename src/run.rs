use super::{parse_config, Action, Config};
use crate::{git, git::*};
use colored::Colorize;
use futures::future::join_all;
use std::{
    env,
    error::Error,
    fs::File,
    io,
    process::ExitStatus,
    time::{Duration, Instant},
};
use structopt::StructOpt;
use tokio::process::Command;

const STDIN_HOOKS: &[&str] = &["pre-push", "pre-receive", "post-receive", "post-rewrite"];

#[derive(StructOpt)]
pub struct Run {
    /// name of git hook group to run
    ///
    /// see https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks#_client_side_hooks for a list of hooks
    hook: String,
    /// any additional git args that may come after --
    #[structopt(raw(true))]
    args: Vec<String>,
}

async fn act(
    action: Action,
    paths: Vec<String>,
    instant: Instant,
) -> io::Result<(Action, ExitStatus, Duration)> {
    println!(
        "{} {}",
        "  › Executing".to_string().bright_green(),
        action.name
    );
    Command::new("sh")
        .args(&["-c", &format!("{} {}", action.run, paths.join(" "))])
        .status()
        .await
        .map(|result| (action, result, instant.elapsed()))
}

fn paths(
    action: &Action,
    context: &Context,
) -> Vec<String> {
    let Action {
        include,
        exclude,
        run,
        ..
    } = action;
    let files = if run.contains("{staged_files}") {
        &context.staged
    } else if run.contains("{push_files}") {
        &context.push
    } else {
        &context.ls
    };
    files
        .iter()
        .filter_map(|f| {
            if include.iter().any(|p| !p.matches(f)) {
                return None;
            }
            if exclude.iter().any(|p| p.matches(f)) {
                return None;
            }
            Some(f.to_owned())
        })
        .collect()
}

async fn exec(
    hook: &str,
    config: &mut Config,
    args: Vec<String>,
    instant: Instant,
) -> io::Result<Vec<io::Result<(Action, ExitStatus, Duration)>>> {
    if env::var("PREFIX_SKIP").is_ok() {
        return Ok(Vec::default());
    }
    if !args.is_empty() {
        env::set_var("PREFIX_GIT_ARGS", args.join(" "));
    }
    if STDIN_HOOKS.contains(&hook) {
        use io::Read;
        let mut buf = String::new();
        io::stdin().lock().read_to_string(&mut buf)?;
        env::set_var("PREFIX_GIT_STDIN", buf);
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
        let files = paths(&action, &ctx);
        if files.is_empty() {
            None
        } else {
            Some(act(action, files, instant))
        }
    }))
    .await)
}

pub async fn run(args: Run) -> Result<(), Box<dyn Error>> {
    let Run { hook, args } = args;
    let mut config = parse_config(File::open("tests/data/config.yml")?)?;
    let start = Instant::now();
    for result in exec(&hook, &mut config, args, start).await? {
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
