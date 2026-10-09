# The recipes for the device's flash and SD card use the installer, tools/installer/.
# docs/hardware.md explains the flash layout.
# `just --list` shows the comment line right above each recipe.

set shell := ["bash", "-euo", "pipefail", "-c"]

app_bin := justfile_directory() / "src/firmware/target/cute-display-app.bin"
elf_dir := "src/firmware/target/xtensa-esp32s3-espidf/release"
backup_dir := justfile_directory() / "backup"
release_dir := justfile_directory() / "release"
cute_display := "uv run --quiet --project " + justfile_directory() / "tools/installer" + " cute-display"
sim_console := justfile_directory() / "target/simulator.sock"

# list recipes
default:
    @just --list

# run the host tests: the crates, the installer and its link, the end-to-end harness, the test image maker, the release version script
test:
    cargo test --workspace
    uv run --quiet --project tools/installer python -m unittest discover --start-directory tools/installer/tests --quiet
    uv run --quiet --project tools/link python -m unittest discover --start-directory tools/link/tests --quiet
    uv run --quiet --project tools/e2e python -m unittest discover --start-directory tools/e2e/tests --quiet
    uv run --quiet --project tools/installer python -m unittest discover --start-directory tools/test_flashes/tests --quiet
    python3 -m unittest discover --start-directory tools/release/tests --quiet

# clippy over the host workspace and the firmware
lint:
    cargo clippy --workspace --all-targets
    cd src/firmware && source ~/export-esp.sh && cargo clippy --release

# draw an app screen into a PNG; the screens are the ones `main` names in src/simulator/examples/app_screen.rs
preview screen="system" zoom="2":
    cargo run --quiet -p simulator --example app_screen -- /tmp/cute-display.fb {{screen}}
    uv run --quiet --project tools/link python -m cute_display_link.frame /tmp/cute-display.fb /tmp/cute-display.png {{zoom}}
    xdg-open /tmp/cute-display.png >/dev/null 2>&1 &

# run the app image on this computer, with a directory as the SD card and a console for `just remote-sim`; flags in docs/simulator/README.md
sim *args:
    cargo run --quiet -p simulator -- --console {{sim_console}} {{args}}

# the same on the simulator started by `just sim`
remote-sim *args:
    uv run --quiet --project {{justfile_directory()}}/tools/link remote --simulator {{sim_console}} {{args}}

# the end-to-end scenarios on `simulator`, each on its own simulator, or the ones marked device on `device`: builds and flashes the test image, plays them with the stubs on this computer, then flashes the app image again whatever the result; takes pytest's arguments, such as `-k alarm`
e2e on *args:
    #!/usr/bin/env bash
    set -uo pipefail
    case {{on}} in
        simulator)
            exec uv run --quiet --project tools/e2e pytest tools/e2e/scenarios {{args}} ;;
        device)
            just fw-build e2e && just fw-build app && just fw-flash e2e || exit
            trap 'just fw-flash app' EXIT
            uv run --quiet --project tools/e2e pytest tools/e2e/scenarios --device {{args}} ;;
        *)
            echo "just e2e simulator or just e2e device, not {{on}}" >&2
            exit 2 ;;
    esac

# draw the hardware test's report page into a PNG (`pattern` for the checkerboard)
preview-hwtest page="" zoom="2":
    cargo run --quiet -p hwtest --example report_page -- /tmp/cute-display.fb {{page}}
    uv run --quiet --project tools/link python -m cute_display_link.frame /tmp/cute-display.fb /tmp/cute-display.png {{zoom}}
    xdg-open /tmp/cute-display.png >/dev/null 2>&1 &

# build, flash and monitor an image: `app` (default) or `hwtest`
fw bin="app": (fw-build bin) (fw-flash bin) (fw-monitor bin)

# build an image (release): `app` or `hwtest`
fw-build bin="app":
    cd src/firmware && source ~/export-esp.sh && cargo build --release --bin {{bin}}

# check the device, then flash an image into an OTA slot and start it
fw-flash bin="app":
    espflash save-image --chip esp32s3 {{elf_dir}}/{{bin}} {{app_bin}}
    {{cute_display}} install {{app_bin}}

