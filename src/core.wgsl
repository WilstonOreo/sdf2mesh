
/// Bounds
struct Bounds3D {
    min: vec3f,
    max: vec3f,
}

fn bounds_size(bounds: Bounds3D) -> vec3f {
    return bounds.max - bounds.min;
}
