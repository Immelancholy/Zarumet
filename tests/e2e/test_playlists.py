import unittest

from support import MPDFixture, TUI, eventually, write_cover


class PlaylistTests(unittest.TestCase):
    def setUp(self):
        self.f = MPDFixture(restrict_control=self._testMethodName in ('test_partial_replacement_error_reconciles_queue', 'test_manage_permission_errors_preserve_playlist_and_queue')).__enter__()
        self.addCleanup(self.f.__exit__, None, None, None)
        self.c = self.f.client
        self.a, self.b = self.f.songs[:2]

    def playlist(self, name='A mix', songs=None):
        self.c.clear()
        for song in songs if songs is not None else [self.b, self.b]:
            self.c.add(song)
        self.c.save(name)
        self.c.clear()

    def paused_queue(self):
        current = self.c.addid(self.a)
        self.c.playid(current)
        self.c.pause(1)
        return current

    def open(self, extra_config='', color=False):
        ui = TUI(self.f, self._testMethodName, extra_config, color=color)
        self.addCleanup(ui.close)
        return ui

    def browse(self, ui, count=2, key='7'):
        ui.send(key)
        ui.wait_text(f'Tracks ({count})')
        if count:
            ui.wait_text('Beta fixture')

    def test_browse_and_append_preserve_paused_current_id_and_duplicates(self):
        self.playlist()
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui)
        ui.send('ljkh')
        self.assertEqual([song['id'] for song in self.c.playlistinfo()], [current])
        ui.send('a')
        eventually(lambda: len(self.c.playlistinfo()) == 3)
        self.assertEqual([song['file'] for song in self.c.playlistinfo()], [self.a, self.b, self.b])
        self.assertEqual(self.c.status()['songid'], current)
        self.assertEqual(self.c.status()['state'], 'pause')

    def test_replace_requires_confirmation_then_plays_first_playlist_entry(self):
        self.playlist()
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui)
        ui.send('P')
        ui.wait_text('Replace queue and play?')
        self.assertEqual([song['id'] for song in self.c.playlistinfo()], [current])
        ui.send('\x1b')
        ui.send('P')
        ui.wait_text('Replace queue and play?')
        ui.send('\r')
        eventually(lambda: self.c.status()['state'] == 'play' and self.c.status().get('songid') != current)
        queue = self.c.playlistinfo()
        self.assertEqual([song['file'] for song in queue], [self.b, self.b])
        self.assertEqual(self.c.status()['songid'], queue[0]['id'])

    def test_empty_playlist_does_not_clear_queue(self):
        self.playlist(songs=[])
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui, 0)
        ui.wait_text('Empty playlist')
        ui.send('P')
        ui.wait_text('Replace queue and play?')
        ui.send('\r')
        ui.wait_text('queue unchanged')
        self.assertEqual([song['id'] for song in self.c.playlistinfo()], [current])
        self.assertEqual(self.c.status()['state'], 'pause')

    def test_deleted_playlist_between_confirmation_and_play_preserves_queue(self):
        self.playlist()
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui)
        ui.send('P')
        ui.wait_text('Replace queue and play?')
        self.c.rm('A mix')
        ui.send('\r')
        ui.wait_text('No such playlist')
        self.assertEqual([song['id'] for song in self.c.playlistinfo()], [current])

    def test_invalid_track_rolls_back_staged_entries_without_losing_current_id(self):
        (self.f.root / 'playlists/Broken.m3u').write_text(self.b + '\nmissing-file.flac\n')
        current = self.paused_queue()
        ui = self.open()
        ui.send('7')
        ui.wait_text('Broken')
        ui.send('P')
        ui.wait_text('Replace queue and play?')
        ui.send('\r')
        ui.wait_text('Staged entries removed')
        self.assertEqual([song['id'] for song in self.c.playlistinfo()], [current])
        self.assertEqual(self.c.status()['state'], 'pause')

    def test_save_unicode_name_from_queue_consumes_shortcuts_and_preserves_queue(self):
        current = self.paused_queue()
        ui = self.open()
        ui.send('S')
        ui.wait_text('Save queue as playlist')
        name = 'Été "q d d r z"'
        ui.send(name + '\r')
        eventually(lambda: any(p['playlist'] == name for p in self.c.listplaylists()))
        self.assertEqual([song['file'] for song in self.c.listplaylistinfo(name)], [self.a])
        self.assertEqual([song['id'] for song in self.c.playlistinfo()], [current])
        self.assertEqual(self.c.status()['random'], '0')
        self.assertEqual(self.c.status()['repeat'], '0')

    def test_save_collision_created_while_typing_is_not_overwritten(self):
        self.c.add(self.a)
        ui = self.open()
        ui.send('S')
        ui.wait_text('Save queue as playlist')
        ui.send('Collision')
        self.c.save('Collision')
        self.c.add(self.b)
        ui.send('\r')
        ui.wait_text('already exists')
        self.assertEqual([song['file'] for song in self.c.listplaylistinfo('Collision')], [self.a])
        ui.send('\x1b')

    def test_save_empty_queue_in_playlists_and_cancel_second_prompt(self):
        ui = self.open()
        ui.send('7')
        ui.wait_text('No stored playlists')
        ui.send('S')
        ui.wait_text('Save queue as playlist')
        ui.send('\r')
        ui.wait_text('Enter a playlist name')
        ui.send('Empty saved\r')
        eventually(lambda: any(p['playlist'] == 'Empty saved' for p in self.c.listplaylists()))
        self.assertEqual(self.c.listplaylistinfo('Empty saved'), [])
        ui.send('S')
        ui.wait_text('Save queue as playlist')
        ui.send('Cancelled\x1b')
        ui.send('1')
        ui.wait_text(' Queue ')
        self.assertFalse(any(p['playlist'] == 'Cancelled' for p in self.c.listplaylists()))

    def test_external_playlist_content_and_deletion_refresh_without_queue_change(self):
        self.playlist(songs=[self.b])
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui, 1)
        self.c.playlistadd('A mix', self.b)
        ui.wait_text('Tracks (2)')
        self.c.rm('A mix')
        ui.wait_text('No stored playlists')
        self.assertEqual([song['id'] for song in self.c.playlistinfo()], [current])

    def test_configurable_mode_and_sequential_add_binding(self):
        self.playlist()
        current = self.paused_queue()
        ui = self.open('[binds]\nswitch_to_playlists = ["8"]\nadd_playlist_to_queue = ["v v"]\n')
        ui.send('vv')
        self.browse(ui, key='8')
        self.assertEqual(len(self.c.playlistinfo()), 1)
        ui.send('vv')
        eventually(lambda: len(self.c.playlistinfo()) == 3)
        self.assertEqual(self.c.status()['songid'], current)

    def test_partial_replacement_error_reconciles_queue(self):
        self.playlist()
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui)
        ui.send('P')
        ui.wait_text('Replace queue and play?')
        ui.send('\r')
        ui.wait_text('Replacement incomplete')
        ui.wait_text('3 songs')
        self.assertEqual(len(self.c.playlistinfo()), 3)
        self.assertEqual(self.c.status()['songid'], current)
        self.assertEqual(self.c.status()['state'], 'pause')

    def test_external_insert_preserves_selected_playlist_name(self):
        self.playlist('B selected', [self.b])
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui, 1)
        self.c.save('A inserted')
        ui.wait_text('A inserted')
        ui.send('a')
        eventually(lambda: len(self.c.playlistinfo()) == 2)
        self.assertEqual([song['file'] for song in self.c.playlistinfo()], [self.a, self.b])
        self.assertEqual(self.c.status()['songid'], current)

    def test_long_unicode_name_and_small_terminal_remain_usable(self):
        name = 'Playlist été ' * 12
        self.playlist(name, [self.b])
        ui = self.open()
        self.browse(ui, 1)
        ui.screen.resize(20, 70)
        ui.proc.setwinsize(20, 70)
        ui.send('S')
        ui.wait_text('Save queue as playlist')
        ui.send('A new name\x1b')
        ui.send('a')
        eventually(lambda: len(self.c.playlistinfo()) == 1)
        self.assertEqual(self.c.playlistinfo()[0]['file'], self.b)

    def assert_paused_queue(self, current):
        self.assertEqual([song['id'] for song in self.c.playlistinfo()], [current])
        self.assertEqual(self.c.status()['songid'], current)
        self.assertEqual(self.c.status()['state'], 'pause')

    def test_rename_preserves_contents_queue_and_selection(self):
        self.playlist()
        self.playlist('B other', [self.a])
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui)
        ui.send('R')
        ui.wait_text('╭ Rename playlist')
        name = 'Été "q r z"'
        ui.send('\x15' + name + '\r')
        eventually(lambda: name in [p['playlist'] for p in self.c.listplaylists()])
        self.assertNotIn('A mix', [p['playlist'] for p in self.c.listplaylists()])
        self.assertEqual([song['file'] for song in self.c.listplaylistinfo(name)], [self.b, self.b])
        self.assert_paused_queue(current)
        ui.wait_text('Tracks (2)')
        ui.send('a')
        eventually(lambda: len(self.c.playlistinfo()) == 3)
        self.assertEqual([song['file'] for song in self.c.playlistinfo()], [self.a, self.b, self.b])
        self.assertEqual(self.c.status()['random'], '0')
        self.assertEqual(self.c.status()['repeat'], '0')

    def test_rename_collision_created_while_typing_preserves_both(self):
        self.playlist()
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui)
        ui.send('R')
        ui.wait_text('╭ Rename playlist')
        ui.send('\x15Collision')
        self.c.save('Collision')
        ui.send('\r')
        ui.wait_text('Playlist exists already')
        self.assertEqual([s['file'] for s in self.c.listplaylistinfo('A mix')], [self.b, self.b])
        self.assertEqual([s['file'] for s in self.c.listplaylistinfo('Collision')], [self.a])
        self.assert_paused_queue(current)
        ui.send('\x1b')

    def test_rename_validation_cancel_and_missing_source(self):
        self.playlist()
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui)
        ui.send('R')
        ui.wait_text('╭ Rename playlist')
        ui.send('\x15\r')
        ui.wait_text('Enter a playlist name')
        ui.send('bad/name\r')
        ui.wait_text('Names cannot contain slashes')
        ui.send('\x1b')
        self.assertEqual([p['playlist'] for p in self.c.listplaylists()], ['A mix'])
        ui.send('R')
        ui.wait_text('╭ Rename playlist')
        ui.send('\x15Renamed')
        self.c.rm('A mix')
        ui.send('\r')
        ui.wait_text('No such playlist')
        self.assertEqual(self.c.listplaylists(), [])
        self.assert_paused_queue(current)

    def test_delete_confirm_cancel_and_preserve_queue(self):
        self.playlist()
        self.playlist('B remaining', [self.a])
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui)
        ui.send('D')
        ui.wait_text('Delete playlist?')
        ui.wait_text('Delete "A mix"?')
        self.assertEqual(len(self.c.listplaylists()), 2)
        ui.send('\x1b')
        self.assertEqual(len(self.c.listplaylists()), 2)
        ui.send('D')
        ui.wait_text('Delete playlist?')
        ui.send('\r')
        eventually(lambda: [p['playlist'] for p in self.c.listplaylists()] == ['B remaining'])
        ui.wait_text('Tracks (1)')
        self.assert_paused_queue(current)
        self.assertEqual(sorted(s['file'] for s in self.c.listallinfo() if 'file' in s), self.f.songs)

    def test_delete_last_playlist_shows_empty_browser(self):
        self.playlist()
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui)
        ui.send('D')
        ui.wait_text('Delete playlist?')
        ui.send('\r')
        ui.wait_text('No stored playlists')
        self.assertEqual(self.c.listplaylists(), [])
        self.assert_paused_queue(current)
        ui.send('RD')
        ui.send('S')
        ui.wait_text('Save queue as playlist')
        ui.send('\x1b')

    def test_delete_target_stays_named_when_another_client_changes_selection(self):
        self.playlist()
        self.playlist('B remaining', [self.a])
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui)
        ui.send('D')
        ui.wait_text('Delete "A mix"?')
        self.c.rm('A mix')
        ui.send('\r')
        ui.wait_text('No such playlist')
        self.assertEqual([p['playlist'] for p in self.c.listplaylists()], ['B remaining'])
        self.assert_paused_queue(current)
        ui.send('\x1b')
        ui.wait_text('Tracks (1)')

    def test_configured_manage_bindings_are_scoped_to_playlists(self):
        self.playlist()
        current = self.paused_queue()
        ui = self.open('[binds]\nrename_playlist = ["v v"]\ndelete_playlist = ["w w"]\n')
        ui.send('vvww')
        self.assert_paused_queue(current)
        self.browse(ui)
        ui.send('vv')
        ui.wait_text('╭ Rename playlist')
        ui.send('\x1b')
        ui.send('ww')
        ui.wait_text('Delete playlist?')
        ui.send('\x1b')
        self.assertEqual([p['playlist'] for p in self.c.listplaylists()], ['A mix'])

    def test_manage_permission_errors_preserve_playlist_and_queue(self):
        self.playlist()
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui)
        ui.send('R')
        ui.wait_text('╭ Rename playlist')
        ui.send(' new\r')
        ui.wait_text('permission')
        ui.send('\x1b')
        ui.send('D')
        ui.wait_text('Delete playlist?')
        ui.send('\r')
        ui.wait_text('permission')
        self.assertEqual([s['file'] for s in self.c.listplaylistinfo('A mix')], [self.b, self.b])
        self.assert_paused_queue(current)

    def test_track_panel_adds_only_selected_entry_and_preserves_pause(self):
        self.playlist(songs=[self.a, self.b, self.b])
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui, 3)
        ui.send('lj')
        ui.send('a')
        eventually(lambda: len(self.c.playlistinfo()) == 2)
        self.assertEqual([s['file'] for s in self.c.playlistinfo()], [self.a, self.b])
        ui.send('j\r')
        eventually(lambda: len(self.c.playlistinfo()) == 3)
        self.assertEqual([s['file'] for s in self.c.playlistinfo()], [self.a, self.b, self.b])
        self.assertEqual(self.c.status()['songid'], current)
        self.assertEqual(self.c.status()['state'], 'pause')
        self.assertEqual([s['file'] for s in self.c.listplaylistinfo('A mix')], [self.a, self.b, self.b])
        ui.send('h')
        ui.send('a')
        eventually(lambda: len(self.c.playlistinfo()) == 6)
        self.assertEqual([s['file'] for s in self.c.playlistinfo()][3:], [self.a, self.b, self.b])

    def test_empty_track_panel_does_not_add_anything(self):
        self.playlist(songs=[])
        current = self.paused_queue()
        ui = self.open()
        self.browse(ui, 0)
        ui.send('la\r')
        for _ in range(4):
            ui.pump()
        self.assert_paused_queue(current)

    def test_missing_selected_track_reports_error_without_queue_change(self):
        (self.f.root / 'playlists/Broken.m3u').write_text(self.b + '\nmissing-file.flac\n')
        current = self.paused_queue()
        ui = self.open()
        ui.send('7')
        ui.wait_text('Tracks (2)')
        ui.send('lj\r')
        ui.wait_text('No such')
        self.assert_paused_queue(current)

    def test_now_playing_art_renders_in_playlists_and_survives_dialog(self):
        cover = self.f.root / 'music/Album A/cover.png'
        write_cover(cover, [(220, 90, 30), (20, 130, 150)])
        self.assertEqual(self.c.albumart(self.a)['binary'], cover.read_bytes())
        self.playlist()
        current = self.paused_queue()
        ui = self.open(color=True)
        self.browse(ui)

        def art_visible():
            ui.pump()
            return any(ui.screen.buffer[y][x].fg in ('dc5a1e', '148296')
                       or ui.screen.buffer[y][x].bg in ('dc5a1e', '148296')
                       for y in range(4, 35) for x in range(72, 139))

        eventually(art_visible)
        ui.send('R')
        ui.wait_text('╭ Rename playlist')
        ui.send('\x1b')
        eventually(art_visible)
        self.assert_paused_queue(current)
        self.assertNotIn('Play playlist (replace queue)', ui.pump())


if __name__ == '__main__':
    unittest.main()
