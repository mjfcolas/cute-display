# Flashing writes app1, then otadata to boot it; docs/hardware.md explains the layout.
# `just --list` shows the comment line right above each recipe.

set shell := ["bash", "-euo", "pipefail", "-c"]

app_bin := justfile_directory() / "src/firmware/target/cute-display-app.bin"
elf_dir := "src/firmware/target/xtensa-esp32s3-espidf/release"
app1 := "0x820000"
otadata := "0x19000"
backup_dir := justfile_directory() / "backup"
stock_otadata := backup_dir / "otadata-stock.bin"

# list recipes
default:
    @just --list

# run the host crates' tests
test:
    cargo test --workspace

# clippy over the host workspace and the firmware
lint:
    cargo clippy --workspace --all-targets
    cd src/firmware && source ~/export-esp.sh && cargo clippy --release

# render an app screen to a PNG: system, weather, weather-week or radar
preview screen="system" zoom="2":
    cargo run --quiet -p ui --example app_screen -- /tmp/cute-display.fb {{screen}}
    python3 tools/fb2png.py /tmp/cute-display.fb /tmp/cute-display.png {{zoom}}
    xdg-open /tmp/cute-display.png >/dev/null 2>&1 &

# render the hardware test's report page to a PNG (`pattern` for the checkerboard)
preview-hwtest page="" zoom="2":
    cargo run --quiet -p hwtest --example report_page -- /tmp/cute-display.fb {{page}}
    python3 tools/fb2png.py /tmp/cute-display.fb /tmp/cute-display.png {{zoom}}
    xdg-open /tmp/cute-display.png >/dev/null 2>&1 &

# build + flash + monitor an image: `app` (default) or `hwtest`
fw bin="app": (fw-build bin) (fw-flash bin "monitor")

# build an image (release): `app` or `hwtest`
fw-build bin="app":
    cd src/firmware && source ~/export-esp.sh && cargo build --release --bin {{bin}}

# flash an image into the spare OTA slot (app1) and boot it
fw-flash bin="app" monitor="": _check-partitions _save-otadata
    espflash save-image --chip esp32s3 {{elf_dir}}/{{bin}} {{app_bin}}
    espflash write-bin --baud 921600 {{app1}} {{app_bin}}
    just _otadata {{bin}} "{{monitor}}"

# attach to the serial log of an image (ctrl-C to quit)
fw-monitor bin="app":
    espflash monitor --baud 115200 --elf {{elf_dir}}/{{bin}}

# Refuses a partition table other than the expected one.
_check-partitions:
    #!/usr/bin/env python3
    import os, struct, subprocess, sys, tempfile
    path = os.path.join(tempfile.gettempdir(), 'cute-display-ptable.bin')
    subprocess.run(['espflash', 'read-flash', '0x8000', '0xc00', path], check=True,
                   stdout=subprocess.DEVNULL)
    table = open(path, 'rb').read()
    parts = {}
    for i in range(0, len(table), 32):
        e = table[i:i + 32]
        if e[:2] != b'\xaa\x50':
            break
        ptype, sub, off, size = e[2], e[3], *struct.unpack_from('<II', e, 4)
        label = e[12:28].split(b'\0')[0].decode(errors='replace')
        parts[(ptype, sub)] = (label, off, size)
        print(f'  {label:<10} type {ptype} sub 0x{sub:02x}  0x{off:06x}  {size // 1024:>5} KB')
    want = {(1, 0x00): ('otadata', int('{{otadata}}', 16)),
            (0, 0x11): ('ota_1', int('{{app1}}', 16))}
    for key, (name, off) in want.items():
        got = parts.get(key)
        if not got or got[1] != off:
            sys.exit(f'partition table mismatch: expected {name} at 0x{off:x}, found {got}.\n'
                     'Refusing to flash — this is not the layout the recipes were written for.')
    print('partition table OK: otadata and ota_1 are where the recipes expect them')

