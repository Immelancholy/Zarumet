"""Real MPD and real terminal support. Automated instances are disposable and silent."""
import os
from pathlib import Path
import subprocess
import tempfile
import time

from mpd import MPDClient
import pexpect
import pyte

REPO = Path(__file__).resolve().parents[2]
ARTIFACTS = Path(os.environ.get('ZARUMET_TEST_ARTIFACTS', Path(tempfile.gettempdir()) / 'zarumet-test-artifacts'))
ARTIFACTS.mkdir(parents=True, exist_ok=True)
BINARY = Path(os.environ.get('ZARUMET_BINARY', REPO / 'target/release/zarumet'))

def eventually(predicate, timeout=8):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(.04)
    raise AssertionError('Condition not met before timeout')

def mpd_config(root, music, address):
    root = Path(root)
    (root / 'playlists').mkdir(exist_ok=True)
    text = f'''music_directory "{music}"
playlist_directory "{root / 'playlists'}"
db_file "{root / 'database'}"
state_file "{root / 'state'}"
pid_file "{root / 'mpd.pid'}"
log_file "{root / 'mpd.log'}"
bind_to_address "{address}"
auto_update "no"
restore_paused "yes"
zeroconf_enabled "no"
audio_output {{
    type "null"
    name "Zarumet silent test"
    mixer_type "software"
}}
'''
    (root / 'mpd.conf').write_text(text)
    return root / 'mpd.conf'

def client_at(address):
    client = MPDClient()
    client.timeout = 3
    client.connect(str(address), 6600)
    return client

def write_cover(path, colors):
    """Generate a small, original PNG fixture without an image-library dependency."""
    import struct
    import zlib

    width = height = 96
    rows = bytearray()
    for y in range(height):
        rows.append(0)
        for x in range(width):
            rows.extend(colors[(x // 24 + y // 24) % len(colors)])

    def chunk(kind, data):
        return (struct.pack('!I', len(data)) + kind + data
                + struct.pack('!I', zlib.crc32(kind + data)))

    Path(path).write_bytes(b'\x89PNG\r\n\x1a\n'
                          + chunk(b'IHDR', struct.pack('!2I5B', width, height, 8, 2, 0, 0, 0))
                          + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b''))


def prepare_fixtures(music):
    """Small tagged silent files; no archive, network, or copyrighted assets required."""
    fixtures = [
        ('Album A/01 Alpha.flac', 'Alpha fixture', 'Fixture Artist', 'Album A', '2020', 'Electronic'),
        ('Album A/02 Beta.flac', 'Beta fixture', 'Fixture Artist', 'Album A', '2020', 'Electronic'),
        ('Nested/深い/Sp ace "quote"/03 café.flac', 'Café fixture', 'Artiste été', 'Unicode album', '2025', 'Ambient'),
        ('untagged.wav', None, None, None, None, None),
    ]
    for path, title, artist, album, date, genre in fixtures:
        dst = music / path
        dst.parent.mkdir(parents=True, exist_ok=True)
        cmd = ['ffmpeg', '-v', 'error', '-nostdin', '-n', '-f', 'lavfi', '-i',
               'anullsrc=r=44100:cl=stereo', '-t', '120']
        for key, value in [('title', title), ('artist', artist), ('album_artist', artist),
                           ('album', album), ('date', date), ('genre', genre)]:
            if value:
                cmd += ['-metadata', f'{key}={value}']
        subprocess.run(cmd + [str(dst)], check=True, stdout=subprocess.DEVNULL)
    (music / 'Empty folder').mkdir()


class MPDFixture:
    def __init__(self, restrict_control=False):
        self.restrict_control = restrict_control

    def __enter__(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='zarumet-test-')
        self.root = Path(self.tmp.name)
        self.address = self.root / 'mpd.sock'
        music = self.root / 'music'
        prepare_fixtures(music)
        config = mpd_config(self.root, music, self.address)
        if self.restrict_control:
            with config.open('a') as file:
                file.write('default_permissions "read,add"\npassword "fixture-admin@read,add,control,admin"\n')
        self.log = (self.root / 'stderr').open('w')
        self.process = subprocess.Popen(['mpd', '--no-daemon', str(config)], stdout=self.log, stderr=self.log)
        try:
            eventually(lambda: self.address.exists())
            self.client = client_at(self.address)
            if self.restrict_control:
                self.client.password('fixture-admin')
            self.client.update()
            eventually(lambda: 'updating_db' not in self.client.status())
            self.songs = sorted(song['file'] for song in self.client.listallinfo() if 'file' in song)
            assert len(self.songs) == 4, self.songs
            return self
        except BaseException:
            self.__exit__(None, None, None)
            raise

    def __exit__(self, *_):
        if hasattr(self, 'client'):
            self.client.disconnect()
        self.process.terminate()
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
        self.log.close()
        self.tmp.cleanup()

class TUI:
    def __init__(self, fixture, name, extra_config='', color=False):
        self.name = name
        self.root = fixture.root
        config = self.root / 'zarumet.toml'
        config.write_text('[pipewire]\nbit_perfect_enabled = false\n[logging]\nenabled = false\n' + extra_config)
        env = dict(os.environ, TERM='xterm-256color', COLORTERM='truecolor',
                   XDG_STATE_HOME=str(self.root / 'state-home'))
        if color:
            env.pop('NO_COLOR', None)
        self.screen = pyte.Screen(140, 42)
        self.stream = pyte.Stream(self.screen)
        self.proc = pexpect.spawn(str(BINARY), ['-c', str(config), '-a', str(fixture.address), '-b', 'off'],
                                  env=env, encoding='utf-8', codec_errors='replace', dimensions=(42, 140), timeout=8)
        self.raw = (ARTIFACTS / f'{name}.ansi').open('w')
        self.proc.logfile_read = self.raw
        try:
            self.wait_text('Queue')
        except BaseException:
            self.proc.terminate(force=True)
            self.raw.close()
            raise

    def pump(self):
        try:
            data = self.proc.read_nonblocking(65536, timeout=.05)
            self.stream.feed(data)
            # Answer terminal capability/cursor queries; no desktop window is created.
            if '\x1b[6n' in data:
                self.proc.send('\x1b[1;1R')
            if '\x1b[c' in data or '\x1b[0c' in data:
                self.proc.send('\x1b[?1;2c')
            if '\x1b[16t' in data:
                self.proc.send('\x1b[6;16;8t')
            if '\x1b[5n' in data:
                self.proc.send('\x1b[0n')
        except pexpect.TIMEOUT:
            pass
        return '\n'.join(self.screen.display)

    def wait_text(self, text):
        return eventually(lambda: text in self.pump())

    def send(self, keys):
        self.proc.send(keys)

    def close(self):
        try:
            (ARTIFACTS / f'{self.name}.screen.txt').write_text(self.pump())
            # Cancel an open modal, then quit; neither key can become name text.
            self.proc.send('\x03\x03')
            self.proc.expect(pexpect.EOF, timeout=5)
            self.proc.close()
            assert self.proc.exitstatus == 0, f'Zarumet exit: {self.proc.exitstatus}'
        finally:
            if self.proc.isalive():
                self.proc.terminate(force=True)
            self.raw.close()
