use crate::{Input, MouseButtonState, PhysicalSize, PressState};
use etagere::euclid::default;
use glam::{dvec2, DVec2, Vec2};

use crate::ui::{
    batching::ElementBatches,
    div,
    element::{ComputedBounds, Element},
    element_id::ElementId,
    ElementBox, IntoElementBox,
};

use super::layout::ComputedBoundsVisitor;

/// Use this as a `&mut impl ComputedBoundsVisitor` in layout functions at the end of each frame
/// to fill the `id_bounds` buffer with valuable bounds information. Make sure to reset this buffer
/// every frame, before layout.
#[derive(Debug, Clone)]
pub struct ElementContext {
    // this contains the elements roughly in z-order when constructed in
    // a `StoredElement::set_position()` pass. That means, that children, come first, then their parents. Explicit z index is not regarded here...
    // To find the first element hit by a mouse cursor, search from front to back.
    id_bounds: Vec<(ElementId, ComputedBounds)>,
    interaction_state: InteractionState<ElementId>,
}

impl ElementContext {
    pub fn new() -> Self {
        ElementContext {
            id_bounds: vec![],
            interaction_state: InteractionState::default(),
        }
    }

    #[inline(always)]
    pub fn clear_id_bounds(&mut self) {
        self.id_bounds.clear()
    }

    #[inline(always)]
    pub fn hot_state(&self) -> HotState<ElementId> {
        self.interaction_state.hot_state
    }

    #[inline(always)]
    pub fn state(&self) -> InteractionState<ElementId> {
        self.interaction_state
    }
    #[inline(always)]
    pub fn state_of(&self, id: ElementId) -> Interaction {
        self.interaction_state.of(id)
    }

    #[inline(always)]
    pub fn start_frame_scaled_to_fixed_height(
        &mut self,
        cursor_pos: DVec2,
        mouse: MouseButtonState,
        screen_px_size: PhysicalSize<u32>,
        fixed_layout_height: f64,
    ) {
        let cursor_pos = cursor_pos * fixed_layout_height / screen_px_size.height as f64;
        self.start_frame(cursor_pos, mouse);
    }

    /// Note: cursor_pos needs to be in layout space, which could be different from the pixel space on screen.
    pub fn start_frame(&mut self, cursor_pos: DVec2, mouse: MouseButtonState) {
        // find element hovered:
        let hovered = self.hovered_element(&cursor_pos);
        let left_mouse_down = mouse.left().pressed();
        self.interaction_state.transition(hovered, left_mouse_down);
    }

    pub fn hovered_element(&self, cursor_pos: &DVec2) -> Option<ElementId> {
        for (id, bounds) in self.id_bounds.iter() {
            if bounds.contains(cursor_pos) {
                return Some(*id);
            }
        }
        None
    }
}

pub struct IdElementBounds {}
impl ComputedBoundsVisitor for ElementContext {
    fn visit(&mut self, id: ElementId, computed_bounds: &ComputedBounds) {
        if !id.is_none() {
            self.id_bounds.push((id, *computed_bounds));
        }
    }
}

// #[deprecated]
// #[derive(Debug, Clone)]
// pub struct ElementContext {
//     pub mouse_buttons: MouseButtonState,
//     pub scroll: f32,
//     pub cursor_pos: Vec2,
//     pub cursor_delta: Vec2,
//     pub hot_active: HotActiveElement,
// }

// impl Default for ElementContext {
//     fn default() -> Self {
//         Self::new()
//     }
// }

// impl ElementContext {
//     pub fn hot_active(&self, id: impl Into<ElementId>) -> HotActive {
//         let id: ElementId = id.into();
//         match self.hot_active {
//             HotActiveElement::Hot(i) if i == id => HotActive::Hot,
//             HotActiveElement::Active(i) if i == id => HotActive::Active,
//             _ => HotActive::None,
//         }
//     }

//     // determines what hot_active state the element should be in next state and if it was clicked (Active -> Hot while in bounds).
//     pub fn btn_hot_active(&mut self, id: impl Into<ElementId>) -> (HotActive, bool) {
//         let id: ElementId = id.into();
//         let hot_active = self.hot_active(id);
//         let next = next_hot_active(hot_active, self.is_hovered(id), self.mouse_buttons.left());
//         self.set_hot_active(id, next);
//         let clicked = hot_active == HotActive::Active && next == HotActive::Hot;
//         (next, clicked)
//     }

//     pub fn is_hovered(&self, id: impl Into<ElementId>) -> bool {
//         let id: ElementId = id.into();
//         let Some(bounds) = ElementStore::get_computed_bounds(&id) else {
//             return false;
//         };
//         bounds.contains(&self.cursor_pos.as_dvec2())
//     }

