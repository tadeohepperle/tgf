struct AlphaSdfVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) border_color: vec4<f32>,
    // params: in_to_border_cutoff, in_to_border_smooth, border_to_out_cutof, border_to_out_smooth
    @location(2) params: vec4<f32>,
    @location(3) uv: vec2<f32>,
};

@fragment
fn alpha_sdf_fs(in: AlphaSdfVertexOutput) -> @location(0) vec4<f32> {   
    // border inside:
    let in_cutoff = in.params.x;
    let in_smooth = in.params.y;
    // border outside:
    let out_cutoff = in.params.z;
    let out_smooth = in.params.w;

    let in_color_with_alpha_1 = vec4(in.color.rgb, 1.0);
    var image_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.uv) * in_color_with_alpha_1;
    let in_sdf = image_color.a - in_cutoff;
    let out_sdf = image_color.a - out_cutoff;

    let inside_factor = smoothstep(-in_smooth, in_smooth, in_sdf);
    let inside_border_factor = smoothstep(-out_smooth, out_smooth, out_sdf);

    var color = mix(in.border_color, image_color, inside_factor);
    color.a = inside_border_factor * in.color.a;

    if color.a == 0.0{
        discard;
    }

    return color; // in.color for transparency, todo! maybe not multiply
}
