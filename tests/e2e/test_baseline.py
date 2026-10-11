import unittest
from mpd import CommandError
from support import MPDFixture, TUI, client_at, eventually


class MPDContractTests(unittest.TestCase):
    """Server contracts our forthcoming adapters must respect; not feature coverage."""
    def setUp(self):
        self.f = MPDFixture().__enter__()
        self.addCleanup(self.f.__exit__, None, None, None)
        self.c = self.f.client

    def test_save_collision_preserves_original_and_duplicates(self):
        a, b = self.f.songs[:2]
        self.c.add(a)
        self.c.add(a)
        self.c.save('Été "mix"')
        self.c.clear()
        self.c.add(b)
        with self.assertRaises(CommandError):
            self.c.save('Été "mix"')
        self.assertEqual([s['file'] for s in self.c.listplaylistinfo('Été "mix"')], [a, a])
        self.assertEqual([s['file'] for s in self.c.playlistinfo()], [b])

    def test_append_stored_playlist_preserves_current_entry_and_pause(self):
        a, b = self.f.songs[:2]
        self.c.add(b)
        self.c.save('Append sample')
        self.c.clear()
        current = self.c.addid(a)
        self.c.playid(current)
        self.c.pause(1)
        self.c.load('Append sample')
        self.assertEqual(self.c.status()['songid'], current)
        self.assertEqual(self.c.status()['state'], 'pause')
        self.assertEqual([s['file'] for s in self.c.playlistinfo()], [a, b])

    def test_recursive_directory_and_unicode_paths(self):
        self.c.add('Nested')
        tracks = self.c.playlistinfo()
        self.assertEqual(len(tracks), 1)
        self.assertIn('深い/Sp ace "quote"/03 café.flac', tracks[0]['file'])
        self.assertEqual(self.c.lsinfo('Empty folder'), [])

    def test_external_saved_playlist_change_has_distinct_idle_event(self):
        watcher = client_at(self.f.address)
        self.addCleanup(watcher.disconnect)
        self.c.add(self.f.songs[0])
        self.c.save('External edit')
        self.assertIn('stored_playlist', watcher.idle('stored_playlist'))
        self.assertEqual(watcher.listplaylists()[0]['playlist'], 'External edit')

    def test_command_list_failure_is_not_atomic(self):
        self.c.add(self.f.songs[0])
        self.c.command_list_ok_begin()
        self.c.clear()
        self.c.load('Missing playlist')
        with self.assertRaises(CommandError):
            self.c.command_list_end()
        # This is why replace-and-play must validate before clearing the user's queue.
        self.assertEqual(self.c.playlistinfo(), [])


class ZarumetEndToEndTests(unittest.TestCase):
    """Drive the actual binary via PTY and observe effects through a second MPD client."""
    def setUp(self):
        self.f = MPDFixture().__enter__()
        self.addCleanup(self.f.__exit__, None, None, None)
        self.c = self.f.client

    def open_tui(self):
        self.tui = TUI(self.f, self._testMethodName)
        self.addCleanup(self.tui.close)
        return self.tui

    def test_duplicate_removal_preserves_current_id(self):
        a = self.f.songs[0]
        first = self.c.addid(a)
        current = self.c.addid(a)
        self.c.playid(current)
        self.c.pause(1)
        ui = self.open_tui()
        ui.wait_text('Alpha fixture')
        ui.send('gg')
        ui.send('x')
        eventually(lambda: len(self.c.playlistinfo()) == 1)
        self.assertEqual(self.c.playlistinfo()[0]['id'], current)
        self.assertNotEqual(first, current)
        self.assertEqual(self.c.status()['songid'], current)
        self.assertEqual(self.c.status()['state'], 'pause')

    def test_enter_plays_selected_then_space_pauses_same_id(self):
        first = self.c.addid(self.f.songs[0])
        second = self.c.addid(self.f.songs[1])
        ui = self.open_tui()
        ui.wait_text('Beta fixture')
        ui.send('ggj\r')
        eventually(lambda: self.c.status().get('songid') == second and self.c.status()['state'] == 'play')
        ui.send(' ')
        eventually(lambda: self.c.status()['state'] == 'pause')
        self.assertEqual(self.c.status()['songid'], second)
        self.assertEqual([s['id'] for s in self.c.playlistinfo()], [first, second])

    def test_browsing_all_modes_does_not_mutate_queue(self):
        current = self.c.addid(self.f.songs[0])
        self.c.playid(current)
        self.c.pause(1)
        before = self.c.playlistinfo()
        ui = self.open_tui()
        # Labels below belong to the panels, not just the always-visible mode strip.
        for key, label in [('2', ' Artists '), ('3', ' Tracks '), ('4', ' Year '), ('5', ' Genre ')]:
            ui.send(key)
            ui.wait_text(label)
            self.assertEqual(self.c.playlistinfo(), before)
            self.assertEqual(self.c.status()['songid'], current)
            self.assertEqual(self.c.status()['state'], 'pause')

    def test_external_queue_update_appears_without_manual_refresh(self):
        ui = self.open_tui()
        self.c.add(self.f.songs[1])
        ui.wait_text('Beta fixture')
        self.assertEqual(len(self.c.playlistinfo()), 1)

    def test_clear_requires_completed_sequence(self):
        self.c.add(self.f.songs[0])
        ui = self.open_tui()
        ui.wait_text('Alpha fixture')
        ui.send('d')
        ui.wait_text('Seq:')
        self.assertEqual(len(self.c.playlistinfo()), 1)
        ui.send('d')
        eventually(lambda: not self.c.playlistinfo())


if __name__ == '__main__':
    unittest.main()
