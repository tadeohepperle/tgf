use crate::{Input, MouseButtonState, PhysicalSize, PressState};
use glam::{dvec2, DVec2, Vec2};

use crate::ui::{
    batching::ElementBatches,
    div,
    element::{ComputedBounds, Element},
    element_id::ElementId,
    ElementBox, ElementStore, IntoElementBox,
};

#[derive(Debug, Clone)]
pub struct ElementContext {
    pub mouse_buttons: MouseButtonState,
    pub scroll: f32,
    pub cursor_pos: Vec2,
    pub cursor_delta: Vec2,
    pub hot_active: HotActiveElement,
}

impl Default for ElementContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ElementContext {
    pub fn hot_active(&self, id: impl Into<ElementId>) -> HotActive {
        let id: ElementId = id.into();
        match self.hot_active {
            HotActiveElement::Hot(i) if i == id => HotActive::Hot,
            HotActiveElement::Active(i) if i == id => HotActive::Active,
            _ => HotActive::None,
        }
    }

    // determines what hot_active state the element should be in next state and if it was clicked (Active -> Hot while in bounds).
    pub fn btn_hot_active(&mut self, id: impl Into<ElementId>) -> (HotActive, bool) {
        let id: ElementId = id.into();
        let hot_active = self.hot_active(id);
        let next = next_hot_active(hot_active, self.is_hovered(id), self.mouse_buttons.left());
        self.set_hot_active(id, next);
        let clicked = hot_active == HotActive::Active && next == HotActive::Hot;
        (next, clicked)
    }

    pub fn is_hovered(&self, id: impl Into<ElementId>) -> bool {
        let id: ElementId = id.into();
        let Some(bounds) = ElementStore::get_computed_bounds(&id) else {
            return false;
        };
        bounds.contains(&self.cursor_pos.as_dvec2())
    }

    /// useful for overlay ui in games, to not check for camera click raycasts into the scene ]
    /// if some part of the ui is hovered in front of it
    pub fn any_element_with_id_hovered(&self) -> bool {
        ElementStore::any_element_with_id_hovered(self.cursor_pos.as_dvec2())
    }

    pub fn get_computed_bounds(&self, id: impl Into<ElementId>) -> Option<ComputedBounds> {
        let id: ElementId = id.into();
        ElementStore::get_computed_bounds(&id)
    }

    pub fn new() -> Self {
        ElementContext {
            cursor_pos: Vec2::MAX,
            hot_active: HotActiveElement::None,
            mouse_buttons: Default::default(),
            scroll: 0.0,
            cursor_delta: Vec2::ZERO,
        }
    }

    pub fn set_input(&mut self, input: &Input) {
        self.cursor_delta = input.cursor_delta();
        self.mouse_buttons = input.mouse_buttons();
        self.cursor_pos = input.cursor_pos();
    }

    /// Use this, if we layout the UI always at a fixed height, but scale it up by some factor in the shader
    /// to match the actual screen resolution
    ///
    /// `input` is taken at screen resolution.
    /// `screen_size` is the actual screen resolution.
    /// `fixed_height` is the height of our ui layout. (width is calculated to be proportional to the screen_size, both scaled up to screen_size in rendering later).
    pub fn set_input_scaled_to_fixed_height(
        &mut self,
        input: &Input,
        screen_size: PhysicalSize<u32>,
        fixed_height: f32,
    ) {
        let scale_factor = fixed_height / screen_size.height as f32;
        self.mouse_buttons = input.mouse_buttons();
        self.cursor_pos = input.cursor_pos() * scale_factor;
        self.cursor_delta = input.cursor_delta() * scale_factor;
    }

    pub fn set_cursor_delta(&mut self, cursor_delta: Vec2) {
        self.cursor_delta = cursor_delta
    }

    pub fn set_mouse_buttons(&mut self, mouse_buttons: MouseButtonState) {
        self.mouse_buttons = mouse_buttons
    }

    pub fn set_cursor_pos(&mut self, cursor_pos: Vec2) {
        self.cursor_pos = cursor_pos;
    }

    pub fn set_hot_active(&mut self, id: ElementId, state: HotActive) {
        match state {
            HotActive::None => {
                // dont allow change to none if currently other item is hot or active
                if matches!(self.hot_active, HotActiveElement::Hot(i) | HotActiveElement::Active(i) if i != id)
                {
                    return;
                }
                self.hot_active = HotActiveElement::None;
            }
            HotActive::Hot => self.hot_active = HotActiveElement::Hot(id),
            HotActive::Active => self.hot_active = HotActiveElement::Active(id),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum HotActiveElement {
    None,
    Hot(ElementId),
    Active(ElementId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HotActive {
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
    pub fn ctx(&mut self) -> &mut ElementContext {
        &mut self.ctx
    }

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
        self.element.layout_in_size(self.size, self.pos_offset);
        self.batches = self.element.element.get_batches();
    }

    // pub fn render(&mut self, element: &mut impl IntoElement) {
    //     self.element = element.into_element(&mut self.ctx).store();
    //     self.element
    //         .layout_in_size(dvec2(self.size.width as f64, self.size.height as f64));
    //     self.batches = self.element.element.get_batches();
    // }

    pub fn new(element: &mut impl IntoElement, size: DVec2) -> Self {
        let mut ctx = ElementContext::new();
        let mut element = element.into_element(&mut ctx).store();
        let pos_offset = DVec2::ZERO;
        element.layout_in_size(size, pos_offset);
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
