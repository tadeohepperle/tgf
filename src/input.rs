use std::{
    fmt::Debug,
    ops::{BitOr, Index},
    path::PathBuf,
};

use glam::{vec2, Vec2};
use smallvec::SmallVec;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::ToRaw;

#[derive(Debug)]
pub struct Input {
    keys: KeyState,
    mouse_buttons: MouseButtonState,
    resized: Option<PhysicalSize<u32>>,
    close_requested: bool,
    cursor_just_moved: bool,
    cursor_just_entered: bool,
    cursor_just_left: bool,
    cursor_pos: Vec2,
    _last_frame_cursor_pos: Vec2,
    cursor_delta: Vec2,
    scroll: Option<f32>,
    dropped_file: Option<PathBuf>,
    hovered_file: Option<PathBuf>,
}

impl Input {
    pub fn receive_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Resized(new_size) => {
                self.resized = Some(*new_size);
            }
            WindowEvent::CloseRequested => {
                self.close_requested = true;
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let KeyEvent {
                    physical_key: PhysicalKey::Code(key),
                    state,
                    ..
                } = event
                {
                    self.keys.receive_element_state(*key, *state)
                }
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                self.cursor_just_moved = true;
                self.cursor_pos = vec2(position.x as f32, position.y as f32);
                self.cursor_delta = self.cursor_pos - self._last_frame_cursor_pos;
                // println!("new cursort pos: {}", self.cursor_pos);
            }
            WindowEvent::CursorEntered { device_id: _ } => {
                self.cursor_just_entered = true;
            }
            WindowEvent::CursorLeft { device_id: _ } => {
                self.cursor_just_left = true;
            }
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase: _,
            } => {
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(_right, down) => {
                        let scroll = self.scroll.get_or_insert(0.0);
                        *scroll += down;
                    }
                    winit::event::MouseScrollDelta::PixelDelta(_) => {
                        // Default::default()
                    }
                }
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                let button = match button {
                    winit::event::MouseButton::Left => MouseButton::Left,
                    winit::event::MouseButton::Right => MouseButton::Right,
                    winit::event::MouseButton::Middle => MouseButton::Middle,
                    winit::event::MouseButton::Back => MouseButton::Back,
                    winit::event::MouseButton::Forward => MouseButton::Forward,
                    winit::event::MouseButton::Other(_) => {
                        // ignore
                        return;
                    }
                };
                self.mouse_buttons.receive_state(button, *state);
            }
            // /////////////////////////////////////////////////////////////////////////////
            // Currently unused:
            // /////////////////////////////////////////////////////////////////////////////
            WindowEvent::Moved(_) => {}
            WindowEvent::Destroyed => {}
            WindowEvent::DroppedFile(path) => {
                self.dropped_file = Some(path.clone());
            }
            WindowEvent::HoveredFile(path) => {
                self.hovered_file = Some(path.clone());
            }
            WindowEvent::HoveredFileCancelled => {}
            WindowEvent::Focused(_) => {}
            WindowEvent::ModifiersChanged(_) => {}
            WindowEvent::Ime(_) => {}

            WindowEvent::TouchpadMagnify {
                device_id: _,
                delta: _,
                phase: _,
            } => {}
            WindowEvent::SmartMagnify { device_id: _ } => {}
            WindowEvent::TouchpadRotate {
                device_id: _,
                delta: _,
                phase: _,
            } => {}
            WindowEvent::TouchpadPressure {
                device_id: _,
                pressure: _,
                stage: _,
            } => {}
            WindowEvent::AxisMotion {
                device_id: _,
                axis: _,
                value: _,
            } => {}
            WindowEvent::Touch(_) => {}
            WindowEvent::ScaleFactorChanged {
                scale_factor: _,
                inner_size_writer: _,
            } => {
                // Default::default()
            }
            WindowEvent::ThemeChanged(_) => {}
            WindowEvent::Occluded(_) => {}
            WindowEvent::RedrawRequested => {}
            WindowEvent::ActivationTokenDone {
                serial: _,
                token: _,
            } => {}
        }
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

impl Input {
    pub fn new() -> Self {
        Input {
            keys: Default::default(),
            mouse_buttons: Default::default(),
            resized: Default::default(),
            close_requested: Default::default(),
            cursor_just_moved: Default::default(),
            cursor_just_entered: Default::default(),
            cursor_just_left: Default::default(),
            cursor_pos: Default::default(),
            cursor_delta: Default::default(),
            scroll: Default::default(),
            _last_frame_cursor_pos: Default::default(),
            dropped_file: None,
            hovered_file: None,
        }
    }

