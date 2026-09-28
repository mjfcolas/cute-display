# The device's flash and SD card go through the installer, tools/installer/;
# docs/hardware.md explains the flash layout.
# `just --list` shows the comment line right above each recipe.

set shell := ["bash", "-euo", "pipefail", "-c"]

app_bin := justfile_directory() / "src/firmware/target/cute-display-app.bin"
elf_dir := "src/firmware/target/xtensa-esp32s3-espidf/release"
backup_dir := justfile_directory() / "backup"
release_dir := justfile_directory() / "release"
cute_display := "uv run --quiet --project " + justfile_directory() / "tools/installer" + " cute-display"

# list recipes
default:
    @just --list

# run the host tests: the crates, the installer, and the test images' maker
test:
    cargo test --workspace
    uv run --quiet --project tools/installer python -m unittest discover --start-directory tools/installer/tests --quiet
    uv run --quiet --project tools/installer python -m unittest discover --start-directory tools/test_flashes/tests --quiet

# clippy over the host workspace and the firmware
lint:
    cargo clippy --workspace --all-targets
    cd src/firmware && source ~/export-esp.sh && cargo clippy --release

# render an app screen to a PNG: system, system-settings, alarm, alarm-days, weather, weather-week or radar
preview screen="system" zoom="2":
    cargo run --quiet -p ui --example app_screen -- /tmp/cute-display.fb {{screen}}
    uv run --quiet --no-project python tools/fb2png.py /tmp/cute-display.fb /tmp/cute-display.png {{zoom}}
    xdg-open /tmp/cute-display.png >/dev/null 2>&1 &

# run the app image on this computer, the SD card being a directory
sim card="sim-sd":
    cargo run --quiet -p simulator -- {{card}}

# render the hardware test's report page to a PNG (`pattern` for the checkerboard)
preview-hwtest page="" zoom="2":
    cargo run --quiet -p hwtest --example report_page -- /tmp/cute-display.fb {{page}}
    uv run --quiet --no-project python tools/fb2png.py /tmp/cute-display.fb /tmp/cute-display.png {{zoom}}
    xdg-open /tmp/cute-display.png >/dev/null 2>&1 &

# build + flash + monitor an image: `app` (default) or `hwtest`
fw bin="app": (fw-build bin) (fw-flash bin) (fw-monitor bin)

# build an image (release): `app` or `hwtest`
fw-build bin="app":
    cd src/firmware && source ~/export-esp.sh && cargo build --release --bin {{bin}}

# flash an image into an OTA slot and boot it, once the device is checked
fw-flash bin="app":
    espflash save-image --chip esp32s3 {{elf_dir}}/{{bin}} {{app_bin}}
    {{cute_display}} install {{app_bin}}

# build a release into release/ from a committed tree: the app image, its SHA-256, the installer and its scripts
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
    uv build --quiet --wheel --out-dir {{release_dir}} tools/installer
    sed "s/@VERSION@/$version/g" tools/installer/install.sh > {{release_dir}}/install.sh
    sed "s/@VERSION@/$version/g; s/\r*\$/\r/" tools/installer/install.cmd > {{release_dir}}/install.cmd
    ls {{release_dir}}/*$version* {{release_dir}}/install.*
    echo "Next: tag v$version, push it, and attach these five files to its GitHub release."

# attach to the serial log of an image (ctrl-C to quit)
fw-monitor bin="app":
    espflash monitor --baud 115200 --elf {{elf_dir}}/{{bin}}

# list a directory of the SD card (the root without one); close the monitor first
sd-ls dir="":
    {{cute_display}} card ls {{dir}}

# copy a file off the SD card (to the terminal without a destination)
sd-get path dest="":
    {{cute_display}} card get {{path}} {{dest}}

# put a local file on the SD card, somewhere under cute-display/
sd-put file path:
    {{cute_display}} card put {{file}} {{path}}

# remove a file from the SD card, somewhere under cute-display/
sd-rm path:
    {{cute_display}} card rm {{path}}

# step by step: back up the device, install or update Cute Display, set it up (monitor closed)
setup:
    {{cute_display}} setup

# put the airports within 100 km of radar.conf's place on the device (from OurAirports)
radar-airports:
    {{cute_display}} radar-airports

# boot habity (its newest firmware), factory (as shipped), cute-display, or a slot: app0, app1
fw-boot target:
    {{cute_display}} boot {{target}}

# what each app slot holds, which one boots, and where Cute Display would go
fw-check:
    {{cute_display}} check

# dump the whole 16 MB flash into backup/ (gitignored)
backup:
    {{cute_display}} backup {{backup_dir}}

# make whole-flash test images into tools/test_flashes/test-bins/, one per case the installer meets, from a backup
test-bins backup bin="app":
    espflash save-image --chip esp32s3 {{elf_dir}}/{{bin}} {{app_bin}}
    uv run --quiet --project tools/installer python tools/test_flashes/make_test_flashes.py {{backup}} {{app_bin}}

# write one of tools/test_flashes/test-bins/ onto the device, the way a backup is put back
test-flash name:
    uvx --quiet --from "esptool>=5.1,<6" esptool --chip esp32s3 write-flash 0x0 tools/test_flashes/test-bins/{{name}}.bin

# run the installer on the device against each test image (`just test-bins` first); NVS goes back to the backup's
test-on-device:
    uv run --quiet --project tools/installer python tools/test_flashes/on_device.py

# copy the whole SD card into backup/sd/, resuming where it stopped (app image, monitor closed)
backup-sd:
    {{cute_display}} card pull "" {{backup_dir}}/sd

# copy the RTC's registers into backup/ (hardware test image)
backup-rtc:
    mkdir -p {{backup_dir}}
    {{cute_display}} rtc-registers {{backup_dir}}/ds3231-registers-$(date +%Y%m%d-%H%M%S).bin
