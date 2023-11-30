
fn sdf_op_smooth_union(d1: f32, d2: f32, k: f32) -> f32
{
    let h = clamp(0.5 + 0.5*(d2-d1)/k, 0.0, 1.0);
    return mix(d2, d1, h) - k*h*(1.0-h);
}

fn sdf_op_smooth_intersection(d1: f32, d2: f32, k: f32) -> f32
{
    let h = clamp(0.5 - 0.5*(d2-d1)/k, 0.0, 1.0);
    return mix(d2, d1, h) + k*h*(1.0-h);
}

fn sdf_op_smooth_subtraction(d1: f32, d2: f32, k: f32) -> f32
{
    let h = clamp(0.5 - 0.5*(d2+d1)/k, 0.0, 1.0);
    return mix(d2, -d1, h) + k*h*(1.0-h);
}
