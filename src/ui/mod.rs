pub mod allocator;
pub mod batching;
pub mod element;
pub mod element_context;
pub mod element_id;
pub mod element_store;
pub mod font;
pub mod layout;

pub use element::{
    div, red_box, Align, Axis, Corners, Div, DivTexture, Edges, Element, Len, MainAlign,
    SdfTextureRegion, Text, TextSection, TextureRegion,
};
pub use element_context::{Board, ElementContext, IntoElement};
pub use element_id::ElementId;
pub use element_store::{ElementBox, ElementStore, ElementWithComputed, IntoElementBox};
pub use font::SdfFont;

pub use fontdue::{Font, FontSettings};

pub use batching::get_batches;
use glam::{dvec2, DVec2, Vec2};

pub const REFERENCE_SCREEN_SIZE_D: DVec2 = dvec2(1920.0, 1080.0); // this is the reference we design all ui for.
pub const REFERENCE_SCREEN_SIZE: Vec2 = Vec2 {
    x: REFERENCE_SCREEN_SIZE_D.x as f32,
    y: REFERENCE_SCREEN_SIZE_D.y as f32,
};
