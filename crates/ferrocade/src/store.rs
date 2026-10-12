//! Central sprite store: all sprite and sprite-list data lives here, in Rust.

slotmap::new_key_type! {
    pub struct SpriteId;
    pub struct ListId;
}
