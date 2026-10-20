//! A small software rasteriser, so a frame can be produced with no GPU, no
//! window and no display.
//!
//! # Why it lives in the UI crate
//!
//! "Run the command and look at the result" is the difference between an
//! agent-ready application and one an agent can only hope at. A JSON dump of
//! layout proves the layout; a PNG proves the *rendering*, which is where most
//! visible bugs live.
//!
//! It is here, in `craftkit-ui`, rather than in the CLI, because three callers
//! need it and the CLI is only the first:
//!
//! - `craftkit-cli snapshot` writes a PNG to a file;
//! - the control channel's `ui.render` returns a PNG to an agent;
//! - the control channel's `ui.click` resolves a coordinate against the same
//!   layout, so "what is at (412, 288)?" has an answer that matches the
//!   window.
//!
//! All three draw into a fresh [`egui::Context`]. Nothing here reads a
//! framebuffer, which is what makes it work on a build machine with no display
//! and inside a process whose window is on somebody else's screen.
//!
//! # What egui gives us
//!
//! [`egui::Context::tessellate`] returns triangles with a position, a colour and
//! a texture coordinate. That is everything a rasteriser needs, and it means
//! this file has no knowledge of widgets, layout or craftkit at all — it draws
//! shapes, and any egui program can use it.
//!
//! # Text
//!
//! egui batches glyph quads against the font atlas and names the atlas in each
//! [`Mesh::texture_id`], so this samples it and **text rasterises correctly**.
//! The atlas comes from the frame's `TexturesDelta`, which is where egui puts
//! newly rasterised glyphs.
//!
//! # Limits, stated plainly
//!
//! - Edges are hard. egui's geometry is the only anti-aliasing and there is
//!   none beyond it, so a rendered frame looks crisper and slightly jaggy
//!   compared with the window.
//! - Gradients are sampled at the triangle's centroid rather than per pixel.
//!   craftkit's theme has none; a future theme that adds one will band.
//! - Only the font atlas is uploaded. An image a document loaded would need its
//!   texture delta collected too, and until then it draws as a flat rectangle.

// ---------------------------------------------------------------------------
// The session-level API
// ---------------------------------------------------------------------------

/// The default frame size, chosen to match a laptop window.
///
/// An agent gets a picture of the same proportions the window shows, so a
/// screenshot taken headlessly can be compared with a screenshot taken of the
/// window without rescaling.
pub const DEFAULT_SIZE: [usize; 2] = [1180, 720];

/// Bounds on a requested frame size.
///
/// Hostile input like any other: `width` and `height` arrive from an agent, a
/// script or a file, and a zero or a hundred-thousand would either divide by
/// zero or allocate until the process dies.
pub const MIN_SIZE: usize = 64;
pub const MAX_SIZE: usize = 8192;

/// How many pixels a frame may have in total.
///
/// [`MAX_SIZE`] bounds each side; this bounds the area, which is what the
/// memory is actually made of. Without it the cap is per side and the area
/// is whatever two sides multiply to: 8192 × 8192 is 67 million pixels, and
/// the frame that holds them is 268 MB of pixels before the PNG and its
/// base64 copy — measured 301 MB of resident memory, 3.5 seconds of drawing
/// and a 14 789 463-byte reply. The limit's own words are *"allocate until the
/// process dies"*, and 268 MB is not that on a build machine. §59.
///
/// 16 777 216 is 4096 × 4096: it keeps every frame the tool draws itself and
/// every size a person would ask for, and refuses the one that hurts.
pub const MAX_PIXELS: usize = 16_777_216;

/// How many frames to draw.
///
/// egui needs one frame to lay out and one to settle: the first lays panels out
/// at their default sizes, the second places them. With one frame a snapshot
/// shows an unsettled layout, which is the sort of thing an agent reports as a
/// bug that does not exist.
pub const FRAMES: usize = 2;

/// What one headless frame produced.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// The PNG bytes. Empty only if encoding failed, which [`Frame`] refuses.
    pub png: Vec<u8>,
    pub width: usize,
    pub height: usize,
    /// The shell's state after the frame: selection and the status line.
    pub state: crate::shell::ShellState,
    /// Every widget that was present, for "what is at this point?" and for an
    /// agent to check that a control it expects is actually on screen.
    pub automation: crate::automation::Automation,
}

