use std::rc::Rc;

use crate::{
    renderer::sdf_sprite::AlphaSdfParams, Aabb, BindableTexture, Color,
    GrowableBuffer, VertexT,
};
use wgpu::BufferUsages;

use crate::ui::{
    element::{ComputedBounds, DivComputed, SdfTextureRegion, Section, TextureRegion},
    layout::GlyphBoundsAndUv,
    Corners, Div, DivTexture, ElementWithComputed, SdfFont, TextSection,
};

use crate::utils::rc_addr_as_u64;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct RectRaw {
    pub bounds: Aabb,
    pub color: Color,
    pub border_radius: Corners<f32>,
    pub border_color: Color,
    // these are bundled together into another 16 byte chunk.
    border_width: f32,
    border_softness: f32,
    shadow_width: f32,
    shadow_curve: f32,
    shadow_color: Color,
}

impl VertexT for RectRaw {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x4, // "pos"
        wgpu::VertexFormat::Float32x4, // "color"
        wgpu::VertexFormat::Float32x4, // "border_radius"
        wgpu::VertexFormat::Float32x4, // "border_color"
        wgpu::VertexFormat::Float32x4, // "border_width", "border_softness", "shadow_width", "shadow_curve"
        wgpu::VertexFormat::Float32x4, // "shadow_color",
    ];
}

impl RectRaw {
    fn new(div: &Div, computed: &DivComputed) -> Self {
        RectRaw {
            bounds: bounds_from_computed(&computed.bounds),
            color: div.color,
            border_radius: div.border.radius,
            border_color: div.border.color,
            border_width: div.border.width,
            border_softness: div.border.softness,
            shadow_width: div.shadow.width,
            shadow_curve: div.shadow.curve_param,
            shadow_color: div.shadow.color,
        }
    }
}

#[inline(always)]
fn bounds_from_computed(computed: &ComputedBounds) -> Aabb {
    let pos = computed.pos.as_vec2();
    let size = computed.size.as_vec2();
    Aabb::new(pos, pos + size)
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct TexturedRectRaw {
    pub rect: RectRaw,
    pub uv: Aabb,
}

impl VertexT for TexturedRectRaw {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x4, // "pos"
        wgpu::VertexFormat::Float32x4, // "color"
        wgpu::VertexFormat::Float32x4, // "border_radius"
        wgpu::VertexFormat::Float32x4, // "border_color"
        wgpu::VertexFormat::Float32x4, // "border_width", "border_softness", "shadow_width", "shadow_curve"
        wgpu::VertexFormat::Float32x4, // "shadow_color",
        wgpu::VertexFormat::Float32x4, // "uv"
    ];
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct AlphaSdfRectRaw {
    pub bounds: Aabb,
    pub color: Color,
    pub params: AlphaSdfParams,
    pub uv: Aabb,
}

impl VertexT for AlphaSdfRectRaw {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x4, // "bounds"
        wgpu::VertexFormat::Float32x4, // "color"
        wgpu::VertexFormat::Float32x4, // "border_color"
        wgpu::VertexFormat::Float32x4, // "in_to_border_cutoff", "in_to_border_smooth", "border_to_out_cutoff", "border_to_out_smooth"
        wgpu::VertexFormat::Float32x4, // "uv"
    ];
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct GlyphRaw {
    pub bounds: Aabb,
    pub color: Color,
    pub uv: Aabb,
    pub shadow_intensity: f32,
}

impl VertexT for GlyphRaw {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x4, // "pos"
        wgpu::VertexFormat::Float32x4, // "color"
        wgpu::VertexFormat::Float32x4, // "uv"
        wgpu::VertexFormat::Float32,   // "shadow_intensity"
    ];
}

#[derive(Debug)]
pub struct Batch {
    /// Note: the key is not unique, it just describes what elements the batch is compatible with.
    /// There could be two batches with the same key in one `ElementBatches` object (but not directly next to each other)
    pub key: u64,
    pub range: std::ops::Range<usize>,
    pub kind: BatchKind,
}

#[derive(Debug)]
pub enum BatchKind {
    Rect,
    TexturedRect(Rc<BindableTexture>),
    AlphaSdfRect(Rc<BindableTexture>),
    Glyph(Rc<SdfFont>),
}

#[derive(Debug, Default)]
pub struct ElementBatches {
    pub rects: Vec<RectRaw>,
    pub textured_rects: Vec<TexturedRectRaw>,
    pub alpha_sdf_rects: Vec<AlphaSdfRectRaw>,
    pub glyphs: Vec<GlyphRaw>,
    pub batches: Vec<Batch>,
}

pub enum PrimElement<'a> {
    Rect(&'a (Div, DivComputed)),
    TexturedRect(&'a (Div, DivComputed), &'a TextureRegion),
    AlphaSdfRect(&'a (Div, DivComputed), &'a SdfTextureRegion),
    Text(&'a TextSection, &'a [GlyphBoundsAndUv]),
}

