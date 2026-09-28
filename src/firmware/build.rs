use std::process::Command;

fn main() {
    embuild::espidf::sysenv::output();

    println!("cargo:rustc-env=BUILD_TIME={}Z", run("date", &["-u", "+%m-%d %H:%M"]));
    // A file that never exists: reruns this script on every build, so the stamp is fresh.
    println!("cargo:rerun-if-changed=.force-build-stamp");
}

fn run(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}