    pub fn end_frame(&mut self) {
        // println!("------ new frame: {}", self.cursor_delta.length());
        self.keys.clear_at_end_of_frame();
        self.mouse_buttons.clear_at_end_of_frame();
        self.resized = None;
        self.scroll = None;
        self.close_requested = false;
        self.cursor_just_entered = false;
        self.cursor_just_left = false;
        self.cursor_just_moved = false;
        self.cursor_delta = Vec2::ZERO;
        self._last_frame_cursor_pos = self.cursor_pos;
        self.dropped_file = None;
        self.hovered_file = None;
    }

    /// shorthand for `self.mouse_buttons.left().just_pressed()`
    pub fn left_click(&self) -> bool {
        self.mouse_buttons.left().just_pressed()
    }

    pub fn ctrl_z_pressed(&self) -> bool {
        self.keys().is_pressed(KeyCode::ControlLeft) && self.keys().just_pressed(KeyCode::KeyZ)
    }

    pub fn ctrl_s_pressed(&self) -> bool {
        self.keys().is_pressed(KeyCode::ControlLeft) && self.keys().just_pressed(KeyCode::KeyS)
    }

    pub fn ctrl_pressed(&self) -> bool {
        self.keys().is_pressed(KeyCode::ControlLeft)
    }

    /// shorthand for `self.mouse_buttons.left().just_released()`
    pub fn left_released(&self) -> bool {
        self.mouse_buttons.left().just_released()
    }

    /// shorthand for `self.mouse_buttons.right().just_pressed()`
    pub fn right_click(&self) -> bool {
        self.mouse_buttons.right().just_pressed()
    }

    pub fn shift_pressed(&self) -> bool {
        self.keys.is_pressed(KeyCode::ShiftLeft)
    }

    pub fn wasd_vec(&self) -> glam::Vec2 {
        let mut v = Vec2::ZERO;
        if self.keys.is_pressed(KeyCode::KeyW) {
            v.y += 1.0;
        }
        if self.keys.is_pressed(KeyCode::KeyS) {
            v.y -= 1.0;
        }
        if self.keys.is_pressed(KeyCode::KeyA) {
            v.x -= 1.0;
        }
        if self.keys.is_pressed(KeyCode::KeyD) {
            v.x += 1.0;
        }
        if v != Vec2::ZERO {
            v.normalize()
        } else {
            v
        }
    }

    pub fn space_shift_updown(&self) -> f32 {
        let mut v = 0.0;
        if self.keys.is_pressed(KeyCode::ShiftLeft) {
            v -= 1.0;
        }
        if self.keys.is_pressed(KeyCode::Space) {
            v += 1.0;
        }
        v
    }

    pub fn rf_updown(&self) -> f32 {
        let mut v = 0.0;
        if self.keys.is_pressed(KeyCode::KeyF) {
            v -= 1.0;
        }
        if self.keys.is_pressed(KeyCode::KeyR) {
            v += 1.0;
        }
        v
    }

    pub fn arrow_just_left_right(&self) -> i32 {
        if self.keys.just_pressed(KeyCode::ArrowLeft) {
            -1
        } else if self.keys.just_pressed(KeyCode::ArrowRight) {
            1
        } else {
            0
        }
    }

    pub fn arrow_vec(&self) -> glam::Vec2 {
        let mut v = Vec2::ZERO;
        if self.keys.is_pressed(KeyCode::ArrowUp) {
            v.y += 1.0;
        }
        if self.keys.is_pressed(KeyCode::ArrowDown) {
            v.y -= 1.0;
        }
        if self.keys.is_pressed(KeyCode::ArrowLeft) {
            v.x -= 1.0;
        }
        if self.keys.is_pressed(KeyCode::ArrowRight) {
            v.x += 1.0;
        }
        if v != Vec2::ZERO {
            v.normalize()
        } else {
            v
        }
    }

    pub fn hovered_file(&self) -> Option<&PathBuf> {
        self.hovered_file.as_ref()
    }

    pub fn dropped_file(&self) -> Option<&PathBuf> {
        self.dropped_file.as_ref()
    }

    pub fn close_requested(&self) -> bool {
        self.close_requested
    }

    pub fn cursor_just_moved(&self) -> bool {
        self.cursor_just_moved
    }

    pub fn cursor_just_entered(&self) -> bool {
        self.cursor_just_entered
    }

    pub fn cursor_just_left(&self) -> bool {
        self.cursor_just_left
    }

    #[inline(always)]
    pub fn cursor_pos(&self) -> Vec2 {
        self.cursor_pos
    }

    pub fn cursor_delta(&self) -> Vec2 {
        self.cursor_delta
    }

    pub fn resized(&self) -> Option<PhysicalSize<u32>> {
        self.resized
    }