impl<'a> PrimElement<'a> {
    fn batch_key(&self) -> u64 {
        match self {
            PrimElement::Rect(_) => 0,
            PrimElement::TexturedRect(_, texture) => rc_addr_as_u64(&texture.texture),
            PrimElement::Text(text, _) => rc_addr_as_u64(&text.font),
            PrimElement::AlphaSdfRect(_, sdf_texture) => {
                rc_addr_as_u64(&sdf_texture.region.texture) ^ 21891209983212317
                // this is such that we do not confuse a key for a AlphaSdfRect with a key for a TexturedRect
            }
        }
    }
}

/// In the stacking order, this is the priority order:
/// - high z-index in front of low z-index
/// - text in front of rects, if z-index is the same
/// - children in front of parents, if both are rects with the same z-index
/// , followed by the fact if it is text or not, then if it is a chi
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackingLevel {
    z_index: i16,
    /// - 0 for divs
    /// - 1 for text
    /// - 1 for inline divs in text
    /// - 2 for text in inline divs
    text_level: u16,
    nesting_level: u16,
}

impl StackingLevel {
    pub fn new(z_index: i16, text_level: u16, nesting_level: u16) -> Self {
        StackingLevel {
            nesting_level,
            text_level,
            z_index,
        }
    }
    pub const ZERO: StackingLevel = StackingLevel {
        z_index: 0,
        text_level: 0,
        nesting_level: 0,
    };
}

impl PartialOrd for StackingLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.z_index.partial_cmp(&other.z_index) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.text_level.partial_cmp(&other.text_level) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.nesting_level.partial_cmp(&other.nesting_level)
    }
}

impl Ord for StackingLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.z_index.cmp(&other.z_index) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.text_level.cmp(&other.text_level) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.nesting_level.cmp(&other.nesting_level)
    }
}

impl ElementWithComputed {
    pub fn get_batches(&self) -> ElementBatches {
        get_batches(&[&self])
    }

    /// The `level` passed in is the level of the parent
    fn collect_prim_elements<'a>(
        &'a self,
        mut level: StackingLevel,
        prim_elements: &mut Vec<(StackingLevel, PrimElement<'a>)>,
    ) {
        level.nesting_level += 1;

        match self {
            ElementWithComputed::Div(div) => {
                level.z_index += div.0.z_index;

                // Note: elements with color = 0,0,0,0 will be discarded even if they have a colored border or shadow!!!
                if div.0.color != Color::TRANSPARENT {
                    let prim = match &div.0.texture {
                        DivTexture::None => PrimElement::Rect(div),
                        DivTexture::Texture(texture) => PrimElement::TexturedRect(div, texture),
                        DivTexture::AlphaSdfTexture(sdf_texture) => {
                            PrimElement::AlphaSdfRect(div, sdf_texture)
                        }
                    };

                    prim_elements.push((level, prim));
                }

                for ch in div.0.children.iter() {
                    ch.element().collect_prim_elements(level, prim_elements);
                }
            }
            ElementWithComputed::Text(text) => {
                level.text_level += 1;

                let mut i: usize = 0;
                for section in text.0.sections.iter() {
                    match section {
                        Section::Text(text_section) => {
                            let glyph_range = text.1.text_section_glyphs[i].clone();
                            i += 1;
                            let glyphs = &text.1.glyphs[glyph_range];
                            let prim = PrimElement::Text(text_section, glyphs);
                            prim_elements.push((level, prim));
                        }
                        Section::Element { element, .. } => {
                            element
                                .element()
                                .collect_prim_elements(level, prim_elements);
                        }
                    }
                }
            }
        }
    }
}

