use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::exit;
use std::process::Output;

static VERSIONS: [(&'static str, &'static str); 53] = [
    ("2015-05-15", "1.0.0"),
    ("2015-06-25", "1.1.0"),
    ("2015-08-07", "1.2.0"),
    ("2015-09-17", "1.3.0"),
    ("2015-10-29", "1.4.0"),
    ("2015-12-10", "1.5.0"),
    ("2016-01-21", "1.6.0"),
    ("2016-03-03", "1.7.0"),
    ("2016-04-14", "1.8.0"),
    ("2016-05-26", "1.9.0"),
    ("2016-07-07", "1.10.0"),
    ("2016-08-18", "1.11.0"),
    ("2016-09-29", "1.12.0"),
    ("2016-10-20", "1.12.1"),
    ("2016-11-10", "1.13.0"),
    ("2016-12-22", "1.14.0"),
    ("2017-02-02", "1.15.0"),
    ("2017-02-09", "1.15.1"),
    ("2017-03-16", "1.16.0"),
    ("2017-04-27", "1.17.0"),
    ("2017-06-08", "1.18.0"),
    ("2017-07-20", "1.19.0"),
    ("2017-08-31", "1.20.0"),
    ("2017-10-12", "1.21.0"),
    ("2017-11-22", "1.22.0"),
    ("2017-11-22", "1.22.1"),
    ("2018-01-04", "1.23.0"),
    ("2018-02-15", "1.24.0"),
    ("2018-03-01", "1.24.1"),
    ("2018-03-29", "1.25.0"),
    ("2018-05-10", "1.26.0"),
    ("2018-05-29", "1.26.1"),
    ("2018-06-05", "1.26.2"),
    ("2018-06-21", "1.27.0"),
    ("2018-07-10", "1.27.1"),
    ("2018-07-20", "1.27.2"),
    ("2018-08-02", "1.28.0"),
    ("2018-09-13", "1.29.0"),
    ("2018-09-25", "1.29.1"),
    ("2018-10-11", "1.29.2"),
    ("2018-10-25", "1.30.0"),
    ("2018-11-08", "1.30.1"),
    ("2018-12-06", "1.31.0"),
    ("2018-12-20", "1.31.1"),
    ("2019-01-17", "1.32.0"),
    ("2019-02-28", "1.33.0"),
    ("2019-04-11", "1.34.0"),
    ("2019-04-25", "1.34.1"),
    ("2019-05-14", "1.34.2"),
    ("2019-05-23", "1.35.0"),
    ("2019-07-04", "1.36.0"),
    ("2019-08-15", "1.37.0"),
    ("2019-09-20", "1.38.0"),
];

fn git<S>(git: &Path, args: &[S]) -> Output where S: AsRef<OsStr>
{
    let mut cmd = Command::new("git");
    cmd.arg("--git-dir");
    cmd.arg(git);
    cmd.args(args);
    cmd.output().unwrap()
}

fn check(output: Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        println!("{}", stdout);
        println!("{}", String::from_utf8_lossy(&output.stderr));
        exit(1);
    }
    stdout.trim().to_string()
}

#[allow(deprecated)]
fn get_cargo_home() -> PathBuf {
    env::var_os("CARGO_HOME").map(PathBuf::from).or_else(|| env::home_dir().map(|d| d.join(".cargo"))).expect("$CARGO_HOME not set")
}

fn get_cargo_manifest_dir() -> PathBuf {
    if let Some(dir) = env::var_os("CARGO_MANIFEST_DIR") {
        return PathBuf::from(dir);
    }
    let mut root_dir = env::current_dir().expect("cwd");
    {
        let tmp = root_dir.clone();
        let mut tmp = tmp.as_path();
        while let Some(new_tmp) = tmp.parent() {
            if new_tmp.join("Cargo.toml").exists() {
                root_dir = new_tmp.to_owned();
            }
            tmp = new_tmp;
        }
    }
    root_dir
}

