use std::path::{Path, PathBuf};

use ferrocade::Error;
use ferrocade::texture;

fn ship_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/images/ship.png")
}

fn temp_file(name: &str, contents: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ferrocade-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn loads_ship_png() {
    let data = texture::load(&ship_path()).unwrap();
    assert_eq!((data.width, data.height), (99, 75));
    assert_eq!(data.pixels.len(), 99 * 75 * 4);
    assert!(data.path.is_absolute());
}

#[test]
fn missing_file_is_not_found_with_path() {
    let err = texture::load(Path::new("definitely_missing_ship.png")).unwrap_err();
    assert!(matches!(err, Error::NotFound { .. }), "{err:?}");
    assert!(err.to_string().contains("definitely_missing_ship.png"));
}

#[test]
fn corrupt_png_is_decode_error_with_path() {
    let path = temp_file("corrupt.png", b"this is not a png");
    let err = texture::load(&path).unwrap_err();
    assert!(matches!(err, Error::Decode { .. }), "{err:?}");
    assert!(err.to_string().contains("corrupt.png"));
}

#[test]
fn text_file_is_rejected() {
    let path = temp_file("notes.txt", b"hello");
    let err = texture::load(&path).unwrap_err();
    assert!(
        matches!(err, Error::Decode { .. } | Error::Unsupported { .. }),
        "{err:?}"
    );
}
