import os
import time
import unittest

from support import MPDFixture, MPDProxy, TUI, eventually


class DirectoryTests(unittest.TestCase):
    def setUp(self):
        self.f = MPDFixture().__enter__()
        self.addCleanup(self.f.__exit__, None, None, None)
        self.c = self.f.client
        self.a, self.b = self.f.songs[:2]

    def open(self, config=''):
        ui = TUI(self.f, self._testMethodName, config)
        self.addCleanup(ui.close)
        return ui

    def browse(self, ui, key='6'):
        ui.send(key)
        ui.wait_text('Directories')
        ui.wait_text('Album A/')

    def paused(self):
        current = self.c.addid(self.a)
        self.c.playid(current)
        self.c.pause(1)
        return current

    def test_browse_parent_and_root_do_not_change_queue(self):
        current = self.paused()
        ui = self.open()
        self.browse(ui)
        ui.send('\x7f\x7f\r')
        ui.wait_text('01 Alpha.flac')
        ui.send('\x7f')
        ui.wait_text('Album A/')
        ui.send('\r')
        ui.wait_text('02 Beta.flac')
        self.assertEqual([s['id'] for s in self.c.playlistinfo()], [current])
        self.assertEqual(self.c.status()['state'], 'pause')

    def test_single_track_enter_preserves_paused_playback(self):
        current = self.paused()
        ui = self.open()
        self.browse(ui)
        ui.send('\r')
        ui.wait_text('02 Beta.flac')
        ui.send('j\r')
        eventually(lambda: len(self.c.playlistinfo()) == 2)
        self.assertEqual([s['file'] for s in self.c.playlistinfo()], [self.a, self.b])
        self.assertEqual(self.c.status()['songid'], current)
        self.assertEqual(self.c.status()['state'], 'pause')

    def test_preview_focus_adds_only_selected_track_and_parent_restores_folder(self):
        current = self.paused()
        ui = self.open()
        self.browse(ui)
        ui.wait_text('02 Beta.flac')
        ui.send('lja')
        eventually(lambda: len(self.c.playlistinfo()) == 2)
        self.assertEqual([s['file'] for s in self.c.playlistinfo()], [self.a, self.b])
        ui.send('hjj')
        ui.wait_text('深い/')
        ui.send('la')
        eventually(lambda: len(self.c.playlistinfo()) == 3)
        self.assertEqual(self.c.playlistinfo()[-1]['file'], self.f.songs[2])
        ui.send('\r')
        ui.wait_text('Sp ace "quote"/')
        ui.send('\x7f')
        ui.wait_text('深い/')
        ui.send('a')
        eventually(lambda: len(self.c.playlistinfo()) == 4)
        self.assertEqual(self.c.playlistinfo()[-1]['file'], self.f.songs[2])
        self.assertEqual(self.c.status()['songid'], current)
        self.assertEqual(self.c.status()['state'], 'pause')

    def test_folder_add_recurses_and_preserves_playback(self):
        current = self.paused()
        ui = self.open()
        self.browse(ui)
        ui.send('a')
        eventually(lambda: len(self.c.playlistinfo()) == 3)
        self.assertEqual([s['file'] for s in self.c.playlistinfo()], [self.a, self.a, self.b])
        ui.send('jj')  # Empty folder, then Nested
        ui.send('a')
        eventually(lambda: len(self.c.playlistinfo()) == 4)
        self.assertEqual(self.c.playlistinfo()[-1]['file'], self.f.songs[2])
        self.assertEqual(self.c.status()['songid'], current)
        self.assertEqual(self.c.status()['state'], 'pause')

    def test_empty_folder_add_and_navigation_are_safe(self):
        current = self.paused()
        ui = self.open()
        self.browse(ui)
        ui.send('ja')
        ui.wait_text('Added: Empty folder')
        ui.send('\r')
        ui.wait_text('Empty directory')
        ui.send('a\r')
        ui.send('\x7f')
        ui.wait_text('Empty folder/')
        ui.send('\r')
        ui.wait_text('Empty directory')
        self.assertEqual([s['id'] for s in self.c.playlistinfo()], [current])

    def test_unicode_deep_paths_and_untagged_file_use_server_paths(self):
        ui = self.open()
        self.browse(ui)
        ui.send('jj\r')
        ui.wait_text('深い/')
        ui.send('\r')
        ui.wait_text('Sp ace "quote"/')
        ui.send('\r')
        ui.wait_text('03 café.flac')
        ui.send('a')
        eventually(lambda: len(self.c.playlistinfo()) == 1)
        self.assertEqual(self.c.playlistinfo()[0]['file'], self.f.songs[2])
        ui.send('\x7f\x7f\x7f')
        ui.wait_text('untagged.wav')
        ui.send('Gj')  # bottom then wrap to first
        ui.send('G\r')
        eventually(lambda: len(self.c.playlistinfo()) == 2)
        self.assertEqual(self.c.playlistinfo()[1]['file'], 'untagged.wav')
        self.assertEqual(self.c.status()['state'], 'stop')

    def test_database_update_refreshes_listing_and_preserves_selected_path(self):
        ui = self.open()
        self.browse(ui)
        ui.send('\r')
        ui.wait_text('02 Beta.flac')
        ui.send('j')
        music = self.f.root / 'music'
        os.link(music / self.a, music / 'Album A/00 Earlier.flac')
        self.c.update()
        eventually(lambda: 'updating_db' not in self.c.status())
        ui.wait_text('00 Earlier.flac')
        ui.send('a')
        eventually(lambda: len(self.c.playlistinfo()) == 1)
        self.assertEqual(self.c.playlistinfo()[0]['file'], self.b)

    def test_database_update_preserves_both_pane_selections(self):
        ui = self.open()
        self.browse(ui)
        ui.wait_text('02 Beta.flac')
        ui.send('lj')
        ui.pump()
        music = self.f.root / 'music'
        os.link(music / self.a, music / 'Album A/00 Earlier.flac')
        self.c.update()
        eventually(lambda: 'updating_db' not in self.c.status())
        ui.wait_text('00 Earlier.flac')
        ui.send('a')
        eventually(lambda: len(self.c.playlistinfo()) == 1)
        self.assertEqual(self.c.playlistinfo()[0]['file'], self.b)
        ui.send('ha')
        eventually(lambda: len(self.c.playlistinfo()) == 4)
        self.assertEqual([s['file'] for s in self.c.playlistinfo()][1:],
                         ['Album A/00 Earlier.flac', self.a, self.b])

    def test_missing_current_directory_reports_error_and_parent_recovers(self):
        ui = self.open()
        self.browse(ui)
        ui.send('j\r')
        ui.wait_text('Empty directory')
        (self.f.root / 'music/Empty folder').rmdir()
        self.c.update()
        eventually(lambda: 'updating_db' not in self.c.status())
        ui.wait_text('No such')
        ui.send('a\r')
        self.assertEqual(self.c.playlistinfo(), [])
        ui.send('\x7f')
        ui.wait_text('Album A/')

    def test_custom_and_sequential_bindings_are_scoped(self):
        ui = self.open('[binds]\nswitch_to_directories = ["8"]\nopen_directory = ["x y"]\nparent_directory = ["o"]\nadd_directory_selection = ["v v"]\n')
        self.browse(ui, '8')
        ui.send('xy')
        ui.wait_text('01 Alpha.flac')
        ui.send('vv')
        eventually(lambda: len(self.c.playlistinfo()) == 1)
        ui.send('o')
        ui.wait_text('Album A/')
        ui.send('7vv')
        ui.wait_text('Playlists')
        ui.pump()
        self.assertEqual(len(self.c.playlistinfo()), 1)

    def test_mode_cycle_includes_directories_between_genres_and_playlists(self):
        ui = self.open()
        self.browse(ui)
        # Default mode cycling is Ctrl-L / Ctrl-H.
        ui.send('\x0c')
        ui.wait_text('Playlists')
        ui.send('\x08')
        ui.wait_text('Directories')
        ui.send('\x08')
        ui.wait_text('Genres')

    def proxy_ui(self, intercept):
        proxy = MPDProxy(self.f, intercept)
        self.addCleanup(proxy.close)
        ui = TUI(proxy, self._testMethodName)
        self.addCleanup(ui.close)
        return ui

    def test_delayed_browse_allows_leaving_mode_and_fetches_only_current_level(self):
        import threading
        release = threading.Event()
        seen = []

        def intercept(line, downstream, upstream):
            if line.startswith(b'lsinfo'):
                seen.append(line)
                release.wait(timeout=5)
            return False

        ui = self.proxy_ui(intercept)
        self.addCleanup(release.set)
        ui.send('6')
        ui.wait_text('Loading…')
        eventually(lambda: len(seen) == 1)
        ui.send('1')
        ui.wait_text('Queue')
        self.assertEqual(len(seen), 1)
        release.set()
        ui.send('6')
        ui.wait_text('Album A/')
        self.assertTrue(all(line.strip() in (b'lsinfo', b'lsinfo "Album A"') for line in seen), seen)

    def test_browse_failure_displays_error_and_refresh_recovers(self):
        fail = [True]

        def intercept(line, downstream, upstream):
            if fail[0] and line.startswith(b'lsinfo'):
                downstream.sendall(b'ACK [50@0] {lsinfo} Injected browse failure\n')
                return True
            return False

        ui = self.proxy_ui(intercept)
        ui.send('6')
        ui.wait_text('Injected browse')
        ui.wait_text('failure')
        ui.send('a\r')
        self.assertEqual(self.c.playlistinfo(), [])
        fail[0] = False
        ui.send('u')
        ui.wait_text('Album A/')

    def test_preview_failure_keeps_parent_browsable_and_refresh_retries(self):
        fail = [True]

        def intercept(line, downstream, upstream):
            if fail[0] and line == b'lsinfo "Album A"':
                downstream.sendall(b'ACK [50@0] {lsinfo} Preview unavailable\n')
                return True
            return False

        ui = self.proxy_ui(intercept)
        self.browse(ui)
        ui.wait_text('Preview unavailable')
        ui.send('la')
        self.assertEqual(self.c.playlistinfo(), [])
        fail[0] = False
        ui.send('u')
        ui.wait_text('02 Beta.flac')
        ui.send('ja')
        eventually(lambda: len(self.c.playlistinfo()) == 1)
        self.assertEqual(self.c.playlistinfo()[0]['file'], self.b)

    def test_partial_folder_add_error_reconciles_queue_without_interrupting_playback(self):
        current = self.paused()

        def intercept(line, downstream, upstream):
            if line == b'idle':
                # Exclude queue events so only the explicit error refresh can reconcile.
                upstream.sendall(b'idle player mixer options database update stored_playlist\n')
                return True
            if line.startswith(b'add '):
                # Emulate a server that appends one entry before rejecting the rest.
                upstream.sendall(b'add "Album A/01 Alpha.flac"\n')
                response = b''
                while not response.endswith(b'OK\n'):
                    response += upstream.recv(4096)
                downstream.sendall(b'ACK [50@0] {add} Injected partial add\n')
                return True
            return False

        ui = self.proxy_ui(intercept)
        self.browse(ui)
        ui.send('a')
        ui.wait_text('Injected partial add')
        ui.wait_text('2 songs')
        self.assertEqual([song['file'] for song in self.c.playlistinfo()], [self.a, self.a])
        self.assertEqual(self.c.status()['songid'], current)
        self.assertEqual(self.c.status()['state'], 'pause')

    @unittest.skipUnless(os.environ.get('ZARUMET_LARGE_DIRECTORY'), 'opt-in large directory measurement')
    def test_large_directory_navigation_remains_responsive(self):
        music = self.f.root / 'music'
        big = music / 'Large'
        big.mkdir()
        for n in range(3000):
            os.link(music / self.a, big / f'{n:04}.flac')
        self.c.update()
        eventually(lambda: 'updating_db' not in self.c.status(), timeout=20)
        ui = self.open()
        self.browse(ui)
        start = time.monotonic()
        ui.send('j\r')
        ui.wait_text('0000.flac')
        ui.send('G')
        ui.wait_text('2999.flac')
        elapsed = time.monotonic() - start
        self.assertLess(elapsed, 3.0)
        print(f'3000-entry browse + jump-to-end: {elapsed:.3f}s')