//     /// useful for overlay ui in games, to not check for camera click raycasts into the scene ]
//     /// if some part of the ui is hovered in front of it
//     pub fn any_element_with_id_hovered(&self) -> bool {
//         ElementStore::any_element_with_id_hovered(self.cursor_pos.as_dvec2())
//     }

//     pub fn get_computed_bounds(&self, id: impl Into<ElementId>) -> Option<ComputedBounds> {
//         let id: ElementId = id.into();
//         ElementStore::get_computed_bounds(&id)
//     }

//     pub fn new() -> Self {
//         ElementContext {
//             cursor_pos: Vec2::MAX,
//             hot_active: HotActiveElement::None,
//             mouse_buttons: Default::default(),
//             scroll: 0.0,
//             cursor_delta: Vec2::ZERO,
//         }
//     }

//     pub fn set_input(&mut self, input: &Input) {
//         self.cursor_delta = input.cursor_delta();
//         self.mouse_buttons = input.mouse_buttons();
//         self.cursor_pos = input.cursor_pos();
//     }

//     /// Use this, if we layout the UI always at a fixed height, but scale it up by some factor in the shader
//     /// to match the actual screen resolution
//     ///
//     /// `input` is taken at screen resolution.
//     /// `screen_size` is the actual screen resolution.
//     /// `fixed_height` is the height of our ui layout. (width is calculated to be proportional to the screen_size, both scaled up to screen_size in rendering later).
//     pub fn set_input_scaled_to_fixed_height(
//         &mut self,
//         input: &Input,
//         screen_size: PhysicalSize<u32>,
//         fixed_height: f32,
//     ) {
//         let scale_factor = fixed_height / screen_size.height as f32;
//         self.mouse_buttons = input.mouse_buttons();
//         self.cursor_pos = input.cursor_pos() * scale_factor;
//         self.cursor_delta = input.cursor_delta() * scale_factor;
//     }

//     pub fn set_cursor_delta(&mut self, cursor_delta: Vec2) {
//         self.cursor_delta = cursor_delta
//     }

//     pub fn set_mouse_buttons(&mut self, mouse_buttons: MouseButtonState) {
//         self.mouse_buttons = mouse_buttons
//     }

//     pub fn set_cursor_pos(&mut self, cursor_pos: Vec2) {
//         self.cursor_pos = cursor_pos;
//     }

