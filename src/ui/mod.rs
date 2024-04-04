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
