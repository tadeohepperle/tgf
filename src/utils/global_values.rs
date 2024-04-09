use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::ops::Range;

use glam::Vec3;

use crate::egui::Context;
use crate::egui::{self, Slider};
use crate::Color;

use crate::YoloCell;

thread_local! {
    pub static GLOBAL_VALUES: YoloCell<GlobalValues> = YoloCell::new(GlobalValues::new());
}

// static GLOBAL_VALUES: LazyLock<Mutex<GlobalValues>> =
//     LazyLock::new(|| Mutex::new(GlobalValues::new()));

pub fn global_vals_get<T: EditableValue>(
    label: &'static str,
    lazy: impl Fn() -> (T, Option<T::Params>),
) -> T {
    GLOBAL_VALUES.with(|m| m.get_mut().lazy_get(label, lazy))
}

pub fn global_vals_show_only(label: &'static str, value: String) {
    GLOBAL_VALUES.with(|m| m.get_mut().set_show_only_val(label, value))
}

pub fn global_vals_window(ctx: &mut Context) {
    GLOBAL_VALUES.with(|m| m.get_mut().create_window(ctx));
}

#[macro_export]
macro_rules! edit {
    ($val: expr, $name:literal) => {{
        $crate::utils::global_vals_get($name, || ($val, None))
    }};
    ($val:expr, $params:expr, $name:literal) => {{
        $crate::utils::global_vals_get($name, || ($val, Some($params)))
    }};
}

#[macro_export]
macro_rules! show {
    ($val:expr) => {{
        $crate::utils::global_vals_show_only(stringify!($val), format!("{:?}", $val))
    }};
}

struct GlobalValues {
    // contains: data pointer, params data pointer, ptr to the EditableValue::edit fn, ptr to a fn that converts the data into a string.
    editable_values: BTreeMap<
        &'static str,
        (
            *mut (),
            *mut (),
            fn(*mut (), *const (), &mut egui::Ui),
            fn(*mut ()) -> String,
        ),
    >,
    show_only_values: BTreeMap<&'static str, String>,
}

unsafe impl Send for GlobalValues {}

impl GlobalValues {
    fn new() -> Self {
        GlobalValues {
            editable_values: BTreeMap::new(),
            show_only_values: BTreeMap::new(),
        }
    }

    fn lazy_get<T: EditableValue>(
        &mut self,
        label: &'static str,
        lazy: impl Fn() -> (T, Option<T::Params>),
    ) -> T {
        match self.editable_values.entry(label) {
            Entry::Vacant(vacant) => {
                let (t, t_params) = lazy();
                let t_params = t_params.unwrap_or_else(|| T::Params::default_params());

                let edit_fn: fn(&mut T, &T::Params, &mut egui::Ui) = <T as EditableValue>::edit;
                let edit_fn_punned: fn(*mut (), *const (), &mut egui::Ui) =
                    unsafe { std::mem::transmute(edit_fn) };

                let value_as_string_fn: fn(&T) -> String = <T as EditableValue>::value_as_string;
                let value_as_string_fn_punned: fn(*mut ()) -> String =
                    unsafe { std::mem::transmute(value_as_string_fn) };

                let data_ptr = Box::leak(Box::new(t.clone())) as *mut T as *mut ();
                let params_ptr = Box::leak(Box::new(t_params)) as *mut T::Params as *mut ();

                vacant.insert((
                    data_ptr,
                    params_ptr,
                    edit_fn_punned,
                    value_as_string_fn_punned,
                ));

                t
            }
            Entry::Occupied(occupied) => {
                let (data_ptr, _params_ptr, _edit_fn_ptr, _as_string_fn_ptr) = *occupied.get();
                let t_ref = unsafe { &*(data_ptr as *mut T) };
                t_ref.clone()
            }
        }
    }

    fn export_values(&self, path: &str) {
        let mut s = String::new();
        for (label, (data_ptr, _, _, as_string_fn_ptr)) in self.editable_values.iter() {
            let label = *label;
            let data_ptr = *data_ptr;
            let as_string_fn_ptr = *as_string_fn_ptr;

            let val_as_str = as_string_fn_ptr(data_ptr);

            s.push_str(label);
            s.push_str(": ");
            s.push_str(&val_as_str);
            s.push_str("\n");
        }

        std::fs::write(path, s).expect("should work");
    }

    fn create_window(&mut self, ctx: &mut Context) {
        egui::Window::new("Editable Global Values").show(ctx, |ui| {
            if ui.button("Export Values").clicked() {
                self.export_values("./editable_values_dump.txt");
            }

            for (label, (data_ptr, params_ptr, edit_fn_ptr, _)) in self.editable_values.iter_mut() {
                let label = *label;
                let data_ptr = *data_ptr;
                let params_ptr = *params_ptr;
                let edit_fn_ptr = *edit_fn_ptr;

                ui.separator();
                ui.label(label);
                edit_fn_ptr(data_ptr, params_ptr, ui);
            }

            for (label, val) in self.show_only_values.iter() {
                ui.label(format!("{label}: {val}"));
            }
        });
        self.show_only_values.clear();
    }

    fn set_show_only_val(&mut self, label: &'static str, value: String) {
        self.show_only_values.insert(label, value);
    }
}

pub trait EditableValue: std::fmt::Debug + Clone {
    type Params: DefaultParams;
    fn edit(&mut self, params: &Self::Params, ui: &mut egui::Ui);

    fn value_as_string(&self) -> String {
        format!("{self:?}")
    }
}

pub trait DefaultParams {
    fn default_params() -> Self;
}
impl DefaultParams for () {
    fn default_params() -> Self {}
}

impl EditableValue for f32 {
    type Params = Range<f32>;

    fn edit(&mut self, params: &Self::Params, ui: &mut egui::Ui) {
        ui.add(Slider::new(self, params.start..=params.end));
    }
}
impl DefaultParams for Range<f32> {
    fn default_params() -> Self {
        0.0..1.0
    }
}

impl DefaultParams for f32 {
    fn default_params() -> Self {
        1.0
    }
}

impl EditableValue for Color {
    // added intensity
    type Params = f32;

    fn edit(&mut self, params: &Self::Params, ui: &mut egui::Ui) {
        let intensity = *params;
        ui.label(format!(
            "picked value will be multiplied with intensity {}",
            intensity
        ));
        let mut rgba: egui::Rgba = egui::Rgba::from_rgba_premultiplied(
            self.r / intensity,
            self.g / intensity,
            self.b / intensity,
            self.a,
        );
        egui::color_picker::color_edit_button_rgba(
            ui,
            &mut rgba,
            egui::color_picker::Alpha::OnlyBlend,
        );
        *self = Color {
            r: rgba.r() * intensity,
            g: rgba.g() * intensity,
            b: rgba.b() * intensity,
            a: rgba.a(),
        }
    }
}

impl EditableValue for Vec3 {
    // added intensity
    type Params = (Vec3, Vec3);

    fn edit(&mut self, params: &Self::Params, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("x");
            ui.add(Slider::new(&mut self.x, params.0.x..=params.1.x));

            ui.label("y");
            ui.add(Slider::new(&mut self.y, params.0.y..=params.1.y));

            ui.label("z");
            ui.add(Slider::new(&mut self.z, params.0.z..=params.1.z));
        });
    }
}

impl DefaultParams for (Vec3, Vec3) {
    fn default_params() -> Self {
        (Vec3::splat(-100.0), Vec3::splat(100.0))
    }
}