pub fn get_batches(elements: &[&ElementWithComputed]) -> ElementBatches {
    // step 1: create an array with pointers to all elements and their z-order:
    let mut prim_elements: Vec<(StackingLevel, PrimElement)> = vec![];
    for element in elements {
        element.collect_prim_elements(StackingLevel::ZERO, &mut prim_elements);
    }

    // step 2: sort the array by the stacking level, from back to forth, to render them in correct order:
    prim_elements.sort_by(|a, b| a.0.cmp(&b.0));

    // step 3: create actual badges by merging prim elements of the same type together into one batch:
    let mut rects: Vec<RectRaw> = vec![];
    let mut textured_rects: Vec<TexturedRectRaw> = vec![];
    let mut alpha_sdf_rects: Vec<AlphaSdfRectRaw> = vec![];
    let mut glyphs: Vec<GlyphRaw> = vec![];
    let mut batches: Vec<Batch> = vec![];

    for (_level, element) in prim_elements {
        let key = element.batch_key();

        let add_new_batch = match batches.last_mut() {
            Some(batch) => {
                if batch.key != key {
                    // incompatible, finish the last batch:
                    let batch_end = match batch.kind {
                        BatchKind::Rect => rects.len(),
                        BatchKind::TexturedRect(_) => textured_rects.len(),
                        BatchKind::Glyph(_) => glyphs.len(),
                        BatchKind::AlphaSdfRect(_) => alpha_sdf_rects.len(),
                    };
                    batch.range.end = batch_end;
                    true
                } else {
                    // compatible, no action needed
                    false
                }
            }
            None => true,
        };

        // add a new batch, if last batch in
        if add_new_batch {
            let batch = match &element {
                PrimElement::Rect(_) => Batch {
                    key,
                    range: rects.len()..rects.len(),
                    kind: BatchKind::Rect,
                },
                PrimElement::TexturedRect(_, texture) => Batch {
                    key,
                    range: textured_rects.len()..textured_rects.len(),
                    kind: BatchKind::TexturedRect(texture.texture.clone()),
                },
                PrimElement::AlphaSdfRect(_, sdf_texture) => Batch {
                    key,
                    range: alpha_sdf_rects.len()..alpha_sdf_rects.len(),
                    kind: BatchKind::AlphaSdfRect(sdf_texture.region.texture.clone()),
                },
                PrimElement::Text(section, _) => Batch {
                    key,
                    range: glyphs.len()..glyphs.len(),
                    kind: BatchKind::Glyph(section.font.clone()),
                },
            };
            batches.push(batch);
        }

        // add primitives to the respective arrays:
        match element {
            PrimElement::Rect((div, computed)) => {
                let rect = RectRaw::new(div, computed);
                rects.push(rect);
            }
            PrimElement::TexturedRect((div, computed), texture) => {
                let rect = RectRaw::new(div, computed);
                let textured_rect = TexturedRectRaw {
                    rect,
                    uv: texture.uv,
                };
                textured_rects.push(textured_rect);
            }
            PrimElement::AlphaSdfRect((div, computed), sdf_texture) => {
                let alpha_sdf_rect = AlphaSdfRectRaw {
                    bounds: bounds_from_computed(&computed.bounds),
                    color: div.color,
                    params: sdf_texture.params,
                    uv: sdf_texture.region.uv,
                };
                alpha_sdf_rects.push(alpha_sdf_rect);
            }
            PrimElement::Text(section, text_glyphs) => {
                for g in text_glyphs {
                    let glyph_raw = GlyphRaw {
                        bounds: g.bounds.into(),
                        color: section.color,
                        uv: g.uv,
                        shadow_intensity: section.shadow_intensity,
                    };
                    glyphs.push(glyph_raw);
                }
            }
        }
    }

    // finish the last batch:
    if let Some(batch) = batches.last_mut() {
        let batch_end = match batch.kind {
            BatchKind::Rect => rects.len(),
            BatchKind::TexturedRect(_) => textured_rects.len(),
            BatchKind::AlphaSdfRect(_) => alpha_sdf_rects.len(),
            BatchKind::Glyph(_) => glyphs.len(),
        };
        batch.range.end = batch_end;
    }

    ElementBatches {
        rects,
        textured_rects,
        glyphs,
        batches,
        alpha_sdf_rects,
    }
}

#[derive(Debug)]
pub struct ElementBatchesGR {
    pub rects: GrowableBuffer<RectRaw>,
    pub textured_rects: GrowableBuffer<TexturedRectRaw>,
    pub alpha_sdf_rects: GrowableBuffer<AlphaSdfRectRaw>,
    pub glyphs: GrowableBuffer<GlyphRaw>,
}

impl ElementBatchesGR {
    pub fn new(batches: &ElementBatches, device: &wgpu::Device) -> ElementBatchesGR {
        let rects: GrowableBuffer<RectRaw> =
            GrowableBuffer::new_from_data(device, BufferUsages::VERTEX, &batches.rects);
        let textured_rects =
            GrowableBuffer::new_from_data(device, BufferUsages::VERTEX, &batches.textured_rects);
        let alpha_sdf_rects =
            GrowableBuffer::new_from_data(device, BufferUsages::VERTEX, &batches.alpha_sdf_rects);
        let glyphs = GrowableBuffer::new_from_data(device, BufferUsages::VERTEX, &batches.glyphs);

        ElementBatchesGR {
            rects,
            textured_rects,
            glyphs,
            alpha_sdf_rects,
        }
    }

    pub fn prepare(
        &mut self,
        batches: &ElementBatches,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.rects.prepare(&batches.rects, device, queue);
        self.textured_rects
            .prepare(&batches.textured_rects, device, queue);
        self.glyphs.prepare(&batches.glyphs, device, queue);
    }
}
