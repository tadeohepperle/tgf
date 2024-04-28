use std::{sync::Arc};

use std::rc::Rc;

use crate::{Aabb, AlphaSdfParams, BindableTexture, Color};

use glam::{DVec2, Vec2};
use smallvec::{smallvec, SmallVec};

use crate::ui::{
    element_id::ElementId,
    element_store::{ElementBox, ElementWithComputed, IntoElementBox},
    layout::GlyphBoundsAndUv,
    SdfFont,
};

#[repr(C)]
pub enum Element {
    Div(Div),
    Text(Text),
}

#[derive(Debug, Default)]
pub struct Div {
    style: DivStyle,
    pub children: SmallVec<[ElementBox; 4]>,
}

impl std::ops::Deref for Div {
    type Target = DivStyle;

    fn deref(&self) -> &Self::Target {
        &self.style
    }
}

impl std::ops::DerefMut for Div {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.style
    }
}

pub fn div() -> Div {
    Div::default()
}

/// vertical fill
#[inline]
pub fn v_fill(px: f64) -> Element {
    Element::Div(div().style(|s| {
        s.height = Some(Len::Px(px));
    }))
}

/// horizontal fill
#[inline]
pub fn h_fill(px: f64) -> Element {
    Element::Div(div().style(|s| {
        s.width = Some(Len::Px(px));
    }))
}

/// A quick element for debugging
pub fn red_box() -> Element {
    Element::Div(div().style(|s| {
        s.color = Color::RED;
        s.width = Some(Len::Px(96.0));
        s.height = Some(Len::Px(48.0));
        s.border.width = 2.0;
        s.border.color = Color::WHITE
    }))
}

impl Div {
    /// Allocates the div in the threadlocal slab allocator.
    pub fn store(self) -> ElementBox {
        Element::Div(self).store()
    }

    pub fn element(self) -> Element {
        Element::Div(self)
    }

    pub fn store_with_id(self, id: impl Into<ElementId>) -> ElementBox {
        Element::Div(self).store_with_id(id)
    }

    pub fn style(mut self, f: impl FnOnce(&mut DivStyle)) -> Self {
        f(&mut self.style);
        self
    }

    pub fn child(mut self, child: impl IntoElementBox) -> Self {
        self.children.push(child.store());
        self
    }

    pub fn child_with_id(mut self, id: impl Into<ElementId>, child: impl IntoElementBox) -> Self {
        self.children.push(child.store_with_id(id));
        self
    }

    pub fn child_box(mut self, child: ElementBox) -> Self {
        self.children.push(child);
        self
    }

    /// full transparent and absolute. good for overlays
    pub fn full(mut self) -> Self {
        self.style.height = Some(Len::FULL);
        self.style.width = Some(Len::FULL);
        self.style.absolute = Some(Vec2::ZERO);
        self.style.color = Color::TRANSPARENT;
        self
    }
}

#[derive(Debug)]
pub struct DivStyle {
    /// None means, the div has a non-fixed width, the children dictate the size of this div
    pub width: Option<Len>,
    /// None means, the div has a non-fixed height, the children dictate the size of this div
    pub height: Option<Len>,
    /// Determines how children are layed out: X = horizontally, Y = vertically.
    pub axis: Axis,
    pub main_align: MainAlign,
    pub cross_align: Align,
    /// Note: for padding in the `vert` crate we had `Edges<Len>` before, to allow for fractional padding,
    /// but most of the time it is not worth it. Requires some reverse logic to determine padding in px
    /// if own size depends on size of children.
    pub padding: Edges<f64>,
    /// the Vec2 should be in the unit square. (0,0) is the top right corner, (1,0) the top left corner and so on...
    pub absolute: Option<Vec2>,
    pub offset: DVec2,
    pub color: Color,
    pub border: DivBorder,
    pub texture: DivTexture,
    pub z_index: i16,
    pub shadow: DivShadow,
    /// gap is padding inserted *between* children of this div.
    /// If own size is wrapping children (width/height in main axis is None), this affects the size of the div.
    /// This avoids having to add placeholder v_fill() or h_fill() divs in lists of elements.
    ///
    /// Note: gap has no effect if `MainAlign::SpaceBetween`` or `MainAlign::SpaceAround`!
    pub gap: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct DivBorder {
    pub color: Color,
    pub radius: Corners<f32>,
    pub width: f32,
    pub softness: f32,
}

impl DivBorder {
    pub const ZERO: DivBorder = DivBorder {
        color: Color::TRANSPARENT,
        radius: Corners::all(0.0),
        width: 0.0,
        softness: 0.0,
    };
}

#[derive(Debug, Clone, Copy)]
pub struct DivShadow {
    pub color: Color,
    // an outer padding in each of the 4 directions.
    pub width: f32,
    // how intense a simple sdf shadow should be
    pub curve_param: f32,
}

impl DivShadow {
    pub const ZERO: DivShadow = DivShadow {
        color: Color::TRANSPARENT,
        width: 0.0,
        curve_param: 1.0,
    };
}

impl Default for DivStyle {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            axis: Axis::Y,
            padding: Default::default(),
            main_align: MainAlign::Start,
            cross_align: Align::Start,
            absolute: None,
            color: Color::TRANSPARENT,
            border: DivBorder::ZERO,
            offset: DVec2::ZERO,
            texture: DivTexture::None,
            z_index: 0,
            shadow: DivShadow::ZERO,
            gap: 0.0,
        }
    }
}

