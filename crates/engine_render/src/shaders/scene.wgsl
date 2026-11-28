struct Camera {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,       // xyz: world position
    light_dir: vec4<f32>, // xyz: normalised direction TOWARDS the light
    fog: vec4<f32>,       // rgb: fog colour (linear), a: density
    selection: vec4<u32>, // x: highlighted instance index (0xFFFFFFFF = none)
};
@group(0) @binding(0) var<uniform> camera: Camera;

fn apply_fog(color: vec3<f32>, world_pos: vec3<f32>) -> vec3<f32> {
    let d = distance(camera.eye.xyz, world_pos) * camera.fog.a;
    let f = 1.0 - exp(-d * d); // exp2 fog
    return mix(color, camera.fog.rgb, f);
}

// ---------------------------------------------------------------- meshes

struct MeshIn {
    @builtin(instance_index) instance: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) m0: vec4<f32>,
    @location(3) m1: vec4<f32>,
    @location(4) m2: vec4<f32>,
    @location(5) m3: vec4<f32>,
    @location(6) color: vec4<f32>,
};

struct MeshOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_mesh(in: MeshIn) -> MeshOut {
    let model = mat4x4<f32>(in.m0, in.m1, in.m2, in.m3);
    let world = model * vec4<f32>(in.position, 1.0);

    // Normal matrix = cofactor(M3): equals det * inverse-transpose, correct under
    // non-uniform scale without needing a matrix inverse.
    let a = in.m0.xyz;
    let b = in.m1.xyz;
    let c = in.m2.xyz;
    let n = in.normal.x * cross(b, c) + in.normal.y * cross(c, a) + in.normal.z * cross(a, b);

    var out: MeshOut;
    out.clip = camera.view_proj * world;
    out.world_pos = world.xyz;
    out.normal = n;
    out.color = in.color;
    if (in.instance == camera.selection.x) {
        // Selection tint; keeps the surface shading.
        out.color = vec4<f32>(mix(in.color.rgb, vec3<f32>(1.0, 0.55, 0.08), 0.6), in.color.a);
    }
    return out;
}

@fragment
fn fs_mesh(in: MeshOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let l = camera.light_dir.xyz;
    let ndotl = max(dot(n, l), 0.0);
    let hemi = 0.18 + 0.12 * n.y; // cheap sky/ground ambient
    let lit = in.color.rgb * (hemi + 0.85 * ndotl);
    return vec4<f32>(apply_fog(lit, in.world_pos), in.color.a);
}

// ---------------------------------------------------------------- lines

struct LineIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct LineOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) color: vec3<f32>,
};

@vertex
fn vs_line(in: LineIn) -> LineOut {
    var out: LineOut;
    out.clip = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.world_pos = in.position;
    out.color = in.color;
    return out;
}

@fragment
fn fs_line(in: LineOut) -> @location(0) vec4<f32> {
    return vec4<f32>(apply_fog(in.color, in.world_pos), 1.0);
}
