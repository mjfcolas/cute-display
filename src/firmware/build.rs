use std::process::Command;

fn main() {
    embuild::espidf::sysenv::output();
    // Set by the line above, and read by esp-idf-sys's `esp_app_desc!` in the images.
    println!(
        "cargo::rustc-check-cfg=cfg(esp_idf_app_compile_time_date, esp_idf_app_reproducible_build, \
         esp_idf_version_at_least_5_3_2, esp_idf_version_at_least_5_4_0, esp_idf_version_patch_at_least_5_1_7, \
         esp_idf_version_patch_at_least_5_2_3, esp_idf_version_patch_at_least_5_3_2)"
    );

    println!("cargo:rustc-env=BUILD_GIT={}", run("git", &["describe", "--always", "--dirty"]));
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
