// Inspired by: https://www.shadertoy.com/view/fsdyzB
struct ScreenSize {
    width: f32,
    height: f32,
    aspect: f32,
}

const UI_REFERENCE_Y_HEIGHT: f32 = 1080.0;

@group(0) @binding(0)
var<uniform> screen: ScreenSize;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct RectInstance {
    @location(0) aabb: vec4<f32>, // pos aabb for the glyph
    @location(1) color: vec4<f32>,
    @location(2) border_radius: vec4<f32>,
    @location(3) border_color: vec4<f32>,
    // border_width, border_softness, shadow_width, shadow_curve
    @location(4) others: vec4<f32>,
    @location(5) shadow_color: vec4<f32>,
}

struct RectVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) offset: vec2<f32>, // offset from center
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) border_radius: vec4<f32>,
    @location(4) border_color: vec4<f32>,
    // border_width, border_softness, shadow_width, shadow_curve
    @location(5) others: vec4<f32>,
    @location(6) shadow_color: vec4<f32>,
};

struct TexturedRectInstance {
    @location(0) aabb: vec4<f32>, // pos aabb for the glyph
    @location(1) color: vec4<f32>,
    @location(2) border_radius: vec4<f32>,
    @location(3) border_color: vec4<f32>,
    // border_width, border_softness, shadow_width, shadow_curve
    @location(4) others: vec4<f32>,
    @location(5) shadow_color: vec4<f32>,
    // for the texture
    @location(6) uv: vec4<f32>,
}

struct TexturedRectVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) offset: vec2<f32>, // offset from center
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) border_radius: vec4<f32>,
    @location(4) border_color: vec4<f32>,
    // border_width, border_softness, shadow_width, shadow_curve
    @location(5) others: vec4<f32>,
    @location(6) shadow_color: vec4<f32>,
    @location(7) uv: vec2<f32>,
};

struct AlphaSdfRectInstance {
    @location(0) aabb: vec4<f32>, // pos aabb for the glyph
    @location(1) color: vec4<f32>,
    @location(2) border_color: vec4<f32>,
    // params: in_to_border_cutoff, in_to_border_smooth, border_to_out_cutof, border_to_out_smooth
    @location(3) params: vec4<f32>,
    @location(4) uv: vec4<f32>,
}

// see AlphaSdfVertexOutput in sprite_ui_shared.wgsl
struct GlyphInstance {
    @location(0) aabb: vec4<f32>, // pos aabb for the glyph
    @location(1) color: vec4<f32>,
    @location(2) uv: vec4<f32>,    // uv aabb in the texture atlas
    /// font_size, shadow_intensity
    @location(3) others: vec2<f32>,
}

struct GlyphVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
     /// font_size, shadow_intensity
    @location(2) others: vec2<f32>,
};

// we calculate the vertices here in the shader instead of passing a vertex buffer
struct PosVertex {
    pos: vec2<f32>,
}

struct PosUvVertex {
    pos: vec2<f32>,
    uv: vec2<f32>
}


@vertex
fn rect_vs(
    @builtin(vertex_index) vertex_index: u32,
    instance: RectInstance,
) -> RectVertexOutput {
    let vertex = pos_vertex_with_shadow(vertex_index, instance.aabb, instance.others[2]); // instance.others[2] is shadow width
    // the vertex is in ui layout space, lets transform it into screen px space:
    let screen_pos = vertex.pos * screen.height / UI_REFERENCE_Y_HEIGHT;
    let device_pos = vec2<f32>((screen_pos.x / screen.width) * 2.0 - 1.0, 1.0 - (screen_pos.y / screen.height) * 2.0) ;
    let center = (instance.aabb.xy + instance.aabb.zw) * 0.5;

    var out: RectVertexOutput;
    out.clip_position = vec4<f32>(device_pos, 0.0, 1.0);
    out.offset = vertex.pos - center;
    out.size = instance.aabb.zw - instance.aabb.xy;

    out.color = instance.color;
    out.border_radius = instance.border_radius;
    out.border_color = instance.border_color;
    out.others = instance.others;
    out.shadow_color = instance.shadow_color;
    return out;
}
 
