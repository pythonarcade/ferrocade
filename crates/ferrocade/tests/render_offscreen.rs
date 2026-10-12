//! Renders sprite lists offscreen and checks the pixels (skipped when no GPU is available).

use std::path::Path;

use ferrocade::color;
use ferrocade::frame::FrameCommand;
use ferrocade::render::Renderer;
use ferrocade::store::SpriteStore;
use ferrocade::texture::{self, Filter, TextureRef};

const W: u32 = 800;
const H: u32 = 600;
const BACKGROUND: [u8; 3] = [32, 32, 32];

fn ship() -> TextureRef {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/images/ship.png");
    TextureRef {
        data: texture::load(&path).unwrap(),
        filter: None,
    }
}

fn renderer() -> Option<Renderer> {
    match pollster::block_on(Renderer::offscreen(W, H)) {
        Ok(r) => Some(r),
        Err(e) => {
            eprintln!("skipping: no GPU adapter ({e})");
            None
        }
    }
}

/// Pixel at (x, y) with y measured from the bottom, like engine coordinates.
fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 3] {
    let row = H - 1 - y;
    let i = ((row * W + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2]]
}

/// Bounding box (x0, y0, x1, y1), inclusive, of non-background pixels; y from the bottom.
fn bounding_box(pixels: &[u8]) -> Option<(u32, u32, u32, u32)> {
    let mut bbox: Option<(u32, u32, u32, u32)> = None;
    for y in 0..H {
        for x in 0..W {
            if pixel(pixels, x, y) != BACKGROUND {
                bbox = Some(match bbox {
                    None => (x, y, x, y),
                    Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                });
            }
        }
    }
    bbox
}

fn render(renderer: &mut Renderer, store: &mut SpriteStore, commands: &[FrameCommand]) -> Vec<u8> {
    renderer
        .render_frame(commands, store, (W, H), Filter::Nearest)
        .unwrap();
    renderer.read_pixels().unwrap()
}

fn clear() -> FrameCommand {
    FrameCommand::Clear(color::to_linear_f32(color::DARK_GRAY))
}

#[test]
fn ship_is_centered_at_natural_size() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let mut store = SpriteStore::new();
    let sprite = store.create_sprite(ship(), 400.0, 300.0, 1.0).unwrap();
    let list = store.create_list();
    store.append(list, sprite).unwrap();

    let pixels = render(
        &mut renderer,
        &mut store,
        &[clear(), FrameCommand::DrawList(list)],
    );
    let (x0, y0, x1, y1) = bounding_box(&pixels).expect("ship was not drawn");
    let (width, height) = (x1 - x0 + 1, y1 - y0 + 1);
    assert!(
        width.abs_diff(99) <= 1 && height.abs_diff(75) <= 1,
        "size {width}x{height}"
    );
    let center_x = (x0 + x1 + 1) as f64 / 2.0;
    let center_y = (y0 + y1 + 1) as f64 / 2.0;
    assert!((center_x - 400.0).abs() <= 1.0, "center x {center_x}");
    assert!((center_y - 300.0).abs() <= 1.0, "center y {center_y}");
}

#[test]
fn transparent_pixels_show_background() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let tex = ship();
    // Find a fully transparent texel in the image and check its screen pixel.
    let (tx, ty) = (0..tex.data.height)
        .flat_map(|y| (0..tex.data.width).map(move |x| (x, y)))
        .find(|&(x, y)| tex.data.pixels[((y * tex.data.width + x) * 4 + 3) as usize] == 0)
        .expect("ship.png has no transparent pixels");

    let mut store = SpriteStore::new();
    let sprite = store.create_sprite(tex.clone(), 400.0, 300.0, 1.0).unwrap();
    let list = store.create_list();
    store.append(list, sprite).unwrap();
    let pixels = render(
        &mut renderer,
        &mut store,
        &[clear(), FrameCommand::DrawList(list)],
    );

    // The sprite's left edge is at 400 - 49.5; nearest sampling maps texel x to pixel 350 + x.
    let left = 350;
    let top = 300 + 37; // pixel row (from bottom) of the image's top row: 262.5 + 75 - 1 → 337
    assert_eq!(pixel(&pixels, left + tx, top - ty), BACKGROUND);
}

#[test]
fn origin_is_bottom_left() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let mut store = SpriteStore::new();
    let sprite = store.create_sprite(ship(), 0.0, 0.0, 1.0).unwrap();
    let list = store.create_list();
    store.append(list, sprite).unwrap();

    let pixels = render(
        &mut renderer,
        &mut store,
        &[clear(), FrameCommand::DrawList(list)],
    );
    let (x0, y0, x1, y1) = bounding_box(&pixels).expect("sprite was not drawn");
    // Only the sprite's upper-right quarter is on screen, hugging the bottom-left corner.
    assert_eq!((x0, y0), (0, 0));
    assert!(x1 < 50 && y1 < 38, "bbox {:?}", (x0, y0, x1, y1));
}

