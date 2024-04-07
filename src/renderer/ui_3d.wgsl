// Attention:
//
// This file is only partial. It is concatenated with "coal-ui/src/ui.wgsl" as runtime. 
//


struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

struct PushData {
   col1: vec4<f32>,
   col2: vec4<f32>,
   col3: vec4<f32>,
   translation: vec4<f32>,
   color: vec4<f32>,
}
var<push_constant> data: PushData;

@vertex
fn rect_vs_3d(
    @builtin(vertex_index) vertex_index: u32,
    instance: RectInstance,
) -> RectVertexOutput {
    let vertex = pos_vertex_with_shadow(vertex_index, instance.aabb, instance.others[2]); // instance.others[2] is shadow width
    let xy_plane_offset = vec2<f32>(vertex.pos.x / 100.0, -vertex.pos.y / 100.0);
    let model_matrix = mat4x4<f32>(
        data.col1,
        data.col2,
        data.col3,
        data.translation,
    );
    let world_position = vec4<f32>(xy_plane_offset, 0.0, 1.0);
    
    var out: RectVertexOutput;
    out.clip_position = camera.view_proj * model_matrix * world_position;

    let center = (instance.aabb.xy + instance.aabb.zw) * 0.5;
    out.offset = vertex.pos - center;

    out.size = instance.aabb.zw - instance.aabb.xy;
    out.color = instance.color * data.color; // (apply push constants color)
    out.border_radius = instance.border_radius;
    out.border_color = instance.border_color * data.color; // (apply push constants color) 
    out.others = instance.others;
    out.shadow_color = instance.shadow_color * data.color; // (apply push constants color)
    return out;
}

@vertex
fn textured_rect_vs_3d(
    @builtin(vertex_index) vertex_index: u32,
    instance: TexturedRectInstance,
) -> TexturedRectVertexOutput {
    let vertex = pos_uv_vertex(vertex_index, instance.aabb, instance.uv);
    let xy_plane_offset = vec2<f32>(vertex.pos.x / 100.0, -vertex.pos.y / 100.0);
    let model_matrix = mat4x4<f32>(
        data.col1,
        data.col2,
        data.col3,
        data.translation,
    );
    let world_position = vec4<f32>(xy_plane_offset, 0.0, 1.0);

    var out: TexturedRectVertexOutput;
    out.clip_position = camera.view_proj * model_matrix * world_position;
    
    let center = (instance.aabb.xy + instance.aabb.zw) * 0.5;
    out.offset = vertex.pos - center;

    out.size = instance.aabb.zw - instance.aabb.xy;
    out.color = instance.color * data.color; // (apply push constants color)
    out.border_radius = instance.border_radius;
    out.border_color = instance.border_color * data.color; // (apply push constants color)
    out.others = instance.others;
    out.shadow_color = instance.shadow_color * data.color; // (apply push constants color)
    out.uv = vertex.uv;
    return out;
}



@vertex
fn alpha_sdf_rect_vs_3d(
    @builtin(vertex_index) vertex_index: u32,
    instance: AlphaSdfRectInstance,
) -> AlphaSdfVertexOutput {
    let vertex = pos_uv_vertex(vertex_index, instance.aabb, instance.uv);
    let xy_plane_offset = vec2<f32>(vertex.pos.x / 100.0, -vertex.pos.y / 100.0);
    let model_matrix = mat4x4<f32>(
        data.col1,
        data.col2,
        data.col3,
        data.translation,
    );
    let world_position = vec4<f32>(xy_plane_offset, 0.0, 1.0);
    var out: AlphaSdfVertexOutput;
    out.clip_position = camera.view_proj * model_matrix * world_position;
    
    out.color = instance.color * data.color;
    out.border_color = instance.border_color * data.color;
    out.params = instance.params;
    out.uv = vertex.uv;
    return out;
}


@vertex
fn glyph_vs_3d(
    @builtin(vertex_index) vertex_index: u32,
    instance: GlyphInstance,
) -> GlyphVertexOutput {
    let vertex = pos_uv_vertex(vertex_index, instance.aabb, instance.uv);
    let xy_plane_offset = vec2<f32>(vertex.pos.x / 100.0, -vertex.pos.y / 100.0);
    let model_matrix = mat4x4<f32>(
        data.col1,
        data.col2,
        data.col3,
        data.translation,
    );
    let world_position = vec4<f32>(xy_plane_offset, 0.0, 1.0);

    var out: GlyphVertexOutput;
    out.clip_position = camera.view_proj * model_matrix * world_position;

    out.color = instance.color * data.color; // (apply push constants color)
    out.uv = vertex.uv; 
    out.shadow_intensity = instance.shadow_intensity * data.color.a;
    return out;
}


