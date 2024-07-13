struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
}
struct Screen {
    width: f32,
    height: f32,
    aspect: f32,
}
struct Time {
    delta: f32, // in seconds
    total: f32, // in seconds
    frame_count: u32,
}
struct Input {
    cursor_pos: vec2<f32>
}
struct Globals{
    camera: Camera,
    screen: Screen,
    time: Time, 
    input: Input,
}
@group(0) @binding(0)
var<uniform> camera: Camera;
@group(0) @binding(1)
var<uniform> screen: Screen;
@group(0) @binding(2)
var<uniform> time: Time;
@group(0) @binding(3)
var<uniform> input: Input;