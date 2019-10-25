use super::{parse_config, Action, Config};
use crate::{git, git::*};
use colored::Colorize;
use futures::future::join_all;
use std::{
    env,
    error::Error,
    fmt,
    fs::File,
    io,
    path::PathBuf,
    process::Output,
    time::{Duration, Instant},
};
use structopt::StructOpt;
use tokio::net::process::Command;

const STDIN_HOOKS: &[&str] = &["pre-push", "pre-receive", "post-receive", "post-rewrite"];

// todo: send pull to https://github.com/mitsuhiko/indicatif to add millis
struct HumanDuration(Duration);

impl fmt::Display for HumanDuration {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let t = self.0.as_millis();
        let alt = f.alternate();
        macro_rules! try_unit {
            ($secs:expr, $sg:expr, $pl:expr, $s:expr) => {
                let cnt = t / $secs;
                if cnt == 1 {
                    if alt {
                        return write!(f, "{}{}", cnt, $s);
                    } else {
                        return write!(f, "{} {}", cnt, $sg);
                    }
                } else if cnt > 1 {
                    if alt {
                        return write!(f, "{}{}", cnt, $s);
                    } else {
                        return write!(f, "{} {}", cnt, $pl);
                    }
                }
            };
        }

        try_unit!(365 * 24 * 60 * 60 * 1000, "year", "years", "y");
        try_unit!(7 * 24 * 60 * 60 * 1000, "week", "weeks", "w");
        try_unit!(24 * 60 * 60 * 1000, "day", "days", "d");
        try_unit!(60 * 60 * 1000, "hour", "hours", "h");
        try_unit!(60 * 1000, "minute", "minutes", "m");
        try_unit!(1000, "second", "seconds", "s");
        try_unit!(1, "milli", "millis", "ms");
        write!(f, "0{}", if alt { "s" } else { " seconds" })
    }
}

#[derive(StructOpt)]
pub struct Run {
    /// name of git hook to run
    ///
    /// see https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks#_client_side_hooks for a list of hooks
    hook: String,
    #[structopt(short, long)]
    config: Option<PathBuf>,
    /// any additional git args that may come after --
    #[structopt(raw(true))]
    args: Vec<String>,
}

#[derive(Debug)]
struct ActionResult {
    id: String,
    action: Action,
    output: Output,
    elapsed: Duration,
}

impl ActionResult {
    fn failed(&self) -> bool {
        self.output.status.code().iter().any(|code| *code != 0)
    }
}

async fn act(
    id: String,
    action: Action,
    paths: Vec<String>,
    instant: Instant,
) -> io::Result<ActionResult> {
    println!(
        "{} {}",
        "  › Executing".to_string().bright_green(),
        action.name.as_ref().unwrap_or(&id)
    );
    let files = paths.join(" ");
    let command = action
        .run
        .replace("{staged_files}", &files)
        .replace("{push_files}", &files)
        .replace("{files}", &files);
    Command::new("sh")
        .args(&["-c", &command])
        .output()
        .await
        .map(|output| ActionResult {
            id,
            action,
            output,
            elapsed: instant.elapsed(),
        })
}

fn paths(
    action: &Action,
    files: &Files,
) -> Vec<String> {
    let Action {
        include,
        exclude,
        run,
        ..
    } = action;
    let files = if run.contains("{staged_files}") {
        &files.staged
    } else if run.contains("{push_files}") {
        &files.push
    } else {
        &files.ls
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
) -> io::Result<Vec<io::Result<ActionResult>>> {
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
    let actions = config.remove(hook).unwrap_or_default();
    if actions.is_empty() {
        return Ok(Vec::default());
    }
    println!(
        "{}",
        format!("› Running {}", hook.to_string().bold()).bright_green()
    );
    let git_files = git::context().await?;
    Ok(join_all(actions.into_iter().filter_map(|(id, def)| {
        let action: Action = def.into();
        let files = paths(&action, &git_files);
        if files.is_empty() {
            None
        } else {
            Some(act(id, action, files, instant))
        }
    }))
    .await)
}

pub async fn run(args: Run) -> Result<(), Box<dyn Error>> {
    let Run { hook, config, args } = args;
    if let Some(Dir { top_level, .. }) = git::dir().await? {
        let mut config = parse_config(File::open(
            config.unwrap_or_else(|| top_level.join("prefix.yml")),
        )?)?;
        let start = Instant::now();
        let results = exec(&hook, &mut config, args, start).await?;
        let has_errors = results.iter().any(|result| match result {
            Err(_) => true,
            Ok(res) => res.failed(),
        });
        for result in results {
            match result {
                Ok(res) => {
                    let failed = res.failed();
                    let ActionResult {
                        id,
                        action,
                        output,
                        elapsed,
                    } = res;
                    println!(
                        "{} {} {} {}",
                        if failed { "✘".red() } else { "✔".green() },
                        if failed {
                            "failed".red()
                        } else {
                            "passed".green()
                        },
                        action.name.unwrap_or(id),
                        format!("({})", HumanDuration(elapsed)).dimmed()
                    );
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if failed && !stderr.is_empty() {
                        println!("{}", stderr);
                    }
                    if !stdout.is_empty() {
                        println!("{}", stdout);
                    }
                }
                Err(err) => eprintln!("error executing action {}", err),
            }
        }
        println!(
            "{}",
            format!(
                "› {} complete {}",
                hook,
                format!("({})", HumanDuration(start.elapsed())).dimmed()
            )
            .bright_green()
        );
        if has_errors && git::NOVERIFY_HOOKS.contains(&hook.as_str()) {
            println!("add --no-verify to bypass")
        }
    }

    Ok(())
}