# The first time only, and never a copy that already points at app1.
_save-otadata:
    #!/usr/bin/env python3
    import os, struct, subprocess, sys
    dst = '{{stock_otadata}}'
    if os.path.exists(dst):
        print(f'stock otadata already saved at {dst}')
        sys.exit(0)
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    tmp = dst + '.tmp'
    subprocess.run(['espflash', 'read-flash', '{{otadata}}', '0x2000', tmp], check=True,
                   stdout=subprocess.DEVNULL)
    data = open(tmp, 'rb').read()
    seqs = [struct.unpack_from('<I', data, off)[0] for off in (0, 0x1000)]
    live = [s for s in seqs if s != 0xFFFFFFFF]
    if live and (max(live) - 1) % 2 == 1:
        os.remove(tmp)
        sys.exit('otadata already points at app1 but no stock copy was saved — refusing to\n'
                 'save a pointer at ourselves as "stock". Recover it from a `just backup` dump.')
    os.rename(tmp, dst)
    print(f'saved the stock otadata to {dst} ({"factory" if not live else f"ota_{(max(live) - 1) % 2}"})')

# ota_seq 2 selects ota_1. Written last so --monitor catches the boot from its start.
_otadata bin monitor="":
    #!/usr/bin/env python3
    import struct, zlib, subprocess, tempfile, os
    seq, state = 2, 2  # (seq-1) % 2 == 1 -> ota_1 ; ESP_OTA_IMG_VALID
    entry = (struct.pack('<I', seq) + b'\xff' * 20
             + struct.pack('<II', state, zlib.crc32(struct.pack('<I', seq), 0xFFFFFFFF)))
    path = os.path.join(tempfile.gettempdir(), 'cute-display-otadata.bin')
    with open(path, 'wb') as f:
        f.write(entry + b'\xff' * (0x1000 - len(entry)) + b'\xff' * 0x1000)
    cmd = ['espflash', 'write-bin', '--baud', '921600', '{{otadata}}', path]
    if '{{monitor}}':
        cmd += ['--after', 'hard-reset', '--monitor', '--monitor-baud', '115200',
                '--elf', '{{elf_dir}}/{{bin}}']
        # espflash exits non-zero when the monitor is closed.
        rc = subprocess.run(cmd).returncode
        if rc:
            print(f'(espflash exited {rc} — that is the monitor closing, not the flash)')
    else:
        subprocess.run(cmd, check=True)

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

# boot the stock Habity firmware again (writes the saved stock otadata back)
fw-stock:
    #!/usr/bin/env python3
    import os, subprocess, sys
    src = '{{stock_otadata}}'
    if not os.path.exists(src):
        sys.exit(f'{src} is missing, so there is no record of which stock app was booting.\n'
                 'Recover otadata (0x19000, 8 KB) from a `just backup` dump.')
    subprocess.run(['espflash', 'write-bin', '--baud', '921600', '{{otadata}}', src], check=True)
    subprocess.run(['espflash', 'reset'], check=True)
    print('stock otadata written back: the bootloader is on the stock app again.')

# check which app the bootloader is running
fw-which:
    #!/usr/bin/env python3
    import struct, subprocess, tempfile, os
    path = os.path.join(tempfile.gettempdir(), 'cute-display-otadata-read.bin')
    subprocess.run(['espflash', 'read-flash', '{{otadata}}', '32', path], check=True,
                   stdout=subprocess.DEVNULL)
    s, = struct.unpack_from('<I', open(path, 'rb').read())
    print('otadata blank -> booting factory (stock Habity)' if s == 0xFFFFFFFF
          else f'ota_seq={s} -> booting ota_{(s - 1) % 2} '
               f'({"cute-display" if (s - 1) % 2 == 1 else "stock Habity"})')

# dump the whole 16 MB flash into backup/ (gitignored)
backup:
    mkdir -p {{backup_dir}}
    espflash read-flash --baud 921600 0x0 0x1000000 {{backup_dir}}/habity-full-16MB-$(date +%Y%m%d-%H%M%S).bin
    sha256sum {{backup_dir}}/*.bin

# copy the whole SD card into backup/sd/, resuming where it stopped (app image, monitor closed)
backup-sd:
    python3 tools/sd.py pull "" {{backup_dir}}/sd

# copy the RTC's registers into backup/ (hardware test image)
backup-rtc:
    mkdir -p {{backup_dir}}
    python3 tools/rtc_backup.py {{backup_dir}}/ds3231-registers-$(date +%Y%m%d-%H%M%S).bin