@fragment
fn rect_fs(in: RectVertexOutput) -> @location(0) vec4<f32> {
    let smoothness = 0.5; // half a pixel of antialiasing

    let sdf = rounded_box_sdf(in.offset, in.size, in.border_radius);
    let border_width = in.others[0];
    let border_sdf = sdf + border_width;
    let border_factor = smoothstep(0.0 - smoothness, 0.0 + smoothness, border_sdf);
    let rect_color: vec4<f32> = mix(in.color, in.border_color, border_factor);

    let inside_factor = smoothstep(0.0 - smoothness, 0.0 + smoothness, sdf);

    let shadow_width = in.others[2];
    let shadow_factor = 1.0 - (sdf / shadow_width); // in.others[1] is shadow_intensity
    let shadow_factor2 = smoothstep(0.0, 1.0, shadow_factor);
    let shadow_color = vec4(in.shadow_color.rgb, in.shadow_color.a * shadow_factor2);
    let color = mix(rect_color, shadow_color, inside_factor);
    return color;
    // return vec4(rect_color.rgb, rect_color.a * inside_factor);
}


@vertex
fn textured_rect_vs(
    @builtin(vertex_index) vertex_index: u32,
    instance: TexturedRectInstance,
) -> TexturedRectVertexOutput {
    let vertex = pos_uv_vertex(vertex_index, instance.aabb, instance.uv);
    let screen_pos = vertex.pos * screen.height / UI_REFERENCE_Y_HEIGHT; // pos on actual screen.
    let device_pos = vec2<f32>((screen_pos.x / screen.width) * 2.0 - 1.0, 1.0 - (screen_pos.y / screen.height) * 2.0) ;
    let center = (instance.aabb.xy + instance.aabb.zw) * 0.5;

    var out: TexturedRectVertexOutput;
    out.clip_position = vec4<f32>(device_pos, 0.0, 1.0);
    out.offset = vertex.pos - center;
    out.size = instance.aabb.zw - instance.aabb.xy;

    out.color = instance.color;
    out.border_radius = instance.border_radius;
    out.border_color = instance.border_color;
    out.others = instance.others;
    out.shadow_color = instance.shadow_color;
    out.uv = vertex.uv;
    return out;
}

@fragment
fn textured_rect_fs(in: TexturedRectVertexOutput) -> @location(0) vec4<f32> {
    // return vec4(0.5, 0.8,0.8,1.0);
    
    let sdf = rounded_box_sdf(in.offset, in.size, in.border_radius);
 
    let image_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.uv);
    let color: vec4<f32> = mix(image_color, in.border_color, smoothstep(0.0, 1.0, ((sdf + in.others[0]) / in.others[1]) ));
    // todo! add borders and other fancy stuff from above in rect_fs
    return color  * in.color;
}

@vertex
fn alpha_sdf_rect_vs(
    @builtin(vertex_index) vertex_index: u32,
    instance: AlphaSdfRectInstance,
) -> AlphaSdfVertexOutput {
    let vertex = pos_uv_vertex(vertex_index, instance.aabb, instance.uv);
    let screen_pos = vertex.pos * screen.height / UI_REFERENCE_Y_HEIGHT; // pos on actual screen.
    let device_pos = vec2<f32>((screen_pos.x / screen.width) * 2.0 - 1.0, 1.0 - (screen_pos.y / screen.height) * 2.0) ;

    var out: AlphaSdfVertexOutput;
    out.clip_position = vec4<f32>(device_pos, 0.0, 1.0);

    out.color = instance.color;
    out.border_color = instance.border_color;
    out.params = instance.params;
    out.uv = vertex.uv;
    return out;
}

// for alpha_sdf_rect_fs: see alpha_sdf_fs in alpha_sdf.wgsl

@vertex
fn glyph_vs(
    @builtin(vertex_index) vertex_index: u32,
    instance: GlyphInstance,
) -> GlyphVertexOutput {
    let vertex = pos_uv_vertex(vertex_index, instance.aabb, instance.uv);
   
    let scale_factor = screen.height / UI_REFERENCE_Y_HEIGHT;
    let screen_pos = vertex.pos * scale_factor;
    let device_pos = vec2<f32>((screen_pos.x / screen.width) * 2.0 - 1.0, 1.0 - (screen_pos.y / screen.height) * 2.0) ;

    var out: GlyphVertexOutput;
    out.clip_position = vec4<f32>(device_pos, 0.0, 1.0);
    out.color = instance.color;
    out.uv = vertex.uv; 
    out.others = vec2(instance.others.x * scale_factor, instance.others.y); // x = font_size, y = shadow_intensity
    return out;
}

@fragment
fn glyph_fs(in: GlyphVertexOutput) -> @location(0) vec4<f32> {
    let sdf: f32 = textureSample(t_diffuse, s_diffuse, in.uv).r;
    let smoothing: f32 =  4.0 / (in.others[0]); // in.others[0] is font_size  // 6.0 seems to work fine for most cases.
    let inside_factor = smoothstep(0.5 - smoothing, 0.5 + smoothing, sdf);
    let shadow_alpha =  (1.0 - (pow(1.0 - sdf, 2.0)) )* in.others[1] * in.color.a; // in.others[1] is shadow_intensity
    let shadow_color = vec4(0.0,0.0,0.0, shadow_alpha);
    let color = mix(shadow_color, in.color, inside_factor);
    return vec4(color); // * vec4(1.0,1.0,1.0,5.0);
}

