"""The alarm's ringtones: the MP3 files in its directory, as
src/apps/alarm/src/infrastructure/sound_ringer.rs lists them. Habity's own are on the
card too, and may be copied there."""
DIRECTORY = 'cute-display/apps/alarm/ringtones'
HABITY_DIRECTORY = 'sounds/alarm'


def is_ringtone(name):
    return name.lower().endswith('.mp3')


def copies(names):
    """Each of Habity's ringtones named, and where its copy goes."""
    return {f'{HABITY_DIRECTORY}/{name}': f'{DIRECTORY}/{name}' for name in names}
