struct CameraUniform {
    view_proj: mat4x4<f32>,
    screen_size: vec2<f32>,
    _padding: vec2<f32>,
};
@group(0) @binding(0) var<uniform> camera: CameraUniform;

@group(1) @binding(0) var t_sdf: texture_2d<f32>;
@group(1) @binding(1) var s_sdf: sampler;

struct InstanceInput {
    @location(5) p1: vec2<f32>, 
    @location(6) p2: vec2<f32>, 
    @location(7) p3: vec2<f32>, 
    @location(8) z_index: f32,       // 3D Depth
    @location(9) color: vec4<f32>,
    @location(10) width: f32,
    @location(11) radius: f32,
    @location(12) type_id: u32,
    @location(13) angle: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,     
    @location(1) color: vec4<f32>,
    @location(2) type_id: f32,
    @location(3) dimensions: vec2<f32>,
    @location(4) angles: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) v_idx: u32,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = instance.color;
    out.type_id = f32(instance.type_id);
    out.angles = vec2<f32>(0.0);

    var z = instance.z_index;

    // --- CASE 1: TRIANGLE (Type 3) ---
    if (instance.type_id == 3u) {
        var pos = instance.p1;
        if (v_idx == 1u) { pos = instance.p2; }
        if (v_idx >= 2u) { pos = instance.p3; }
        out.clip_position = camera.view_proj * vec4<f32>(pos, z, 1.0);
        out.uv = vec2<f32>(0.0); 
        return out;
    }

    var pos = array<vec2<f32>, 4>(
        vec2<f32>(-0.5, -0.5), vec2<f32>( 0.5, -0.5),
        vec2<f32>(-0.5,  0.5), vec2<f32>( 0.5,  0.5)
    );
    let v_pos = pos[v_idx]; 

    // --- CASE 5: ARC (Type 5) ---
    if (instance.type_id == 5u) {
        let outer_dist = instance.radius + (instance.width * 0.5) + 2.0;
        let size = outer_dist * 2.0;
        let final_pos = instance.p1 + (v_pos * size);
        out.clip_position = camera.view_proj * vec4<f32>(final_pos, z, 1.0);
        out.uv = v_pos * size; 
        out.dimensions = vec2<f32>(instance.width, instance.radius);
        out.angles = instance.p2;
        return out;
    }

    // --- CASE 2: TEXT (Type 4) ---
    if (instance.type_id == 4u) {
       let c = cos(instance.angle);
       let s = sin(instance.angle);
       let rot = mat2x2<f32>(c, s, -s, c);
       
       let scaled = v_pos * instance.p2; 
       let rotated = rot * scaled;
       let final_pos = instance.p1 + rotated;

       out.clip_position = camera.view_proj * vec4<f32>(final_pos, z, 1.0);
       let local_uv = vec2<f32>(v_pos.x + 0.5, v_pos.y + 0.5); 
       out.uv = instance.p3 + (local_uv * vec2<f32>(instance.width, instance.radius));
       return out;
    }

    // --- CASE 3: CIRCLES (Type 1 & 2) ---
    if (instance.type_id == 1u || instance.type_id == 2u) {
        let size = instance.radius * 2.0;
        let final_pos = instance.p1 + (v_pos * size);
        out.clip_position = camera.view_proj * vec4<f32>(final_pos, z, 1.0);
        out.uv = v_pos; 
        out.dimensions = vec2<f32>(instance.width, instance.radius);
        return out;
    }

    // --- CASE 4: LINES (Type 0) ---
    if (instance.type_id == 0u) {
        let start = instance.p1;
        let end = instance.p2;
        let thickness = instance.width;

        let delta = end - start;
        let len = length(delta);
        let angle = atan2(delta.y, delta.x);

        let c = cos(angle);
        let s = sin(angle);
        let rot = mat2x2<f32>(c, s, -s, c);

        let scale = vec2<f32>(len, thickness);
        let center = (start + end) * 0.5;

        let pos_rotated = rot * (v_pos * scale);
        let final_pos = center + pos_rotated;

        out.clip_position = camera.view_proj * vec4<f32>(final_pos, z, 1.0);
        out.uv = v_pos; 
        return out;
    }

    out.clip_position = vec4<f32>(0.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let type_id = u32(in.type_id + 0.1);

    if (type_id == 5u) {
        let uv_len = length(in.uv);
        let radius = in.dimensions.y;
        let width = in.dimensions.x;
        let half_w = width * 0.5;
        
        let px_w = fwidth(uv_len); 
        let radial_dist = abs(uv_len - radius);
        let radial_alpha = 1.0 - smoothstep(half_w - 0.5*px_w, half_w + 0.5*px_w, radial_dist);

        let angle_start = in.angles.x;
        let sweep = in.angles.y;
        let curr_angle = atan2(in.uv.y, in.uv.x);
        
        let PI = 3.14159265;
        let TWO_PI = 6.2831853;
        var d = curr_angle - angle_start;
        if (d <= -PI) { d += TWO_PI; }
        if (d > PI) { d -= TWO_PI; }
        
        var ang_dist = 0.0;
        
        if (sweep >= 0.0) {
            if (d < 0.0) { d += TWO_PI; } 
            if (d >= 0.0 && d <= sweep) {
                ang_dist = 0.0;
            } else {
                ang_dist = min(d - sweep, TWO_PI - d);
            }
        } else {
            if (d > 0.0) { d -= TWO_PI; }
            if (d <= 0.0 && d >= sweep) {
                ang_dist = 0.0;
            } else {
                ang_dist = min(sweep - d, d + TWO_PI);
            }
        }
        
        let px_dist = ang_dist * uv_len;
        let ang_alpha = 1.0 - smoothstep(0.0, px_w, px_dist);
        
        return vec4<f32>(in.color.rgb, in.color.a * radial_alpha * ang_alpha);
    }

    if (type_id == 4u) {
        let alpha = textureSample(t_sdf, s_sdf, in.uv).r;
        if (alpha < 0.1) { discard; }
        return vec4<f32>(in.color.rgb, in.color.a * alpha);
    }

    if (type_id == 1u || type_id == 2u) {
        let dist = length(in.uv);
        let delta = fwidth(dist);
        let alpha = 1.0 - smoothstep(0.5 - delta, 0.5, dist);

        if (type_id == 1u) {
            let stroke_w = in.dimensions.x;
            let radius = in.dimensions.y;
            let inner = 0.5 * ((radius - stroke_w) / radius);
            let inner_alpha = smoothstep(inner - delta, inner, dist);
            return vec4<f32>(in.color.rgb, in.color.a * alpha * inner_alpha);
        }
        return vec4<f32>(in.color.rgb, in.color.a * alpha);
    }

    return in.color;
}
