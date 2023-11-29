use core;

/// Cell
struct Cell {
    data: array<f32, 8>,
    bounds: Bounds3D,
    pos: vec3<i32>,
}

fn sdf3d_cell_bounds(bounds: Bounds3D, res: vec3<u32>, pos: vec3<i32>) -> Bounds3D {
    var v = vec3(f32(res.x - 1u), f32(res.y - 1u), f32(res.z - 1u));
    var size = bounds_size(bounds) / v;    
    var min = bounds.min + vec3(size.x * f32(pos.x), size.y * f32(pos.y), size.z * f32(pos.z));
    return Bounds3D(min, min + size);
}

fn sdf3d_cell_new(b: Bounds3D) -> Cell {
    return sdf3d_Cell(
        array(  
            sdf3d(vec3(b.min.x, b.min.y, b.min.z)),
            sdf3d(vec3(b.max.x, b.min.y, b.min.z)),
            sdf3d(vec3(b.min.x, b.max.y, b.min.z)),
            sdf3d(vec3(b.max.x, b.max.y, b.min.z)),
            sdf3d(vec3(b.min.x, b.min.y, b.max.z)),
            sdf3d(vec3(b.max.x, b.min.y, b.max.z)),
            sdf3d(vec3(b.min.x, b.max.y, b.max.z)),
            sdf3d(vec3(b.max.x, b.max.y, b.max.z)),            
        ),
        b, pos
    );
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


fn _cell_adapt(v0: f32, v1: f32) -> f32 {
    return (0.0 - v0) / (v1 - v0);
}



fn _cell_change(a: f32, b: f32, x: f32, y: f32, z: f32) -> vec3f {
    if a > 0.0 != b > 0.0 {
        return vec3(x,y,z);
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
    changes[0] = _cell_change(c000, c001, 0.0, 0.0, _cell_adapt(c000, c001));
    changes[1] = _cell_change(c010, c011, 0.0, 1.0, _cell_adapt(c010, c011));
    changes[2] = _cell_change(c100, c101, 1.0, 0.0, _cell_adapt(c100, c101));
    changes[3] = _cell_change(c110, c111, 1.0, 1.0, _cell_adapt(c110, c111));

    // Changes in Y direction
    changes[4] = _cell_change(c000, c010, 0.0, _cell_adapt(c000, c010), 0.0);
    changes[5] = _cell_change(c001, c011, 0.0, _cell_adapt(c001, c011), 1.0);
    changes[6] = _cell_change(c100, c110, 1.0, _cell_adapt(c100, c110), 0.0);
    changes[7] = _cell_change(c101, c111, 1.0, _cell_adapt(c101, c111), 1.0);

    // Changes in X direction
    changes[8] = _cell_change(c000, c100, _cell_adapt(c000, c100), 0.0, 0.0);
    changes[9] = _cell_change(c001, c101, _cell_adapt(c001, c101), 0.0, 1.0);
    changes[10] = _cell_change(c010, c110, _cell_adapt(c010, c110), 1.0, 0.0);
    changes[11] = _cell_change(c011, c111, _cell_adapt(c011, c111), 1.0, 1.0);

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