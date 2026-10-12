use std::path::Path;

use ferrocade::Error;
use ferrocade::store::SpriteStore;
use ferrocade::texture::{self, TextureRef};

fn ship() -> TextureRef {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/images/ship.png");
    TextureRef {
        data: texture::load(&path).unwrap(),
        filter: None,
    }
}

#[test]
fn append_writes_instance_and_marks_dirty() {
    let mut store = SpriteStore::new();
    let sprite = store.create_sprite(ship(), 400.0, 300.0, 1.0).unwrap();
    let list = store.create_list();
    store.append(list, sprite).unwrap();

    let state = store.list(list).unwrap();
    assert_eq!(state.instances()[0].center, [400.0, 300.0]);
    assert_eq!(state.instances()[0].size, [99.0, 75.0]);
    assert_eq!(state.dirty(), Some(0..1));
}

#[test]
fn appending_twice_is_rejected() {
    let mut store = SpriteStore::new();
    let sprite = store.create_sprite(ship(), 0.0, 0.0, 1.0).unwrap();
    let list = store.create_list();
    store.append(list, sprite).unwrap();
    assert!(matches!(
        store.append(list, sprite),
        Err(Error::InvalidArgument(_))
    ));
}

#[test]
fn non_finite_values_are_rejected() {
    let mut store = SpriteStore::new();
    assert!(store.create_sprite(ship(), f64::NAN, 0.0, 1.0).is_err());
    assert!(
        store
            .create_sprite(ship(), 0.0, f64::INFINITY, 1.0)
            .is_err()
    );
    assert!(
        store
            .create_sprite(ship(), 0.0, 0.0, f64::NEG_INFINITY)
            .is_err()
    );
}

#[test]
fn setter_updates_every_list_and_marks_dirty() {
    let mut store = SpriteStore::new();
    let sprites: Vec<_> = (0..3)
        .map(|i| store.create_sprite(ship(), i as f64, 0.0, 1.0).unwrap())
        .collect();
    let (a, b) = (store.create_list(), store.create_list());
    for &s in &sprites {
        store.append(a, s).unwrap();
    }
    store.append(b, sprites[2]).unwrap();
    store.list_mut(a).unwrap().take_dirty();
    store.list_mut(b).unwrap().take_dirty();

    store.set_center_x(sprites[2], 123.0).unwrap();
    assert_eq!(store.list(a).unwrap().instances()[2].center[0], 123.0);
    assert_eq!(store.list(b).unwrap().instances()[0].center[0], 123.0);
    assert_eq!(store.list(a).unwrap().dirty(), Some(2..3));
    assert_eq!(store.list(b).unwrap().dirty(), Some(0..1));

    store.set_scale(sprites[2], 2.0).unwrap();
    assert_eq!(store.list(a).unwrap().instances()[2].size, [198.0, 150.0]);
}

#[test]
fn remove_shifts_later_slots() {
    let mut store = SpriteStore::new();
    let sprites: Vec<_> = (0..3)
        .map(|i| store.create_sprite(ship(), i as f64, 0.0, 1.0).unwrap())
        .collect();
    let list = store.create_list();
    for &s in &sprites {
        store.append(list, s).unwrap();
    }
    store.list_mut(list).unwrap().take_dirty();
    let version = store.list(list).unwrap().structure_version();

    store.remove(list, sprites[1]).unwrap();
    let state = store.list(list).unwrap();
    assert_eq!(state.sprites(), &[sprites[0], sprites[2]]);
    assert_eq!(state.instances()[1].center[0], 2.0);
    assert_eq!(state.dirty(), Some(1..2));
    assert!(state.structure_version() > version);

    // The moved sprite's slot was updated: changing it now rewrites slot 1.
    store.set_center_x(sprites[2], 50.0).unwrap();
    assert_eq!(store.list(list).unwrap().instances()[1].center[0], 50.0);

    assert!(matches!(
        store.remove(list, sprites[1]),
        Err(Error::InvalidArgument(_))
    ));
}

#[test]
fn destroying_a_list_or_sprite_cleans_up_memberships() {
    let mut store = SpriteStore::new();
    let s = store.create_sprite(ship(), 0.0, 0.0, 1.0).unwrap();
    let (a, b) = (store.create_list(), store.create_list());
    store.append(a, s).unwrap();
    store.append(b, s).unwrap();

    store.destroy_list(a);
    assert!(!store.contains(a, s));
    assert!(store.contains(b, s));

    store.destroy_sprite(s);
    assert!(store.list(b).unwrap().sprites().is_empty());
}

#[test]
fn set_texture_updates_slot_textures() {
    let mut store = SpriteStore::new();
    let s = store.create_sprite(ship(), 0.0, 0.0, 1.0).unwrap();
    let list = store.create_list();
    store.append(list, s).unwrap();
    let version = store.list(list).unwrap().structure_version();

    let mut smooth = ship();
    smooth.filter = Some(texture::Filter::Linear);
    store.set_texture(s, smooth.clone()).unwrap();
    let state = store.list(list).unwrap();
    assert_eq!(state.slot_textures()[0], smooth);
    assert!(state.structure_version() > version);
}

#[test]
fn shapes_validate_size_and_scale_with_base_size() {
    let mut store = SpriteStore::new();
    for (w, h) in [
        (0.0, 10.0),
        (10.0, -1.0),
        (f64::NAN, 5.0),
        (5.0, f64::INFINITY),
    ] {
        assert!(matches!(
            store.create_rectangle(w, h, [1, 2, 3, 255], 0.0, 0.0),
            Err(Error::InvalidArgument(_))
        ));
    }
    assert!(
        store
            .create_circle(0.0, [1, 2, 3, 255], 0.0, 0.0, None)
            .is_err()
    );

    let r = store
        .create_rectangle(50.0, 20.0, [1, 2, 3, 255], 0.0, 0.0)
        .unwrap();
    let list = store.create_list();
    store.append(list, r).unwrap();
    assert_eq!(store.list(list).unwrap().instances()[0].size, [50.0, 20.0]);
    store.set_scale(r, 2.0).unwrap();
    assert_eq!(store.list(list).unwrap().instances()[0].size, [100.0, 40.0]);
    assert!(store.sprite(r).unwrap().is_shape());
}

#[test]
fn set_color_updates_every_list_and_marks_dirty() {
    let mut store = SpriteStore::new();
    let s = store.create_sprite(ship(), 0.0, 0.0, 1.0).unwrap();
    let (a, b) = (store.create_list(), store.create_list());
    store.append(a, s).unwrap();
    store.append(b, s).unwrap();
    assert_eq!(
        store.list(a).unwrap().instances()[0].color,
        [255, 255, 255, 255]
    );
    store.list_mut(a).unwrap().take_dirty();
    store.list_mut(b).unwrap().take_dirty();
    store.set_color(s, [10, 20, 30, 40]).unwrap();
    for list in [a, b] {
        assert_eq!(
            store.list(list).unwrap().instances()[0].color,
            [10, 20, 30, 40]
        );
        assert_eq!(store.list(list).unwrap().dirty(), Some(0..1));
    }
}
