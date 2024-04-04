struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;


@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct SpriteInstance {
   @location(0) col1: vec4<f32>,         // transform
   @location(1) col2: vec4<f32>,         // transform
   @location(2) col3: vec4<f32>,         // transform
   @location(3) translation: vec4<f32>,  // transform
   @location(4) offset_and_size: vec4<f32>, // pos
   @location(5) uv: vec4<f32>,           // uv
   @location(6) color: vec4<f32>,        // color
   @location(7) border_color: vec4<f32>, // border_color
   @location(8) params: vec4<f32>,       // in_to_border_cutoff, in_to_border_smooth, border_to_out_cutof, border_to_out_smooth
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, sprite: SpriteInstance) -> AlphaSdfVertexOutput {

    let offset = sprite.offset_and_size.xy;
    let size = sprite.offset_and_size.zw;
    let size_half = size / 2.0;

    let u_uv = unit_uv_from_idx(vertex_index);

    let uv = ((vec2(1.0) - u_uv) * sprite.uv.xy) + (u_uv * sprite.uv.zw);

    let pos = ((vec2(u_uv.x, 1.0 -u_uv.y)) * size) - size_half;
    
    let world_position = vec4<f32>(pos + offset, 0.0, 1.0);
    let model_matrix = mat4x4<f32>(
        sprite.col1,
        sprite.col2,
        sprite.col3,
        sprite.translation,
    );


    var out: AlphaSdfVertexOutput;
    out.clip_position = camera.view_proj * model_matrix * world_position;
    
    out.color = sprite.color;
    out.border_color = sprite.border_color;
    out.params = sprite.params;
    out.uv = uv;
    return out;
}


// for fs: see alpha_sdf_fs in alpha_sdf.wgsl          
fn unit_uv_from_idx(idx: u32) ->  vec2<f32> {
    var out: vec2<f32>;
    switch idx {
      case 0u: {
            out = vec2<f32>(0.0, 0.0); // min x, min y 
        }
      case 1u: {
            out = vec2<f32>(0.0, 1.0); // min x, max y 
        }
      case 2u: {
            out = vec2<f32>(1.0, 0.0); // max x, max y
        }
      case 3u, default: {
            out = vec2<f32>(1.0, 1.0); // max x, min y 
        }
    }
    return out;
}