/// Check a requested size, or say what is wrong with it.
pub fn check_size(width: usize, height: usize) -> Result<(), String> {
    if width < MIN_SIZE || height < MIN_SIZE {
        return Err(format!("a frame must be at least {MIN_SIZE}x{MIN_SIZE}; asked for {width}x{height}"));
    }
    if width > MAX_SIZE || height > MAX_SIZE {
        return Err(format!("a frame may be at most {MAX_SIZE}x{MAX_SIZE}; asked for {width}x{height}"));
    }
    // The area, not the sides: the memory a frame costs is its pixels.
    let pixels = width.saturating_mul(height);
    if pixels > MAX_PIXELS {
        return Err(format!(
            "a frame may be at most {MAX_PIXELS} pixels; asked for {width}x{height}, which is {pixels}"
        ));
    }
    Ok(())
}

/// Draw the shell headlessly and return the frame.
///
/// `session` is borrowed, not cloned: this draws, it does not act. The two
/// frames are drawn against one shell, so a panel that caches state sees the
/// same shell both times, exactly as it would in a window.
///
/// # Errors
///
/// The requested size is out of bounds, or PNG encoding failed. Both name what
/// was asked for.
pub fn render(session: &mut craftkit_engine::Session, width: usize, height: usize) -> Result<Frame, String> {
    check_size(width, height)?;

    let ctx = egui::Context::default();
    let tokens = crate::theme::Tokens::dark();
    let screen_rect =
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(width as f32, height as f32));
    let raw = egui::RawInput { screen_rect: Some(screen_rect), ..Default::default() };

    let mut shell = crate::shell::Shell::new(session);
    let mut deltas: Vec<egui::TexturesDelta> = Vec::new();
    let mut shapes: Vec<egui::ClippedPrimitive> = Vec::new();
    for _ in 0..FRAMES {
        let out = ctx.run(raw.clone(), |ctx| shell.show(ctx, &tokens));
        shapes = ctx.tessellate(out.shapes, 1.0);
        // The font atlas arrives in these deltas. Without it every glyph
        // rasterises as a solid block, which looks like a rendering bug and is
        // not one.
        deltas.push(out.textures_delta);
    }

    let atlas = collect_textures(&deltas);
    let image = render_pixels(width, height, &shapes, &atlas);
    let png = to_png(&image)?;

    Ok(Frame { png, width, height, state: shell.state.clone(), automation: shell.automation.clone() })
}

/// Draw the shell headlessly at [`DEFAULT_SIZE`].
pub fn render_default(session: &mut craftkit_engine::Session) -> Result<Frame, String> {
    render(session, DEFAULT_SIZE[0], DEFAULT_SIZE[1])
}

/// Render with whatever pointer input the caller supplies, and report what came
/// back.
///
/// This is how `ui.click` works. egui consumes input once per frame, so a
/// caller cannot poke a live context from another thread; instead the input is
/// *replayed* into a fresh context and the automation registry says which
/// widget the click landed on. The answer is the layout's answer, which is the
/// honest one — it is the widget that was at that point, not a guess.
pub fn click(
    session: &mut craftkit_engine::Session,
    x: f32,
    y: f32,
    width: usize,
    height: usize,
) -> Result<Click, String> {
    check_size(width, height)?;
    if !x.is_finite() || !y.is_finite() {
        return Err(format!("a click at ({x}, {y}) is not a point in this world"));
    }

    let ctx = egui::Context::default();
    let tokens = crate::theme::Tokens::dark();
    let screen_rect =
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(width as f32, height as f32));
    let raw = egui::RawInput { screen_rect: Some(screen_rect), ..Default::default() };

    // Settle the layout first, with no pointer at all. A click into a layout
    // that has not settled answers about where things *will* be.
    let mut shell = crate::shell::Shell::new(session);
    for _ in 0..FRAMES {
        let _ = ctx.run(raw.clone(), |ctx| shell.show(ctx, &tokens));
    }

    let before = shell.automation.clone();
    let down = egui::RawInput {
        screen_rect: Some(screen_rect),
        events: vec![egui::Event::PointerButton {
            pos: egui::pos2(x, y),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        }],
        ..Default::default()
    };
    let _ = ctx.run(down, |ctx| shell.show(ctx, &tokens));

    Ok(Click {
        point: [x, y],
        hit: before.at(x, y).map(str::to_string),
        clicked: shell.automation.clone(),
        state: shell.state.clone(),
    })
}

