struct AppState {
    bb_min: vec4<f32>,
    bb_max: vec4<f32>,
    dims: vec4<u32>,
}

fn sdf3d_torus(p: vec3f, t: vec2f) -> f32
{
  let q = vec2(length(p.xz)-t.x,p.y);
  return length(q)-t.y;
}

fn normal(p: vec3<f32>, eps: f32) -> vec3<f32> {
    let v1 = vec3( 1.0,-1.0,-1.0);
    let v2 = vec3(-1.0,-1.0, 1.0);
    let v3 = vec3(-1.0, 1.0,-1.0);
    let v4 = vec3( 1.0, 1.0, 1.0);
    return v1*sdf(p + v1*eps) + v2*sdf(p + v2*eps) + v3*sdf(p + v3*eps) + v4*sdf(p + v4*eps);
}

fn sdf(p: vec3<f32>) -> f32 {
    return sdf3d_torus(p, vec2(0.5, 0.2));
}

@group(0)
@binding(0)
var<uniform> app_state: AppState;


@group(0)
@binding(1)
var tex_vertex_normals: texture_storage_2d<rgba32float, write>;

@group(0)
@binding(2)
var tex_vertex_positions: texture_storage_2d<rgba32float, write>;


fn grid_resolution() -> vec3<u32> {
    return vec3(app_state.dims.xyz);
}

/// Bounds
struct Bounds3D {
    min: vec3f,
    max: vec3f,
}

fn bounds_size(bounds: Bounds3D) -> vec3f {
    return bounds.max - bounds.min;
}


fn state_bounds() -> Bounds3D {
    return Bounds3D(app_state.bb_min.xyz, app_state.bb_max.xyz);   
}

fn adapt(v0: f32, v1: f32) -> f32 {
//    assert!((v1 > 0.0) != (v0 > 0.0), "v0 and v1 do not have opposite sign");
    return (0.0 - v0) / (v1 - v0);
}


/// Cell
struct Cell {
    data: array<f32, 8>,
    bounds: Bounds3D,
    pos: vec3<i32>,
}

fn cell_bounds(bounds: Bounds3D, pos: vec3<i32>) -> Bounds3D {
    var res = grid_resolution();
    var v = vec3(f32(res.x - 1u), f32(res.y - 1u), f32(res.z - 1u));
    var size = bounds_size(bounds) / v;    
    var min = bounds.min + vec3(size.x * f32(pos.x), size.y * f32(pos.y), size.z * f32(pos.z));
    return Bounds3D(min, min + size);
}

fn cell_new(pos: vec3<i32>) -> Cell {
    var b = cell_bounds(state_bounds(), pos); // Bounds3D

    return Cell(
        array(  
            sdf(vec3(b.min.x, b.min.y, b.min.z)),
            sdf(vec3(b.max.x, b.min.y, b.min.z)),
            sdf(vec3(b.min.x, b.max.y, b.min.z)),
            sdf(vec3(b.max.x, b.max.y, b.min.z)),
            sdf(vec3(b.min.x, b.min.y, b.max.z)),
            sdf(vec3(b.max.x, b.min.y, b.max.z)),
            sdf(vec3(b.min.x, b.max.y, b.max.z)),
            sdf(vec3(b.max.x, b.max.y, b.max.z)),            
        ),
        b, pos);
}


// cell_getXYZ
fn cell_get000(cell: Cell) -> f32 { return cell.data[0]; }
fn cell_get100(cell: Cell) -> f32 { return cell.data[1]; }
fn cell_get010(cell: Cell) -> f32 { return cell.data[2]; }
fn cell_get110(cell: Cell) -> f32 { return cell.data[3]; }
fn cell_get001(cell: Cell) -> f32 { return cell.data[4]; }
fn cell_get101(cell: Cell) -> f32 { return cell.data[5]; }
fn cell_get011(cell: Cell) -> f32 { return cell.data[6]; }
fn cell_get111(cell: Cell) -> f32 { return cell.data[7]; }

