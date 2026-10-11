# MPD and terminal regression tests

These optional Unix tests run the actual Zarumet binary against a fresh MPD per test. They require `mpd`, `ffmpeg`, Python 3.10+, and the Python packages in `requirements.txt`. No real music, running MPD service, desktop window, audio device, or elevated permissions are needed. Audio output is silent.

From the repository root:

```sh
cargo build --locked --release
cargo test --locked
python3 -m venv tests/e2e/.venv
tests/e2e/.venv/bin/pip install -r tests/e2e/requirements.txt
tests/e2e/.venv/bin/python -m unittest discover -s tests/e2e -v
```

Set `ZARUMET_BINARY` to an absolute binary path to test another build. Set `ZARUMET_TEST_ARTIFACTS` to retain terminal transcripts and screen text in a chosen directory; the default is `zarumet-test-artifacts` under the system temporary directory. Output files are overwritten on the next run.

Each test creates a temporary music directory, MPD database, stored-playlist directory and Unix socket. Fixtures cover duplicate queue entries, untagged audio, Unicode/quoted/nested paths, and an empty directory. Cleanup terminates only the owned MPD and terminal processes. The suite never connects to the user's MPD port or accesses their library/configuration.

## Requirements covered

| Layer | Assertions |
| --- | --- |
| Unit tests in `binds_handler.rs` | Default mode numbers retain their meaning; partial clear sequence does nothing; an empty configured binding disables clearing; global priority wins over mode bindings; artist navigation dispatches navigation rather than playback. |
| MPD protocol contracts | A save collision preserves stored contents and duplicates; loading a playlist appends while preserving the paused current ID; directory addition is recursive and supports special characters; external saved-playlist changes emit their own idle event; command-list failure is not atomic. |
| Zarumet end to end | Removing one duplicate preserves the other current ID and pause; Enter plays the selected entry and Space pauses it; browsing all existing modes leaves queue/play state unchanged; external queue addition appears without manual refresh; queue clear requires the complete sequence. |

The MPD contract tests establish assumptions for adapters; they are not tests of unimplemented UI features. The end-to-end tests send keys through a PTY, inspect rendered screen text, and query MPD independently as the state oracle. Terminal capability queries are answered so the image probe does not consume test keystrokes.

Feature work should add focused pure-state/validation unit tests plus observable adapter/UI outcomes here. Playlist replacement failure, save dialogs, stale selections, crop, disconnects and large-directory responsiveness are not covered by this baseline. Human testing is still needed for audible output, album art, fonts and interaction feel.