/// Whether any non-background pixel lies within `radius` of (x, y).
fn drawn_near(pixels: &[u8], x: u32, y: u32, radius: u32) -> bool {
    (y.saturating_sub(radius)..=(y + radius).min(H - 1)).any(|py| {
        (x.saturating_sub(radius)..=(x + radius).min(W - 1))
            .any(|px| pixel(pixels, px, py) != BACKGROUND)
    })
}

#[test]
fn ten_sprites_share_a_texture_and_moves_show_next_frame() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let mut store = SpriteStore::new();
    let list = store.create_list();
    let tex = ship();
    let x = |i: usize| 60 + 70 * i as u32;
    let ids: Vec<_> = (0..10)
        .map(|i| {
            let id = store
                .create_sprite(tex.clone(), x(i) as f64, 300.0, 0.5)
                .unwrap();
            store.append(list, id).unwrap();
            id
        })
        .collect();
    let frame = [clear(), FrameCommand::DrawList(list)];
    let pixels = render(&mut renderer, &mut store, &frame);
    for i in 0..10 {
        assert!(drawn_near(&pixels, x(i), 300, 10), "sprite {i} not drawn");
    }

    // Move the first sprite up; it must show at the new place on the very next frame.
    store.set_center_y(ids[0], 500.0).unwrap();
    let pixels = render(&mut renderer, &mut store, &frame);
    assert!(
        !drawn_near(&pixels, x(0), 300, 20),
        "old position still drawn"
    );
    assert!(drawn_near(&pixels, x(0), 500, 10), "new position not drawn");
    assert!(drawn_near(&pixels, x(1), 300, 10));
}

#[test]
fn later_sprites_draw_on_top() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("ferrocade-render-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let solid = |name: &str, rgb: [u8; 3]| {
        let path = dir.join(name);
        image::RgbaImage::from_pixel(20, 20, image::Rgba([rgb[0], rgb[1], rgb[2], 255]))
            .save(&path)
            .unwrap();
        TextureRef {
            data: texture::load(&path).unwrap(),
            filter: None,
        }
    };
    let mut store = SpriteStore::new();
    let list = store.create_list();
    let red = store
        .create_sprite(solid("red.png", [255, 0, 0]), 100.0, 100.0, 1.0)
        .unwrap();
    let blue = store
        .create_sprite(solid("blue.png", [0, 0, 255]), 105.0, 100.0, 1.0)
        .unwrap();
    store.append(list, red).unwrap();
    store.append(list, blue).unwrap();

    let pixels = render(
        &mut renderer,
        &mut store,
        &[clear(), FrameCommand::DrawList(list)],
    );
    assert_eq!(pixel(&pixels, 102, 100), [0, 0, 255]); // overlap: blue (appended later) wins
    assert_eq!(pixel(&pixels, 92, 100), [255, 0, 0]);
}

#[test]
fn linear_override_smooths_and_nearest_does_not() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("ferrocade-filter-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // 2x1 texture: black | white. Drawn 4x larger, linear blends a gray ramp between them.
    let path = dir.join("bw.png");
    let mut img = image::RgbaImage::new(2, 1);
    img.put_pixel(0, 0, image::Rgba([0, 0, 0, 255]));
    img.put_pixel(1, 0, image::Rgba([255, 255, 255, 255]));
    img.save(&path).unwrap();
    let data = texture::load(&path).unwrap();

    let mut store = SpriteStore::new();
    let list = store.create_list();
    let plain = TextureRef {
        data: data.clone(),
        filter: None,
    };
    let sprite = store.create_sprite(plain, 400.0, 300.0, 40.0).unwrap();
    store.append(list, sprite).unwrap();

    let mut distinct_grays = |store: &mut SpriteStore| {
        // The window filter is Nearest (see `render`); only an override can make it Linear.
        let pixels = render(
            &mut renderer,
            store,
            &[clear(), FrameCommand::DrawList(list)],
        );
        let mut values: Vec<u8> = (360..440).map(|x| pixel(&pixels, x, 300)[0]).collect();
        values.sort();
        values.dedup();
        values.len()
    };
    assert_eq!(
        distinct_grays(&mut store),
        2,
        "nearest should give only black and white"
    );
    let smooth = TextureRef {
        data,
        filter: Some(Filter::Linear),
    };
    store.set_texture(sprite, smooth).unwrap();
    assert!(
        distinct_grays(&mut store) > 10,
        "linear override should give a gray ramp"
    );
}