impl DivStyle {
    pub fn texture(&mut self, region: TextureRegion) {
        self.texture = DivTexture::Texture(region);
    }

    pub fn alpha_sdf(&mut self, region: TextureRegion, params: AlphaSdfParams) {
        self.texture = DivTexture::AlphaSdfTexture(SdfTextureRegion { region, params });
    }

    pub fn size(&mut self, w: u32, h: u32) {
        self.width = Some(Len::Px(w as f64));
        self.height = Some(Len::Px(h as f64));
    }

    pub fn center(&mut self) {
        self.main_align = MainAlign::Center;
        self.cross_align = Align::Center;
    }
}

#[derive(Debug, Clone)]
pub enum DivTexture {
    None,
    /// RGBA texture
    Texture(TextureRegion),
    /// RGBA texture where the alpha channel stores sdf information.
    AlphaSdfTexture(SdfTextureRegion),
}

#[derive(Debug, Clone)]
pub struct SdfTextureRegion {
    pub region: TextureRegion,
    pub params: AlphaSdfParams,
}

#[derive(Debug, Clone)]
pub struct TextureRegion {
    pub texture: Rc<BindableTexture>,
    pub uv: Aabb,
}

impl TextureRegion {
    pub fn scale(mut self, factor: f32) -> TextureRegion {
        self.uv = self.uv.scale(factor);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Len {
    Px(f64),
    Fraction(f64),
}

impl Len {
    pub const ZERO: Len = Len::Px(0.0);
    pub const FULL: Len = Len::Fraction(1.0);

    pub fn fixed(&self, full_fraction_px: f64) -> f64 {
        match self {
            Len::Px(px) => *px,
            Len::Fraction(f) => *f * full_fraction_px,
        }
    }
}

impl Default for Len {
    fn default() -> Self {
        Len::ZERO
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    #[default]
    Y,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MainAlign {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Corners<T> {
    pub top_left: T,
    pub top_right: T,
    pub bottom_right: T,
    pub bottom_left: T,
}

impl<T: Copy> Corners<T> {
    pub const fn all(value: T) -> Self {
        Corners {
            top_left: value,
            top_right: value,
            bottom_right: value,
            bottom_left: value,
        }
    }
}
unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for Corners<T> {}
unsafe impl<T: bytemuck::Pod> bytemuck::Pod for Corners<T> {}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Edges<T> {
    pub left: T,
    pub right: T,
    pub top: T,
    pub bottom: T,
}
impl<T: Copy> Edges<T> {
    pub const fn all(value: T) -> Self {
        Edges {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    pub const fn horizontal(mut self, value: T) -> Self {
        self.left = value;
        self.right = value;
        self
    }

    pub const fn vertical(mut self, value: T) -> Self {
        self.top = value;
        self.bottom = value;
        self
    }

    pub const fn top(mut self, value: T) -> Self {
        self.top = value;
        self
    }

    pub const fn bottom(mut self, value: T) -> Self {
        self.bottom = value;
        self
    }

    pub const fn left(mut self, value: T) -> Self {
        self.left = value;
        self
    }

    pub const fn right(mut self, value: T) -> Self {
        self.right = value;
        self
    }
}

unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for Edges<T> {}
unsafe impl<T: bytemuck::Pod> bytemuck::Pod for Edges<T> {}

#[derive(Debug)]
pub struct Text {
    pub sections: SmallVec<[Section; 1]>,
    pub offset: DVec2,
    pub additional_line_gap: f32,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            sections: Default::default(),
            offset: Default::default(),
            additional_line_gap: 0.0,
        }
    }
}

impl Text {
    pub fn element_sections_mut(&mut self) -> impl Iterator<Item = &mut ElementWithComputed> {
        self.sections.iter_mut().filter_map(|s| match s {
            Section::Element { element, .. } => Some(element.element_mut()),
            Section::Text(_) => None,
        })
    }

    pub fn element_sections(&self) -> impl Iterator<Item = &ElementWithComputed> {
        self.sections.iter().filter_map(|s| match s {
            Section::Element { element, .. } => Some(element.element()),
            Section::Text(_) => None,
        })
    }
}

#[derive(Debug)]
pub enum Section {
    Text(TextSection),
    Element {
        element: ElementBox,
        sets_line_height: bool,
    },
}

impl From<Div> for Element {
    fn from(value: Div) -> Self {
        Element::Div(value)
    }
}

impl From<Text> for Element {
    fn from(value: Text) -> Self {
        Element::Text(value)
    }
}

impl From<TextSection> for Element {
    fn from(value: TextSection) -> Self {
        Element::Text(Text {
            sections: smallvec![Section::Text(value)],
            offset: DVec2::ZERO,
            additional_line_gap: 0.0,
        })
    }
}

#[derive(Debug, Clone)]
pub enum UiString {
    Static(&'static str),
    String(String),
    Arc(Arc<str>),
    Rc(Rc<str>),
}

impl std::ops::Deref for UiString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            UiString::Static(s) => *s,
            UiString::String(s) => s,
            UiString::Arc(s) => s,
            UiString::Rc(s) => s,
        }
    }
}

impl AsRef<str> for UiString {
    fn as_ref(&self) -> &str {
        match self {
            UiString::Static(s) => *s,
            UiString::String(s) => s,
            UiString::Arc(s) => s,
            UiString::Rc(s) => s,
        }
    }
}

impl From<&'static str> for UiString {
    fn from(value: &'static str) -> Self {
        UiString::Static(value)
    }
}

impl From<String> for UiString {
    fn from(value: String) -> Self {
        UiString::String(value)
    }
}

impl From<Arc<str>> for UiString {
    fn from(value: Arc<str>) -> Self {
        UiString::Arc(value)
    }
}

impl From<Rc<str>> for UiString {
    fn from(value: Rc<str>) -> Self {
        UiString::Rc(value)
    }
}

#[derive(Debug, Clone)]
pub struct TextSection {
    pub string: UiString,
    pub font: Rc<SdfFont>,
    pub color: Color,
    pub font_size: f32,
    pub shadow_intensity: f32,
}

pub enum ElementSection {}

pub trait ElementT {
    type Computed: 'static + Default;
}

impl ElementT for Div {
    type Computed = DivComputed;
}

impl ElementT for Text {
    type Computed = TextComputed;
}

#[derive(Debug, Clone, Default)]
pub struct DivComputed {
    pub bounds: ComputedBounds,
    pub content_size: DVec2,
}

#[derive(Debug, Default)]
pub struct TextComputed {
    pub bounds: ComputedBounds,
    /// Should have the same length as the number of text-sections in this text. Should point to ranges of the glyphs vec below.
    pub text_section_glyphs: SmallVec<[std::ops::Range<usize>; 2]>,
    pub glyphs: Vec<GlyphBoundsAndUv>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ComputedBounds {
    pub pos: DVec2,
    pub size: DVec2,
}

impl ComputedBounds {
    pub fn contains(&self, point: &DVec2) -> bool {
        point.x >= self.pos.x
            && point.y >= self.pos.y
            && point.x <= self.pos.x + self.size.x
            && point.y <= self.pos.y + self.size.y
    }
}
