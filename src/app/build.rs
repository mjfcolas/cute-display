use std::path::Path;
use std::process::Command;

const VERSION_VARIABLE: &str = "CUTE_DISPLAY_VERSION";

/// The app image's version is the release's git tag, `v` left out: `2026.9.1` on a tagged
/// commit, `2026.9.1-3-gabc1234` after it, `-dirty` with uncommitted changes as they were
/// when this script last ran: a commit, a new tag or staging reruns it, editing alone
/// does not. `CUTE_DISPLAY_VERSION` set when building says the version instead: `just
/// release` gives the tag's, or a snapshot's.
fn main() {
    println!("cargo:rerun-if-env-changed={VERSION_VARIABLE}");
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let version = std::env::var(VERSION_VARIABLE)
        .ok()
        .filter(|version| !version.is_empty())
        .or_else(|| {
            git(&manifest, &["describe", "--tags", "--match", "v[0-9]*", "--dirty"])
                .and_then(|tag| tag.strip_prefix('v').map(str::to_owned))
        })
        .unwrap_or_else(|| "0.0.0-untagged".into());
    println!("cargo:rustc-env={VERSION_VARIABLE}={version}");

    let branch = git(&manifest, &["symbolic-ref", "-q", "HEAD"]);
    let watched = ["HEAD", "index", "packed-refs", "refs/tags"].into_iter().map(str::to_owned).chain(branch);
    for path in watched.filter_map(|name| git(&manifest, &["rev-parse", "--path-format=absolute", "--git-path", &name])) {
        // A path that does not exist would rerun this script, and rebuild the crate, on every build.
        if Path::new(&path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
    }
}

fn git(directory: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new("git").current_dir(directory).args(arguments).output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