# build a release into release/ from a tagged commit: the app image, its SHA-256, the installer and its scripts
release:
    #!/usr/bin/env bash
    set -euo pipefail
    version=$(git describe --tags --exact-match --match 'v[0-9]*' 2>/dev/null | sed 's/^v//' || true)
    wheel_version=$version
    if [ -z "$version" ]; then
        next=$(sh tools/release/next_version.sh)
        version=$next-snapshot wheel_version=$next.dev0
        echo "No tag on this commit: snapshot $version, to try out, not to publish." >&2
    elif ! echo "$version" | grep -Eq '^[0-9]{4}\.[1-9][0-9]?\.(0|[1-9][0-9]*)$'; then
        echo "The tag v$version is not v<year>.<month>.<n> with no leading zero." >&2
        exit 1
    elif grep -qx '## Unreleased' CHANGELOG.md UPDATING.md; then
        echo "CHANGELOG.md or UPDATING.md still says Unreleased, not $version." >&2
        exit 1
    elif [ -n "$(git status --porcelain)" ]; then
        echo "Uncommitted changes go into the release, which still says $version." >&2
    fi
    # Set here, because otherwise the builds would give git describe's -<n>-g<hash> or -dirty.
    export CUTE_DISPLAY_VERSION=$version SETUPTOOLS_SCM_PRETEND_VERSION=$wheel_version
    just fw-build app
    mkdir -p {{release_dir}}
    image={{release_dir}}/cute-display-$version.bin
    espflash save-image --chip esp32s3 {{elf_dir}}/app "$image"
    (cd {{release_dir}} && sha256sum "$(basename "$image")" > "$(basename "$image").sha256")
    cat "$image.sha256"
    cp UPDATING.md {{release_dir}}/
    if ! grep -Eqx '[0-9]{4}\.[1-9][0-9]?\.(0|[1-9][0-9]*)' tools/release/oldest-installer.txt; then
        echo "tools/release/oldest-installer.txt is not one <year>.<month>.<n>." >&2
        exit 1
    fi
    cp tools/release/oldest-installer.txt {{release_dir}}/
    uv build --quiet --wheel --out-dir {{release_dir}} tools/installer
    wheel=cute_display_installer-$wheel_version-py3-none-any.whl
    if [ "$version" = "$wheel_version" ]; then
        wheel=https://github.com/mjfcolas/cute-display/releases/download/v$version/$wheel
    else
        wheel=$(cd {{release_dir}} && pwd)/$wheel
    fi
    sed "s|@WHEEL@|$wheel|g" tools/installer/install.sh > {{release_dir}}/install.sh
    sed "s|@WHEEL@|$wheel|g; s/\r*\$/\r/" tools/installer/install.cmd > {{release_dir}}/install.cmd
    ls {{release_dir}}/*$version* {{release_dir}}/*$wheel_version* {{release_dir}}/install.* {{release_dir}}/UPDATING.md {{release_dir}}/oldest-installer.txt
    if [ "$version" = "$wheel_version" ]; then
        echo "Next: push v$version, and attach these seven files to its GitHub release."
    else
        echo "To try it: sh {{release_dir}}/install.sh, which installs this snapshot's installer and runs its setup."
    fi

# attach to the serial log of an image (ctrl-C to quit)
fw-monitor bin="app":
    espflash monitor --baud 115200 --elf {{elf_dir}}/{{bin}}

# list a directory of the SD card (the root if none is given); close the monitor first
sd-ls dir="":
    {{cute_display}} card ls {{dir}}

# copy a file from the SD card (to the terminal if no destination is given)
sd-get path dest="":
    {{cute_display}} card get {{path}} {{dest}}

# put a local file on the SD card, somewhere under cute-display/
sd-put file path:
    {{cute_display}} card put {{file}} {{path}}

# remove a file from the SD card, somewhere under cute-display/
sd-rm path:
    {{cute_display}} card rm {{path}}

# the device's controls and what it does: tap yellow, hold 1500 yellow long, turn -2, lights, sound, clock (monitor closed)
remote *args:
    uv run --quiet --project {{justfile_directory()}}/tools/link remote {{args}}

# measure how fast a put, the console, a card read through the console and a copy on the card are (monitor closed)
bench *args:
    uv run --quiet --project {{justfile_directory()}}/tools/link bench {{args}}

# step by step: back up the device, install or update Cute Display, set it up (monitor closed)
setup:
    {{cute_display}} setup

# write the airports within 100 km of general.conf's place to the device (from OurAirports)
radar-airports:
    {{cute_display}} radar-airports

# start habity (its newest firmware), factory (the shipped firmware), cute-display, or a slot: app0, app1
fw-boot target:
    {{cute_display}} boot {{target}}

# show what each app slot contains, which one starts, and where Cute Display would go
fw-check:
    {{cute_display}} check

# copy the whole 16 MB flash into backup/ (ignored by git)
backup:
    {{cute_display}} backup {{backup_dir}}

# make whole-flash test images in tools/test_flashes/test-bins/ from a backup, one for each case the installer can meet
test-bins backup bin="app":
    espflash save-image --chip esp32s3 {{elf_dir}}/{{bin}} {{app_bin}}
    uv run --quiet --project tools/installer python tools/test_flashes/make_test_flashes.py {{backup}} {{app_bin}}

# write one of tools/test_flashes/test-bins/ onto the device, the way a backup is put back
test-flash name:
    uvx --quiet --from "esptool>=5.1,<6" esptool --chip esp32s3 write-flash 0x0 tools/test_flashes/test-bins/{{name}}.bin

# run the installer on the device with each test image (`just test-bins` first); NVS is reset to the backup's
test-on-device:
    uv run --quiet --project tools/installer python tools/test_flashes/on_device.py

# copy the whole SD card into backup/sd/, continuing where it stopped (app image, monitor closed)
backup-sd:
    {{cute_display}} card pull "" {{backup_dir}}/sd

# copy the RTC's registers into backup/ (hardware test image)
backup-rtc:
    mkdir -p {{backup_dir}}
    {{cute_display}} rtc-registers {{backup_dir}}/ds3231-registers-$(date +%Y%m%d-%H%M%S).bin
