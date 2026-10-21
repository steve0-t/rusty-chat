//! Initially generated with AI assistance on 2026-10-21.
use super::read_last_logged_user;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct SessionFileFixture {
    directory: PathBuf,
}

impl SessionFileFixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "chat-app-client-session-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).expect("create isolated session-file fixture");
        Self { directory }
    }

    fn path(&self) -> PathBuf {
        self.directory.join("last_logged_in")
    }

    fn write(&self, contents: impl AsRef<[u8]>) {
        fs::write(self.path(), contents).expect("write session-file fixture");
    }
}

impl Drop for SessionFileFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn missing_session_file_returns_none() {
    let fixture = SessionFileFixture::new();
    assert_eq!(read_last_logged_user(fixture.path()), None);
}

#[test]
fn missing_session_directory_returns_none() {
    let fixture = SessionFileFixture::new();
    assert_eq!(
        read_last_logged_user(fixture.directory.join("missing/last_logged_in")),
        None
    );
}

#[test]
fn directory_in_place_of_session_file_returns_none() {
    let fixture = SessionFileFixture::new();
    fs::create_dir(fixture.path()).unwrap();
    assert_eq!(read_last_logged_user(fixture.path()), None);
}

#[test]
fn ascii_username_is_read_from_session_file() {
    let fixture = SessionFileFixture::new();
    fixture.write("alice");
    assert_eq!(
        read_last_logged_user(fixture.path()).as_deref(),
        Some("alice")
    );
}

#[test]
fn unicode_username_is_read_without_corruption() {
    let fixture = SessionFileFixture::new();
    fixture.write("Žofia_界_🦀");
    assert_eq!(
        read_last_logged_user(fixture.path()).as_deref(),
        Some("Žofia_界_🦀")
    );
}

#[test]
fn invalid_utf8_session_file_returns_none() {
    let fixture = SessionFileFixture::new();
    fixture.write([b'a', 0xff, 0xfe]);
    assert_eq!(read_last_logged_user(fixture.path()), None);
}

#[test]
fn updated_session_file_is_read_again_instead_of_cached() {
    let fixture = SessionFileFixture::new();
    fixture.write("alice");
    assert_eq!(
        read_last_logged_user(fixture.path()).as_deref(),
        Some("alice")
    );
    fixture.write("bob");
    assert_eq!(
        read_last_logged_user(fixture.path()).as_deref(),
        Some("bob")
    );
}

#[test]
fn deleted_session_file_returns_none_on_subsequent_read() {
    let fixture = SessionFileFixture::new();
    fixture.write("alice");
    assert!(read_last_logged_user(fixture.path()).is_some());
    fs::remove_file(fixture.path()).unwrap();
    assert_eq!(read_last_logged_user(fixture.path()), None);
}

#[test]
fn independent_session_paths_do_not_share_usernames() {
    let first = SessionFileFixture::new();
    let second = SessionFileFixture::new();
    first.write("alice");
    second.write("bob");
    assert_eq!(
        read_last_logged_user(first.path()).as_deref(),
        Some("alice")
    );
    assert_eq!(read_last_logged_user(second.path()).as_deref(), Some("bob"));
}

#[test]
fn regression_empty_session_file_does_not_select_an_empty_database_name() {
    let fixture = SessionFileFixture::new();
    fixture.write("");
    assert_eq!(read_last_logged_user(fixture.path()), None);
}
