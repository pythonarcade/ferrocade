//! Its own test binary: the disk-read counter is process-wide, so no other tests may load
//! textures concurrently while this one measures it.

use std::path::Path;
use std::sync::Arc;

use ferrocade::texture;

#[test]
fn same_file_is_shared_and_read_once() {
    let ship = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/images/ship.png");
    let before = texture::disk_reads();
    let first = texture::load(&ship).unwrap();
    for _ in 0..1000 {
        assert!(Arc::ptr_eq(&first, &texture::load(&ship).unwrap()));
    }
    assert_eq!(texture::disk_reads() - before, 1);
}