//     pub fn set_hot_active(&mut self, id: ElementId, state: HotActive) {
//         match state {
//             HotActive::None => {
//                 // dont allow change to none if currently other item is hot or active
//                 if matches!(self.hot_active, HotActiveElement::Hot(i) | HotActiveElement::Active(i) if i != id)
//                 {
//                     return;
//                 }
//                 self.hot_active = HotActiveElement::None;
//             }
//             HotActive::Hot => self.hot_active = HotActiveElement::Hot(id),
//             HotActive::Active => self.hot_active = HotActiveElement::Active(id),
//         }
//     }
// }

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum HotState<T: std::fmt::Debug + Clone + Copy + PartialEq> {
    #[default]
    None,
    Hot(T),
    Active(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Interaction {
    pub hot_active: HotActive,
    pub hovered: bool,
    pub just_started_click: bool,
    pub just_ended_click: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InteractionState<T: std::fmt::Debug + Clone + Copy + PartialEq> {
    pub hot_state: HotState<T>,
    pub hovered: Option<T>,
    pub just_started_click: Option<T>,
    pub just_ended_click: Option<T>,
}

impl<T: std::fmt::Debug + Clone + Copy + PartialEq> InteractionState<T> {
    pub fn transition(&mut self, hovered: Option<T>, mouse_down: bool) {
        self.hovered = hovered;
        self.just_started_click = None;
        self.just_ended_click = None;
        self.hot_state.transition(
            hovered,
            mouse_down,
            &mut self.just_started_click,
            &mut self.just_ended_click,
        )
    }

    #[inline(always)]
    pub fn of(&self, id: T) -> Interaction {
        Interaction {
            hot_active: self.hot_state.hot_active(id),
            just_started_click: self.just_started_click == Some(id),
            just_ended_click: self.just_ended_click == Some(id),
            hovered: self.hovered == Some(id),
        }
    }
}

impl<T: std::fmt::Debug + Clone + Copy + PartialEq> Default for InteractionState<T> {
    fn default() -> Self {
        Self {
            hot_state: HotState::None,
            hovered: None,
            just_started_click: None,
            just_ended_click: None,
        }
    }
}

impl<T: std::fmt::Debug + Clone + Copy + PartialEq> HotState<T> {
    pub fn hot_active(&self, id: T) -> HotActive {
        match self {
            HotState::Hot(i) if *i == id => HotActive::Hot,
            HotState::Active(i) if *i == id => HotActive::Active,
            _ => HotActive::None,
        }
    }

    pub fn is(&self, id: T) -> bool {
        matches!(self, HotState::Hot(i) | HotState::Active(i) if *i == id)
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        matches!(self, HotState::None)
    }

    pub fn transition(
        &mut self,
        hovered: Option<T>,
        mouse_down: bool,
        just_started_click: &mut Option<T>,
        just_ended_click: &mut Option<T>,
    ) {
        match self {
            HotState::None => {
                if let Some(hovered) = hovered {
                    if mouse_down {
                        *self = HotState::Active(hovered);
                    } else {
                        *self = HotState::Hot(hovered);
                    }
                }
            }
            HotState::Hot(_) => {
                if let Some(hovered) = hovered {
                    if mouse_down {
                        *just_started_click = Some(hovered);
                        *self = HotState::Active(hovered);
                    } else {
                        *self = HotState::Hot(hovered);
                    }
                } else {
                    *self = HotState::None;
                }
            }
            HotState::Active(_) => {
                if !mouse_down {
                    if let Some(hovered) = hovered {
                        *just_ended_click = Some(hovered);
                        *self = HotState::Hot(hovered);
                    } else {
                        *self = HotState::None;
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum HotActive {
    #[default]
    None,
    /// Means: Hovered
    Hot,
    /// Means: Clicked
    Active,
}

pub trait IntoElement {
    fn into_element(&mut self, ctx: &mut ElementContext) -> Element;
}

impl<F> IntoElement for F
where
    F: FnMut(&mut ElementContext) -> Element,
{
    fn into_element(&mut self, ctx: &mut ElementContext) -> Element {
        self(ctx)
    }
}

impl IntoElement for () {
    fn into_element(&mut self, _ctx: &mut ElementContext) -> Element {
        Element::Div(div())
    }
}

#[derive(Debug)]
pub struct Board {
    pub ctx: ElementContext,
    pub size: DVec2,
    pub pos_offset: DVec2,
    pub element: ElementBox,
    pub batches: ElementBatches,
}

impl Board {
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.size = dvec2(size.width as f64, size.height as f64);
    }

    /// resizes, to get the right proportion from `size`, but will always keep the same fixed height.
    /// e.g. if we set that the height should always be 1080px, this will never changed.
    /// So if input size is 2k (2560x1440) px, the inner height will stay 1080px and the width will be
    /// set to 1920px because this reflects the same 16:9 screen ratio
    pub fn resize_scaled_to_fixed_height(&mut self, size: PhysicalSize<u32>) {
        self.size.x = size.width as f64 / size.height as f64 * self.size.y;
    }

    pub fn resize_dvec2(&mut self, size: DVec2) {
        self.size = size;
    }

    pub fn set_element(&mut self, element: ElementBox) {
        self.element = element;
        self.ctx.clear_id_bounds();
        self.element
            .layout_in_size(self.size, self.pos_offset, &mut self.ctx);
        self.batches = self.element.element.get_batches();
    }

    // pub fn render(&mut self, element: &mut impl IntoElement) {
    //     self.element = element.into_element(&mut self.ctx).store();
    //     self.element
    //         .layout_in_size(dvec2(self.size.width as f64, self.size.height as f64));
    //     self.batches = self.element.element.get_batches();
    // }

    pub fn new(mut element: ElementBox, size: DVec2) -> Self {
        let pos_offset = DVec2::ZERO;
        let mut ctx = ElementContext::new();
        element.layout_in_size(size, pos_offset, &mut ctx);
        let batches = element.element.get_batches();
        Board {
            ctx,
            element,
            batches,
            size,
            pos_offset,
        }
    }
}

/// Shout out to Casey Muratori, our lord and savior. (See this Video as well for an exmplanation: https://www.youtube.com/watch?v=geZwWo-qNR4)
pub fn next_hot_active(
    hot_active: HotActive,
    mouse_in_rect: bool,
    button_press: PressState,
) -> HotActive {
    use HotActive::*;

    match hot_active {
        None => {
            if mouse_in_rect {
                Hot
            } else {
                None
            }
        }
        Hot => {
            if mouse_in_rect {
                if button_press.just_pressed() {
                    Active
                } else {
                    Hot
                }
            } else {
                None
            }
        }
        Active => {
            if button_press.just_released() {
                if mouse_in_rect {
                    Hot
                } else {
                    None
                }
            } else {
                Active
            }
        }
    }
}