// glyph with outline (currently not used)
const outline_dist: f32 = 0.3;
const outline_color: vec4<f32> = vec4(1.0,0.0,0.0,0.0);
@fragment
fn glyph_fs_outline(in: GlyphVertexOutput) -> @location(0) vec4<f32> {
    let sdf: f32 = textureSample(t_diffuse, s_diffuse, in.uv).r;
    let smoothing: f32 =  6.0/ (in.others[0]);
    let outline_factor = smoothstep(0.5 - smoothing, 0.5 + smoothing, sdf);
    let color = mix(outline_color, in.color, outline_factor);
    let alpha = smoothstep(outline_dist - smoothing, outline_dist+smoothing, sdf);
    return vec4(color.rgb, alpha * in.color.a); // ,
}

// given some bounding box aabb [f32;4] being min x, min y, max x, max y,
// extracts the x,y position [f32;2] for the given index in a counter clockwise quad:
// 0 ------ 1
// | .      |
// |   .    |
// |     .  |
// 3 ------ 2  
fn pos_vertex(idx: u32, aabb: vec4<f32>) -> PosVertex {
    var out: PosVertex;
    switch idx {
      case 0u: {
            out.pos = vec2<f32>(aabb.x, aabb.y); // min x, min y 
        }
      case 1u: {
            out.pos = vec2<f32>(aabb.x, aabb.w); // min x, max y 
        }
      case 2u: {
            out.pos = vec2<f32>(aabb.z, aabb.y); // max x, min y 
        }
      case 3u, default: {
           out.pos = vec2<f32>(aabb.z, aabb.w); // max x, max y
        }
    }
    return out;
}

// given some bounding box aabb [f32;4] being min x, min y, max x, max y,
// extracts the x,y position [f32;2] for the given index in a counter clockwise quad:
// 0 ------ 1
// | .      |
// |   .    |
// |     .  |
// 3 ------ 2  
// 
// s is the shadow_width
fn pos_vertex_with_shadow(idx: u32, aabb: vec4<f32>, s: f32) -> PosVertex {
    var out: PosVertex;
    switch idx {
      case 0u: {
            out.pos = vec2<f32>(aabb.x - s, aabb.y - s); // min x, min y 
        }
      case 1u: {
            out.pos = vec2<f32>(aabb.x - s, aabb.w + s); // min x, max y 
        }
      case 2u: {
            out.pos = vec2<f32>(aabb.z + s, aabb.y - s); // max x, min y 
        }
      case 3u, default: {
            out.pos = vec2<f32>(aabb.z + s, aabb.w + s); // max x, max y
        }
    }
    return out;
}

// given some bounding box aabb [f32;4] being min x, min y, max x, max y,
// extracts the x,y position [f32;2] for the given index in a counter clockwise quad:
// 0 ------ 1
// | .      |
// |   .    |
// |     .  |
// 3 ------ 2  
fn pos_uv_vertex(idx: u32, pos: vec4<f32>, uv: vec4<f32>) -> PosUvVertex {
    var out: PosUvVertex;
    switch idx {
      case 0u: {
            out.pos = vec2<f32>(pos.x, pos.y); // min x, min y 
            out.uv = vec2<f32>(uv.x, uv.y);
        }
      case 1u: {
            out.pos = vec2<f32>(pos.x, pos.w); // min x, max y 
            out.uv = vec2<f32>(uv.x, uv.w);
        }
      case 2u: {
            out.pos = vec2<f32>(pos.z, pos.y); // max x, min y 
            out.uv = vec2<f32>(uv.z, uv.y);
        }
      case 3u, default: {
            out.pos = vec2<f32>(pos.z, pos.w); // max x, max y
            out.uv = vec2<f32>(uv.z, uv.w);
        }
    }
    return out;
}


fn rounded_box_sdf(offset: vec2<f32>, size: vec2<f32>, border_radius: vec4<f32>) -> f32 {
    let r = select(border_radius.xw, border_radius.yz, offset.x > 0.0);
    let r2 = select(r.x, r.y, offset.y > 0.0);

    let q: vec2<f32> = abs(offset) - size / 2.0 + vec2<f32>(r2);
    let q2: f32 = min(max(q.x, q.y), 0.0);

    let l = length(max(q, vec2(0.0)));
    return q2 + l - r2;
}