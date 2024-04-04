// camera
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

struct PushData {
   col1: vec4<f32>,
   col2: vec4<f32>,
   col3: vec4<f32>,
   translation: vec4<f32>,
}
var<push_constant> push: PushData;

struct Particle {
   @location(0) pos_and_rot: vec4<f32>, // pos and rotation
   @location(1) size: vec2<f32>,       // scale
   @location(2) color: vec4<f32>,       // color
   @location(3) uv: vec4<f32>,          // uv aabb
}

struct ParticleVertexOutput{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}


/// instead of using billboarding, we have all particles face the same direction.
/// the rotation is the rotation passed in the push constant transform.
/// we do not apply rotation and scale to all particles together, so you cannot e.g. rotate all particles together around some point.
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, particle: Particle) -> ParticleVertexOutput {
    let u_uv = unit_uv_from_idx(vertex_index); // in unit space
    let uv: vec2<f32> = ((vec2(1.0) - u_uv) * particle.uv.zw) + (u_uv * particle.uv.xy); // mapped to the actual uv coords in the texture
    let size = particle.size;
    let size_half: vec2<f32> = particle.size / 2.0;
    

    let rot = particle.pos_and_rot.w;
    let pos = vec2(
        u_uv.x * size.x - size_half.x,
        u_uv.y * size.y - size_half.y,
    );
    let pos_rotated = vec2(
        cos(rot)* pos.x - sin(rot)* pos.y,
        sin(rot)* pos.x + cos(rot)* pos.y,     
    );
    let world_position = vec4(
        pos_rotated,
        0.0,
        1.0
    );
    
    // todo! use particle.pos_and_rot.w = rotation in plane

    let model_matrix = mat4x4<f32>(
        push.col1,
        push.col2,
        push.col3,
        push.translation + vec4(particle.pos_and_rot.xyz, 0.0),
    );

    var out: ParticleVertexOutput;
    out.clip_position = camera.view_proj * model_matrix * world_position;
    out.color = particle.color;
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: ParticleVertexOutput) -> @location(0) vec4<f32> {
    // return vec4(1.0,0.0,0.0,1.0);

    // todo! use in.uv
    let image_color = textureSample(t_diffuse, s_diffuse, in.uv);
    let color = in.color * image_color;
    return color;
}

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


