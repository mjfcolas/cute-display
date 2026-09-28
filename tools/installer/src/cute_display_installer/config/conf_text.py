"""The device's configuration files: `key = value` lines, as
src/engine/infrastructure/src/conf_text.rs reads them."""


def parse(text):
    """Every key with its last value; lines without `=` are ignored."""
    pairs = (line.split('=', 1) for line in text.splitlines() if '=' in line)
    return {key.strip(): value.strip() for key, value in pairs}


def render(pairs):
    return ''.join(f'{key} = {value}\n' for key, value in pairs)