/// What [`click`] found.
#[derive(Debug, Clone, PartialEq)]
pub struct Click {
    pub point: [f32; 2],
    /// The name of the topmost widget at the point, before the click.
    ///
    /// Owned, because the registry it was read from is a local of the click
    /// and is gone by the time the answer is returned. §66.
    pub hit: Option<String>,
    /// The widgets present after the click, for a caller that wants to see what
    /// changed.
    pub clicked: crate::automation::Automation,
    pub state: crate::shell::ShellState,
}

/// An RGBA image, 8 bits per channel.
pub struct Image {
    pub width: usize,
    pub height: usize,
    /// Row-major, four bytes per pixel.
    pub pixels: Vec<u8>,
}

impl Image {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height, pixels: vec![0; width * height * 4] }
    }

    /// Blend one pixel. `alpha` is 0..=1.
    fn blend(&mut self, x: usize, y: usize, colour: [u8; 4], alpha: f32) {
        if x >= self.width || y >= self.height || alpha <= 0.0 {
            return;
        }
        let a = f32::from(colour[3]) / 255.0 * alpha.clamp(0.0, 1.0);
        if a <= 0.0 {
            return;
        }
        let i = (y * self.width + x) * 4;
        for (c, src) in [colour[0], colour[1], colour[2]].into_iter().enumerate() {
            let dst = f32::from(self.pixels[i + c]);
            self.pixels[i + c] = (f32::from(src) * a + dst * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
        }
        self.pixels[i + 3] =
            (255.0 * a + f32::from(self.pixels[i + 3]) * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
    }
}

/// A texture egui sampled against.
#[derive(Debug, Clone)]
pub struct AtlasTexture {
    pub width: usize,
    pub height: usize,
    /// Straight (not premultiplied) RGBA, four bytes per pixel.
    pub pixels: Vec<u8>,
}

impl AtlasTexture {
    /// From an egui image delta.
    ///
    /// `ImageData` has one variant today; the `match` is written out anyway so
    /// adding a second one is a compile error here rather than a silently
    /// dropped texture.
    pub fn from_image(delta: &egui::epaint::ImageDelta) -> Option<Self> {
        match &delta.image {
            egui::ImageData::Color(colour) => {
                let mut pixels = Vec::with_capacity(colour.pixels.len() * 4);
                for px in &colour.pixels {
                    let [r, g, b, a] = px.to_array();
                    pixels.extend_from_slice(&[r, g, b, a]);
                }
                Some(Self { width: colour.size[0], height: colour.size[1], pixels })
            }
        }
    }
}

/// Every texture a set of frames produced, keyed by the id meshes refer to.
pub fn collect_textures(
    deltas: &[egui::TexturesDelta],
) -> std::collections::BTreeMap<egui::TextureId, AtlasTexture> {
    let mut map = std::collections::BTreeMap::new();
    for delta in deltas {
        for (id, image) in &delta.set {
            if let Some(tex) = AtlasTexture::from_image(image) {
                map.insert(*id, tex);
            }
        }
    }
    map
}

/// Rasterise a set of tessellated primitives into an image.
///
/// `atlas` supplies the textures the meshes name. A mesh whose texture is absent
/// draws in flat colour, so a missing atlas costs legibility rather than the
/// whole picture.
pub fn render_pixels(
    width: usize,
    height: usize,
    shapes: &[egui::ClippedPrimitive],
    atlas: &std::collections::BTreeMap<egui::TextureId, AtlasTexture>,
) -> Image {
    let mut image = Image::new(width, height);
    for clipped in shapes {
        let clip = clip_rect(clipped.clip_rect, width, height);
        // A paint callback has no geometry of its own; skipping it is exactly
        // what a GPU would do.
        let mesh = match &clipped.primitive {
            egui::epaint::Primitive::Mesh(m) => m,
            egui::epaint::Primitive::Callback(_) => continue,
        };
        for triangle in mesh.indices.chunks_exact(3) {
            // A mesh with a short index list is malformed; skipping the
            // triangle draws less, which beats panicking in a screenshot tool.
            let (Some(a), Some(b), Some(c)) =
                (vertex(mesh, triangle[0]), vertex(mesh, triangle[1]), vertex(mesh, triangle[2]))
            else {
                continue;
            };
            fill_triangle(&mut image, a, b, c, &mesh.texture_id, atlas, clip);
        }
    }
    image
}

/// One triangle's worth of data, already sampled.
struct Vtx {
    x: f32,
    y: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
    u: f32,
    v: f32,
}

fn vertex(mesh: &egui::epaint::Mesh, index: u32) -> Option<Vtx> {
    let v: &egui::epaint::Vertex = mesh.vertices.get(index as usize)?;
    let [r, g, b, a] = v.color.to_array();
    Some(Vtx {
        x: v.pos.x,
        y: v.pos.y,
        r: f32::from(r),
        g: f32::from(g),
        b: f32::from(b),
        a: f32::from(a),
        u: v.uv.x,
        v: v.uv.y,
    })
}

fn clip_rect(rect: egui::Rect, width: usize, height: usize) -> (usize, usize, usize, usize) {
    let min_x = rect.min.x.floor().max(0.0) as usize;
    let min_y = rect.min.y.floor().max(0.0) as usize;
    let max_x = (rect.max.x.ceil().max(0.0) as usize).min(width);
    let max_y = (rect.max.y.ceil().max(0.0) as usize).min(height);
    (min_x.min(width), min_y.min(height), max_x.min(width), max_y.min(height))
}

/// Barycentric scanline fill with perspective-correct interpolation.
///
/// Not the fastest rasteriser there is, and not trying to be: a screenshot is
/// a few hundred thousand triangles at most, once, and correctness plus no
/// dependencies is worth more here than throughput.
fn fill_triangle(
    image: &mut Image,
    a: Vtx,
    b: Vtx,
    c: Vtx,
    texture: &egui::TextureId,
    atlas: &std::collections::BTreeMap<egui::TextureId, AtlasTexture>,
    clip: (usize, usize, usize, usize),
) {
    // Work in integer pixel space so the edge functions stay small and exact.
    let (ax, ay) = ((a.x * 16.0).round() as i64, (a.y * 16.0).round() as i64);
    let (bx, by) = ((b.x * 16.0).round() as i64, (b.y * 16.0).round() as i64);
    let (cx, cy) = ((c.x * 16.0).round() as i64, (c.y * 16.0).round() as i64);

    let area = edge(ax, ay, bx, by, cx, cy);
    if area == 0 {
        return;
    }
    // A negative area means the winding is backwards; flipping it keeps one code
    // path instead of testing the sign on every pixel.
    let (ax, ay, bx, by, cx, cy, area) =
        if area < 0 { (ax, ay, cx, cy, bx, by, -area) } else { (ax, ay, bx, by, cx, cy, area) };

    let min_x = (ax.min(bx).min(cx) >> 4).max(clip.0 as i64);
    let max_x = ((ax.max(bx).max(cx) >> 4) + 1).min(clip.2 as i64);
    let min_y = (ay.min(by).min(cy) >> 4).max(clip.1 as i64);
    let max_y = ((ay.max(by).max(cy) >> 4) + 1).min(clip.3 as i64);

    for py in min_y..max_y {
        for px in min_x..max_x {
            let sx = px * 16 + 8;
            let sy = py * 16 + 8;
            let w0 = edge(bx, by, cx, cy, sx, sy);
            let w1 = edge(cx, cy, ax, ay, sx, sy);
            let w2 = edge(ax, ay, bx, by, sx, sy);
            if w0 < 0 || w1 < 0 || w2 < 0 {
                continue;
            }
            let (f0, f1, f2) = (w0 as f32 / area as f32, w1 as f32 / area as f32, w2 as f32 / area as f32);

            let r = a.r * f0 + b.r * f1 + c.r * f2;
            let g = a.g * f0 + b.g * f1 + c.g * f2;
            let blue = a.b * f0 + b.b * f1 + c.b * f2;
            let al = a.a * f0 + b.a * f1 + c.a * f2;
            if al <= 0.0 {
                continue;
            }

            let u = a.u * f0 + b.u * f1 + c.u * f2;
            let v = a.v * f0 + b.v * f1 + c.v * f2;

            // A textured glyph multiplies its atlas colour by the vertex
            // colour, which is how egui tints text. A missing texture falls
            // back to the flat colour, so a font problem costs legibility
            // rather than the whole picture.
            let (final_r, final_g, final_b, tex_a) = match sample(texture, atlas, u, v) {
                Some((sr, sg, sb, sa)) => (
                    (f32::from(sr) * r / 255.0) as u8,
                    (f32::from(sg) * g / 255.0) as u8,
                    (f32::from(sb) * blue / 255.0) as u8,
                    sa,
                ),
                None => (r as u8, g as u8, blue as u8, 255),
            };

            image.blend(
                px as usize,
                py as usize,
                [final_r, final_g, final_b, 255],
                al * f32::from(tex_a) / 255.0,
            );
        }
    }
}

fn sample(
    texture: &egui::TextureId,
    atlas: &std::collections::BTreeMap<egui::TextureId, AtlasTexture>,
    u: f32,
    v: f32,
) -> Option<(u8, u8, u8, u8)> {
    let tex = atlas.get(texture)?;
    if tex.width == 0 || tex.height == 0 || tex.pixels.len() < tex.width * tex.height * 4 {
        return None;
    }
    let x = (u.clamp(0.0, 1.0) * (tex.width - 1) as f32).round() as usize;
    let y = (v.clamp(0.0, 1.0) * (tex.height - 1) as f32).round() as usize;
    let i = (y * tex.width + x) * 4;
    let slice = tex.pixels.get(i..i + 4)?;
    Some((slice[0], slice[1], slice[2], slice[3]))
}

/// The signed area of the edge a→b at p, in 1/16-pixel units.
fn edge(ax: i64, ay: i64, bx: i64, by: i64, px: i64, py: i64) -> i64 {
    (bx - ax) * (py - ay) - (by - ay) * (px - ax)
}

/// Encode an image as a PNG.
///
/// `PngEncoder` rather than `save_buffer`, which picks its format from a file
/// extension — so a `snapshot` aimed at `shot.jpg` would silently write a
/// different format under the name the caller asked for.
pub fn to_png(image: &Image) -> Result<Vec<u8>, String> {
    let mut out = std::io::BufWriter::new(Vec::new());
    use image::ImageEncoder as _;
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&image.pixels, image.width as u32, image.height as u32, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    out.into_inner().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn triangle_mesh() -> Vec<egui::ClippedPrimitive> {
        use egui::epaint::{Color32, Mesh, Pos2, Primitive, Rect, Vec2};
        let mesh = Mesh {
            texture_id: egui::TextureId::default(),
            indices: vec![0, 1, 2],
            vertices: vec![
                egui::epaint::Vertex {
                    pos: Pos2::new(0.0, 0.0),
                    uv: Pos2::new(0.0, 0.0),
                    color: Color32::RED,
                },
                egui::epaint::Vertex {
                    pos: Pos2::new(10.0, 0.0),
                    uv: Pos2::new(1.0, 0.0),
                    color: Color32::RED,
                },
                egui::epaint::Vertex {
                    pos: Pos2::new(0.0, 10.0),
                    uv: Pos2::new(0.0, 1.0),
                    color: Color32::RED,
                },
            ],
        };
        vec![egui::ClippedPrimitive {
            clip_rect: Rect::from_min_size(Pos2::ZERO, Vec2::new(20.0, 20.0)),
            primitive: Primitive::Mesh(mesh),
        }]
    }

    fn pixel(image: &Image, x: usize, y: usize) -> [u8; 4] {
        let i = (y * image.width + x) * 4;
        [image.pixels[i], image.pixels[i + 1], image.pixels[i + 2], image.pixels[i + 3]]
    }

    #[test]
    fn a_triangle_lands_where_it_was_put() {
        let image = render_pixels(20, 20, &triangle_mesh(), &BTreeMap::new());
        assert_eq!(&pixel(&image, 1, 1)[..3], &[255, 0, 0], "inside the triangle");
        assert_eq!(&pixel(&image, 18, 18)[..3], &[0, 0, 0], "outside it");
    }

    #[test]
    fn an_opaque_triangle_is_opaque() {
        let image = render_pixels(20, 20, &triangle_mesh(), &BTreeMap::new());
        assert_eq!(pixel(&image, 1, 1)[3], 255);
    }

    #[test]
    fn the_clip_rect_is_honoured() {
        let mut shapes = triangle_mesh();
        shapes[0].clip_rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(20.0, 20.0));
        let image = render_pixels(20, 20, &shapes, &BTreeMap::new());
        assert!(image.pixels.chunks_exact(4).all(|p| p[3] == 0), "everything was clipped away");
    }

    #[test]
    fn a_degenerate_triangle_draws_nothing_rather_than_looping_forever() {
        let mut shapes = triangle_mesh();
        if let egui::epaint::Primitive::Mesh(m) = &mut shapes[0].primitive {
            m.vertices[2].pos = m.vertices[0].pos;
        }
        let image = render_pixels(20, 20, &shapes, &BTreeMap::new());
        assert!(image.pixels.chunks_exact(4).all(|p| p[3] == 0));
    }

    #[test]
    fn a_backwards_winding_draws_the_same_triangle() {
        let mut shapes = triangle_mesh();
        if let egui::epaint::Primitive::Mesh(m) = &mut shapes[0].primitive {
            m.indices = vec![0, 2, 1];
        }
        let image = render_pixels(20, 20, &shapes, &BTreeMap::new());
        assert_eq!(pixel(&image, 1, 1)[0], 255, "the winding must not change what is drawn");
    }

    #[test]
    fn a_half_transparent_triangle_blends_over_what_is_there() {
        let mut shapes = triangle_mesh();
        if let egui::epaint::Primitive::Mesh(m) = &mut shapes[0].primitive {
            for v in &mut m.vertices {
                v.color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 128);
            }
        }
        // Something underneath to blend against.
        let mut below = triangle_mesh();
        if let egui::epaint::Primitive::Mesh(m) = &mut below[0].primitive {
            for v in &mut m.vertices {
                v.color = egui::Color32::BLUE;
            }
        }
        below.extend(shapes);
        let image = render_pixels(20, 20, &below, &BTreeMap::new());
        let p = pixel(&image, 1, 1);
        assert!(p[0] > 100 && p[2] > 100, "a 50% red over blue should show both: {p:?}");
        assert_eq!(p[3], 255, "the result is opaque");
    }

    #[test]
    fn an_empty_shape_list_gives_a_transparent_image() {
        // (the empty case)
        let image = render_pixels(4, 4, &[], &BTreeMap::new());
        assert_eq!(image.pixels.len(), 4 * 4 * 4);
        assert!(image.pixels.iter().all(|b| *b == 0));
    }

    #[test]
    fn shapes_outside_the_canvas_are_clipped_not_crashed_on() {
        let mut shapes = triangle_mesh();
        if let egui::epaint::Primitive::Mesh(m) = &mut shapes[0].primitive {
            for v in &mut m.vertices {
                v.pos.x += 1000.0;
                v.pos.y += 1000.0;
            }
        }
        let image = render_pixels(20, 20, &shapes, &BTreeMap::new());
        assert!(image.pixels.iter().all(|b| *b == 0));
    }

    #[test]
    fn negative_coordinates_are_clipped_too() {
        let mut shapes = triangle_mesh();
        if let egui::epaint::Primitive::Mesh(m) = &mut shapes[0].primitive {
            for v in &mut m.vertices {
                v.pos.x -= 1000.0;
            }
        }
        let image = render_pixels(20, 20, &shapes, &BTreeMap::new());
        assert!(image.pixels.iter().all(|b| *b == 0));
    }

    #[test]
    fn a_missing_atlas_costs_text_not_the_picture() {
        let shapes = triangle_mesh();
        let mut atlas = BTreeMap::new();
        atlas.insert(
            egui::TextureId::default(),
            AtlasTexture { width: 1, height: 1, pixels: vec![255, 255, 255, 255] },
        );
        let without = render_pixels(20, 20, &shapes, &BTreeMap::new());
        let with = render_pixels(20, 20, &shapes, &atlas);
        // A red triangle multiplied by a white atlas pixel is still red, so the
        // two agree — which is the point: the atlas changes legibility, not the
        // geometry.
        assert_eq!(pixel(&without, 1, 1)[..3], pixel(&with, 1, 1)[..3]);
    }

    #[test]
    fn a_transparent_atlas_pixel_makes_the_glyph_transparent() {
        let mut shapes = triangle_mesh();
        if let egui::epaint::Primitive::Mesh(m) = &mut shapes[0].primitive {
            for v in &mut m.vertices {
                v.uv = egui::Pos2::new(0.0, 0.0);
            }
        }
        let mut atlas = BTreeMap::new();
        atlas.insert(
            egui::TextureId::default(),
            AtlasTexture { width: 1, height: 1, pixels: vec![0, 0, 0, 0] },
        );
        let image = render_pixels(20, 20, &shapes, &atlas);
        assert_eq!(pixel(&image, 1, 1)[3], 0, "a fully transparent atlas pixel draws nothing");
    }

    #[test]
    fn sampling_outside_the_texture_clamps_rather_than_panicking() {
        let mut atlas = BTreeMap::new();
        atlas.insert(
            egui::TextureId::default(),
            AtlasTexture { width: 1, height: 1, pixels: vec![1, 2, 3, 4] },
        );
        assert!(sample(&egui::TextureId::default(), &atlas, -5.0, 99.0).is_some());
        assert!(sample(&egui::TextureId::default(), &BTreeMap::new(), 0.5, 0.5).is_none());
    }

    #[test]
    fn a_zero_sized_atlas_is_ignored() {
        let mut atlas = BTreeMap::new();
        atlas.insert(egui::TextureId::default(), AtlasTexture { width: 0, height: 0, pixels: vec![] });
        assert!(sample(&egui::TextureId::default(), &atlas, 0.5, 0.5).is_none());
    }

    #[test]
    fn an_empty_delta_contributes_no_textures() {
        let deltas = vec![egui::TexturesDelta::default()];
        assert!(collect_textures(&deltas).is_empty());
    }

    #[test]
    fn a_colour_image_becomes_an_atlas_texture() {
        let colour = egui::ColorImage::new([2, 2], vec![egui::Color32::WHITE; 4]);
        let delta = egui::epaint::ImageDelta {
            image: egui::ImageData::Color(std::sync::Arc::new(colour)),
            options: egui::TextureOptions::default(),
            pos: None,
        };
        let tex = AtlasTexture::from_image(&delta).expect("converted");
        assert_eq!((tex.width, tex.height), (2, 2));
        assert_eq!(tex.pixels.len(), 16);
    }

    // ---- the session-level API ------------------------------------------------

    #[test]
    fn a_session_renders_to_a_real_png() {
        let mut session = craftkit_engine::Session::sample();
        let frame = render(&mut session, 400, 300).expect("renders");
        assert_eq!((frame.width, frame.height), (400, 300));
        assert_eq!(&frame.png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a], "not a PNG");
        assert!(frame.png.len() > 1000, "a blank frame would be tiny: {}", frame.png.len());
    }

    #[test]
    fn a_rendered_frame_is_not_blank() {
        let mut session = craftkit_engine::Session::sample();
        let frame = render(&mut session, 320, 240).expect("renders");
        // Decoded with a real decoder rather than by reading our own bytes
        // back: "the encoder wrote something" and "it is a viewable image" are
        // different claims.
        let decoded = image::load_from_memory(&frame.png).expect("a real decoder reads it");
        let rgba = decoded.to_rgba8();
        assert_eq!((rgba.width(), rgba.height()), (320, 240));
        let painted = rgba.pixels().filter(|p| p.0[3] != 0).count();
        assert!(painted > 1000, "only {painted} pixels are opaque; the frame is blank");
    }

    #[test]
    fn a_rendered_frame_reports_the_widgets_on_screen() {
        let mut session = craftkit_engine::Session::sample();
        let frame = render(&mut session, 600, 400).expect("renders");
        let names = frame.automation.names();
        assert!(!names.is_empty(), "the shell published no widgets at all");
        assert!(names.iter().any(|n| n.contains("node")), "the node list should be on screen: {names:?}");
    }

    #[test]
    fn rendering_does_not_change_the_document() {
        // Rendering is a read. If it were a write, taking a picture of a
        // document would edit it, which is the kind of bug that only shows up
        // as unexplained history.
        let mut session = craftkit_engine::Session::sample();
        let before = session.snapshot_json();
        let _ = render(&mut session, 320, 240).expect("renders");
        assert_eq!(session.snapshot_json(), before);
        assert!(!session.is_dirty(), "a render must not make the session dirty");
        assert_eq!(session.undo_depth(), 0, "a render must not push an undo entry");
    }

    #[test]
    fn an_absurd_size_is_refused_by_name() {
        let mut session = craftkit_engine::Session::sample();
        let err = render(&mut session, 0, 100).expect_err("refused");
        assert!(err.contains("at least"), "{err}");
        assert!(err.contains("0x100"), "the report should say what was asked for: {err}");

        let err = render(&mut session, 100, usize::MAX).expect_err("refused");
        assert!(err.contains("at most"), "{err}");
    }

    /// The area of a frame is bounded, not just its sides.
    ///
    /// Two sides of 8192 are each allowed, and together they are 67 million
    /// pixels — 268 MB of pixels, 301 MB of resident memory and a
    /// 14 789 463-byte reply, measured. §59.
    #[test]
    fn a_frame_that_is_too_big_in_pixels_is_refused() {
        let mut session = craftkit_engine::Session::sample();

        // Each side is inside the per-side limit, and the area is not: the
        // refusal has to be about the pixels, not about the sides.
        let err = render(&mut session, MAX_SIZE, MAX_SIZE).expect_err("refused");
        assert!(err.contains("pixels"), "the refusal should name the limit it broke: {err}");
        assert!(err.contains(&format!("{MAX_SIZE}x{MAX_SIZE}")), "and what was asked for: {err}");

        // A frame whose area is allowed is still allowed, whatever its shape.
        assert!(check_size(MAX_SIZE, 2048).is_ok(), "a wide frame inside the budget is fine");
        assert!(check_size(4096, 4096).is_ok(), "4096 squared is exactly the budget");
        assert!(check_size(4097, 4096).is_err(), "one row over the budget is not");
    }

    /// The default frame is inside the budget, and so is a retina-sized one.
    #[test]
    fn the_frames_the_tool_draws_are_inside_the_budget() {
        assert!(check_size(DEFAULT_SIZE[0], DEFAULT_SIZE[1]).is_ok());
        assert!(check_size(2360, 1440).is_ok(), "a retina-sized frame");
        assert!(check_size(3840, 2160).is_ok(), "a 4K frame");
    }

    #[test]
    fn a_click_on_a_widget_names_that_widget() {
        let mut session = craftkit_engine::Session::sample();
        // Settle once to learn where things are, then click exactly there.
        let frame = render(&mut session, 600, 400).expect("renders");
        let name = frame.automation.names().into_iter().find(|n| n.contains("node")).expect("a node row");
        let rect = frame.automation.rect(name).expect("the row has a rectangle");
        let click = click(&mut session, rect.center().x, rect.center().y, 600, 400).expect("clicks");
        assert_eq!(click.hit.as_deref(), Some(name), "clicked the centre of {name}");
    }

    #[test]
    fn a_click_on_empty_space_says_so_rather_than_guessing() {
        let mut session = craftkit_engine::Session::sample();
        // The bottom-right corner of the frame is background in this layout.
        let hit = click(&mut session, 597.0, 397.0, 600, 400).expect("clicks");
        assert_eq!(hit.hit, None, "something was found at the corner: {:?}", hit.hit);
    }

    #[test]
    fn a_click_outside_the_frame_is_not_a_click() {
        let mut session = craftkit_engine::Session::sample();
        assert_eq!(click(&mut session, 5000.0, 10.0, 600, 400).expect("clicks").hit, None);
        assert_eq!(click(&mut session, -40.0, -40.0, 600, 400).expect("clicks").hit, None);
    }

    #[test]
    fn a_non_finite_click_is_refused() {
        let mut session = craftkit_engine::Session::sample();
        let err = click(&mut session, f32::NAN, 0.0, 600, 400).expect_err("refused");
        assert!(err.contains("not a point"), "{err}");
        assert!(click(&mut session, f32::INFINITY, 0.0, 600, 400).is_err());
    }

    #[test]
    fn a_click_does_not_change_the_document_either() {
        let mut session = craftkit_engine::Session::sample();
        let before = session.snapshot_json();
        let frame = render(&mut session, 600, 400).expect("renders");
        let name = frame.automation.names().first().copied().expect("a widget");
        let rect = frame.automation.rect(name).expect("a rectangle");
        let _ = click(&mut session, rect.center().x, rect.center().y, 600, 400).expect("clicks");
        // A click reports where it landed; it does not activate. Acting is
        // `engine.execute`, which is undoable and auditable. A click that
        // changed the document would be a second way to do everything.
        assert_eq!(session.snapshot_json(), before, "a click must not edit anything");
    }

    #[test]
    fn the_png_has_a_png_signature() {
        let png = to_png(&Image::new(4, 4)).expect("encodes");
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a], "not a PNG");
    }

    #[test]
    fn the_png_has_the_dimensions_it_was_asked_for() {
        let png = to_png(&Image::new(37, 11)).expect("encodes");
        // IHDR width and height are big-endian u32 at a fixed offset.
        assert_eq!(&png[16..20], &[0, 0, 0, 37]);
        assert_eq!(&png[20..24], &[0, 0, 0, 11]);
    }

    #[test]
    fn an_image_starts_transparent() {
        assert!(Image::new(2, 2).pixels.iter().all(|b| *b == 0));
    }

    #[test]
    fn blending_never_goes_out_of_range() {
        let mut image = Image::new(1, 1);
        image.blend(0, 0, [255, 255, 255, 255], 0.5);
        image.blend(0, 0, [255, 255, 255, 255], 2.0);
        // An over-driven blend saturates at full white rather than wrapping.
        assert_eq!(&image.pixels[..3], &[255, 255, 255]);
        // Out of bounds is a no-op rather than a panic.
        image.blend(99, 99, [255, 255, 255, 255], 1.0);
        assert_eq!(image.pixels.len(), 4, "an out-of-range blend must not grow the image");
    }
}