/// Returns a u32 with 4 sign bits 
fn cell_sign_changes(cell: Cell) -> vec4<bool> {
    return vec4(cell_get100(cell) > 0.0, cell_get010(cell) > 0.0, cell_get001(cell) > 0.0, cell_get000(cell) > 0.0);
}

fn cell_sign_changes_f32(cell: Cell) -> f32 {
    var signs = cell_sign_changes(cell);
    var v = 0.0;
    if signs.x { v += 1.0; }
    if signs.y { v += 2.0; }
    if signs.z { v += 4.0; }
    if signs.w { v += 8.0; }
    return v;
}

fn compare(a: f32, b: f32) -> f32 {
    if a > 0.0 != b > 0.0 {
        return 1.0;
    }
    return 0.0;
}

/// Returns interpolated position from cell
fn cell_fetch_interpolated_pos(c: Cell) -> vec4f {        
    var c000 = cell_get000(c);
    var c100 = cell_get100(c);
    var c010 = cell_get010(c);
    var c110 = cell_get110(c);
    var c001 = cell_get001(c);
    var c101 = cell_get101(c);
    var c011 = cell_get011(c);
    var c111 = cell_get111(c);

    var changes = array<vec3f, 12>();

    // Changes in Z direction
    changes[0] = compare(c000, c001) * vec3(0.0, 0.0, adapt(c000, c001));
    changes[1] = compare(c010, c011) * vec3(0.0, 1.0, adapt(c010, c011));
    changes[2] = compare(c100, c101) * vec3(1.0, 0.0, adapt(c100, c101));
    changes[3] = compare(c110, c111) * vec3(1.0, 1.0, adapt(c110, c111));

    // Changes in Y direction
    changes[4] = compare(c000, c010) * vec3(0.0, adapt(c000, c010), 0.0);
    changes[5] = compare(c001, c011) * vec3(0.0, adapt(c001, c011), 1.0);
    changes[6] = compare(c100, c110) * vec3(1.0, adapt(c100, c110), 0.0);
    changes[7] = compare(c101, c111) * vec3(1.0, adapt(c101, c111), 1.0);

    // Changes in X direction
    changes[8] = compare(c000, c100) * vec3(adapt(c000, c100), 0.0, 0.0);
    changes[9] = compare(c001, c101) * vec3(adapt(c001, c101), 0.0, 1.0);
    changes[10] = compare(c010, c110) * vec3(adapt(c010, c110), 1.0, 0.0);
    changes[11] = compare(c011, c111) * vec3(adapt(c011, c111), 1.0, 1.0);

    var avg = vec3(0.0, 0.0, 0.0);
    var change_count = 0.0;

    for (var i: u32 = 0u; i < 12u; i++) {
        if changes[i].x > 0.0 || changes[i].y > 0.0 || changes[i].z > 0.0 {
            avg += changes[i];
            change_count += 1.0;
        } 
    }

    if change_count <= 1.0 {
        return vec4(-1.0,-1.0,-1.0,-1.0);
    }

    return vec4(c.bounds.min + bounds_size(c.bounds) * avg / change_count, 1.0);
}

fn cell_average_value(c: Cell) -> f32 {
    var avg = 0.0;
    for (var i: u32 = 0u; i < 8u; i++) {
        avg += c.data[0];
    }
    return avg / 8.0;
}





@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    var pos = vec3(i32(id.x), i32(id.y), i32(app_state.dims.w));
    var cell = cell_new(pos);

    var p = cell_fetch_interpolated_pos(cell);
    if p.w >= 0.0 { // We have a vertex
        var eps = app_state.bb_max.w;
        var n = normalize(normal(p.xyz, eps));
        var signs = cell_sign_changes_f32(cell);

        textureStore(tex_vertex_normals, pos.xy, vec4(n.xyz, signs));
        textureStore(tex_vertex_positions, pos.xy, p);
    } else {
        textureStore(tex_vertex_normals, pos.xy, vec4(0.0, 0.0, 0.0, 0.0));
        textureStore(tex_vertex_positions, pos.xy, vec4(0.0, 0.0, 0.0, 0.0));
    }
}