    #[inline]
    pub fn keys(&self) -> &KeyState {
        &self.keys
    }

    pub fn mouse_buttons(&self) -> MouseButtonState {
        self.mouse_buttons
    }

    pub fn scroll(&self) -> Option<f32> {
        self.scroll
    }
}

#[derive(Debug, Clone, Default, Copy)]
pub struct MouseButtonState {
    buttons: [PressState; 5],
}

impl Index<MouseButton> for MouseButtonState {
    type Output = PressState;

    fn index(&self, index: MouseButton) -> &Self::Output {
        &self.buttons[index as usize]
    }
}

impl MouseButtonState {
    pub fn receive_state(&mut self, button: MouseButton, element_state: ElementState) {
        let button = button as usize;
        match element_state {
            ElementState::Released => {
                self.buttons[button] = PressState::JustReleased;
            }
            ElementState::Pressed => {
                self.buttons[button] = PressState::JustPressed;
            }
        }
    }

    pub fn clear_at_end_of_frame(&mut self) {
        for b in self.buttons.iter_mut() {
            if *b == PressState::JustPressed {
                *b = PressState::Pressed;
            }
            if *b == PressState::JustReleased {
                *b = PressState::Released
            }
        }
    }

    #[inline]
    pub fn left(&self) -> PressState {
        self.buttons[MouseButton::Left as usize]
    }

    #[inline]
    pub fn right(&self) -> PressState {
        self.buttons[MouseButton::Right as usize]
    }

    #[inline]
    pub fn middle(&self) -> PressState {
        self.buttons[MouseButton::Middle as usize]
    }

    #[inline]
    pub fn back(&self) -> PressState {
        self.buttons[MouseButton::Back as usize]
    }

    #[inline]
    pub fn forward(&self) -> PressState {
        self.buttons[MouseButton::Forward as usize]
    }
}

pub enum MouseButton {
    Left = 0,
    Right = 1,
    Middle = 2,
    Back = 3,
    Forward = 4,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PressState {
    #[default]
    Released = 0,
    Pressed = 1,
    JustPressed = 2,
    JustReleased = 3,
}

impl BitOr for PressState {
    type Output = PressState;

    fn bitor(self, rhs: Self) -> Self::Output {
        unsafe { std::mem::transmute((self as u8).max(rhs as u8)) }
    }
}

impl PressState {
    pub fn pressed(&self) -> bool {
        matches!(self, PressState::JustPressed | PressState::Pressed)
    }

    pub fn just_pressed(&self) -> bool {
        matches!(self, PressState::JustPressed)
    }

    pub fn released(&self) -> bool {
        matches!(self, PressState::JustReleased | PressState::Released)
    }

    pub fn just_released(&self) -> bool {
        matches!(self, PressState::JustReleased)
    }
}

#[derive(Debug, Clone, Default)]
pub struct KeyState {
    just_pressed: SmallVec<[KeyCode; 4]>,
    pressed: SmallVec<[KeyCode; 4]>,
    just_released: SmallVec<[KeyCode; 4]>,
}

impl KeyState {
    pub fn key(&self, key: KeyCode) -> PressState {
        if self.just_pressed.contains(&key) {
            PressState::JustPressed
        } else if self.pressed.contains(&key) {
            PressState::Pressed
        } else if self.just_released.contains(&key) {
            PressState::JustReleased
        } else {
            PressState::Released
        }
    }

    pub fn is_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }

    pub fn just_pressed(&self, key: KeyCode) -> bool {
        self.just_pressed.contains(&key)
    }

    pub fn just_released(&self, key: KeyCode) -> bool {
        self.just_released.contains(&key)
    }

    pub fn clear_at_end_of_frame(&mut self) {
        // A weird note: forgetting to clear these leads to performance drops from 1400 fps to about 300 fps.
        // Even though they don't seem to grow at all.
        // - Tadeo Hepperle, 2023-12-13
        self.just_pressed.clear();
        self.just_released.clear();
    }

    pub fn receive_element_state(&mut self, value: KeyCode, element_state: ElementState) {
        let pressed_already = self.pressed.contains(&value);
        match element_state {
            ElementState::Released => {
                if pressed_already {
                    // remove it from pressed:
                    self.pressed.retain(|e| *e != value);
                }
                self.just_released.push(value);
            }
            ElementState::Pressed => {
                self.just_pressed.push(value);
                self.pressed.push(value);
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct InputRaw {
    cursor_pos: Vec2,
}

impl ToRaw for Input {
    type Raw = InputRaw;
    fn to_raw(&self) -> Self::Raw {
        InputRaw {
            cursor_pos: self.cursor_pos,
        }
    }
}