fn get_cutoff_date(arg: Option<&str>) -> (String, String) {
    if let Some(arg) = arg {
        let arg_dot = format!("{}.", arg);
        if arg.starts_with("20") && arg.contains('-') {
            for &(date, ver) in VERSIONS.iter() {
                if date >= &arg {
                    return (arg.to_owned(), ver.to_owned());
                }
            }
            return (arg.to_owned(), "<date>".into());
        }
        if arg.contains('.') {
            for &(date, ver) in VERSIONS.iter() {
                if ver == arg || ver.starts_with(&arg_dot) {
                    return (date.to_owned(), ver.to_owned());
                }
            }
        }
    }
    let ver_str = check(Command::new("rustc").arg("--version").output().unwrap());
    let arg = ver_str.splitn(3, ' ').skip(1).next().expect("rustc version ???");
    let arg = arg.splitn(2, '-').next().unwrap();
    for &(date, ver) in VERSIONS.iter() {
        if ver == arg {
            return (date.to_owned(), ver.to_owned());
        }
    }
    println!("Specify Rust version (1.x.y) or ISO date (YYYY-MM-DD) as an argument");
    exit(1);
}

fn main() {
    let arg = env::args().skip(1).filter(|a| a != "lts" && !a.starts_with('-')).next();
    let arg = arg.as_ref().map(|s| s.as_str());
    let prefetch_only = arg == Some("prefetch");

    let home = get_cargo_home();
    let (cutoff, rust_vers) = get_cutoff_date(arg);

    let git_dir = env::var_os("CARGO_REGISTRY_GIT_DIR").map(PathBuf::from).unwrap_or_else(|| home.join("registry/index/github.com-1ecc6299db9ec823/.git"));

    if !git_dir.exists() {
        println!("{} doesn't exist. Set CARGO_REGISTRY_GIT_DIR to cargo index .git dir", git_dir.display());
        exit(1);
    }

    if !git(&git_dir, &["rev-parse", "snapshot-2018-09-26", "--"]).status.success() {
        check(git(&git_dir, &["fetch", "https://github.com/rust-lang/crates.io-index", "snapshot-2018-09-26:snapshot-2018-09-26"]));
    }

    if prefetch_only {
        return;
    }

    let root = get_cargo_manifest_dir();
    let last_commit_hash = check(git(&git_dir, &["log", "--all", "-1", "--format=%H", "--until", &cutoff]));

    let treeish = format!("{}^{{tree}}", last_commit_hash);
    let msg = format!("Registry at {}", cutoff);
    // create a new commit that is a snapshot of that commit
    let new_head = check(git(&git_dir, &["commit-tree", &treeish, "-m", &msg]));

    let fork_name = format!("lts-repo-at-{}", last_commit_hash);

    // git requires exposing a commit as a ref in order to clone it
    if !git(&git_dir, &["branch", &fork_name, &new_head]).status.success() {
        let refname = format!("refs/heads/{}", fork_name);
        check(git(&git_dir, &["update-ref", &refname, &new_head]));
    }

    // make a new repo with just that commit
    let cargo_local_dir = root.join(".cargo");
    let _ = fs::create_dir(&cargo_local_dir);

    let fork_repo_git_dir = cargo_local_dir.join(&fork_name);
    let _ = fs::remove_dir_all(&fork_repo_git_dir); // just in case

    check(Command::new("git").args(&["clone", "--single-branch", "--bare", "--branch", &fork_name]).arg(&git_dir).arg(&fork_repo_git_dir).output().unwrap());

    // do fixups, so that cargo can find proper dir
    check(git(&fork_repo_git_dir, &["update-ref", "HEAD", &new_head]));
    check(git(&fork_repo_git_dir, &["branch", "master", &new_head]));

    let config_path = cargo_local_dir.join("config");
    let mut config_toml = String::new();

    if config_path.exists() {
        let f = BufReader::new(File::open(&config_path).expect("can't read .cargo/config"));
        let mut skipping = false;
        for line in f.lines() {
            let line = line.unwrap();
            if line.starts_with('[') || line.starts_with("# delete this") {
                skipping = line.starts_with("[source.crates-io]")
                    || line.starts_with("# delete this")
                    || line.starts_with("[source.lts-repo-");
            }

            if !skipping {
                config_toml.push_str(&line);
                config_toml.push('\n');
            }
        }
    }

    let fork_repo_abs = fs::canonicalize(&fork_repo_git_dir).unwrap();

    config_toml.push_str(&format!("# delete this to restore to the default registry
[source.crates-io]
replace-with = 'lts-repo-replacement'

[source.lts-repo-replacement] # {cutoff}
registry = 'file://{path}'
", cutoff = cutoff, path = fork_repo_abs.to_str().unwrap()));

    let mut out = File::create(&config_path).expect("Writing .cargo/config");
    out.write_all(config_toml.as_bytes()).unwrap();

    println!("Set {} to use registry state from {} ({})", config_path.display(), cutoff, rust_vers);
}
