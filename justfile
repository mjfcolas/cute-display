# Flashing, booting a slot and backing up the flash go through tools/cute_display.py;
# docs/hardware.md explains the layout.
# `just --list` shows the comment line right above each recipe.

set shell := ["bash", "-euo", "pipefail", "-c"]

app_bin := justfile_directory() / "src/firmware/target/cute-display-app.bin"
elf_dir := "src/firmware/target/xtensa-esp32s3-espidf/release"
backup_dir := justfile_directory() / "backup"
release_dir := justfile_directory() / "release"
cute_display := "uv run --quiet --script " + justfile_directory() / "tools/cute_display.py"

# list recipes
default:
    @just --list

# run the host crates' tests
test:
    cargo test --workspace
    python3 -m unittest discover --start-directory tools --quiet

# clippy over the host workspace and the firmware
lint:
    cargo clippy --workspace --all-targets
    cd src/firmware && source ~/export-esp.sh && cargo clippy --release

# render an app screen to a PNG: system, system-settings, alarm, alarm-days, weather, weather-week or radar
preview screen="system" zoom="2":
    cargo run --quiet -p ui --example app_screen -- /tmp/cute-display.fb {{screen}}
    python3 tools/fb2png.py /tmp/cute-display.fb /tmp/cute-display.png {{zoom}}
    xdg-open /tmp/cute-display.png >/dev/null 2>&1 &

# run the app image on this computer, the SD card being a directory
sim card="sim-sd":
    cargo run --quiet -p simulator -- {{card}}

# render the hardware test's report page to a PNG (`pattern` for the checkerboard)
preview-hwtest page="" zoom="2":
    cargo run --quiet -p hwtest --example report_page -- /tmp/cute-display.fb {{page}}
    python3 tools/fb2png.py /tmp/cute-display.fb /tmp/cute-display.png {{zoom}}
    xdg-open /tmp/cute-display.png >/dev/null 2>&1 &

# build + flash + monitor an image: `app` (default) or `hwtest`
fw bin="app": (fw-build bin) (fw-flash bin) (fw-monitor bin)

# build an image (release): `app` or `hwtest`
fw-build bin="app":
    cd src/firmware && source ~/export-esp.sh && cargo build --release --bin {{bin}}

# flash an image into the spare OTA slot (app1) and boot it, once the device is checked
fw-flash bin="app":
    espflash save-image --chip esp32s3 {{elf_dir}}/{{bin}} {{app_bin}}
    {{cute_display}} install {{app_bin}}

# build the app image of a release into release/, with its SHA-256, from a committed tree
release:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -n "$(git status --porcelain)" ]; then
        echo 'Commit first: a release is built from a commit, and its log says which.' >&2
        exit 1
    fi
    version=$(sed -n 's/^version = "\(.*\)"/\1/p' src/app/Cargo.toml)
    just fw-build app
    mkdir -p {{release_dir}}
    image={{release_dir}}/cute-display-$version.bin
    espflash save-image --chip esp32s3 {{elf_dir}}/app "$image"
    (cd {{release_dir}} && sha256sum "$(basename "$image")" > "$(basename "$image").sha256")
    cat "$image.sha256"
    echo "Next: tag v$version, push it, and attach both files to its GitHub release."

# attach to the serial log of an image (ctrl-C to quit)
fw-monitor bin="app":
    espflash monitor --baud 115200 --elf {{elf_dir}}/{{bin}}

# list a directory of the SD card (the root without one); close the monitor first
sd-ls dir="":
    python3 tools/sd.py ls {{dir}}

# copy a file off the SD card (to the terminal without a destination)
sd-get path dest="":
    python3 tools/sd.py get {{path}} {{dest}}

# put a local file on the SD card, somewhere under cute-display/
sd-put file path:
    python3 tools/sd.py put {{file}} {{path}}

# remove a file from the SD card, somewhere under cute-display/
sd-rm path:
    python3 tools/sd.py rm {{path}}

# put the airports within 100 km of radar.conf's place on the device (from OurAirports)
radar-airports:
    python3 tools/airports.py

# boot the firmware in one slot: app0 (Habity updated), factory (Habity as shipped) or app1 (cute-display)
fw-boot slot:
    {{cute_display}} boot {{slot}}

# what each app slot holds, which one boots, and whether the device can take cute-display
fw-check:
    {{cute_display}} check

# dump the whole 16 MB flash into backup/ (gitignored)
backup:
    {{cute_display}} backup {{backup_dir}}

# copy the whole SD card into backup/sd/, resuming where it stopped (app image, monitor closed)
backup-sd:
    python3 tools/sd.py pull "" {{backup_dir}}/sd

# copy the RTC's registers into backup/ (hardware test image)
backup-rtc:
    mkdir -p {{backup_dir}}
    python3 tools/rtc_backup.py {{backup_dir}}/ds3231-registers-$(date +%Y%m%d-%H%M%S).bin
