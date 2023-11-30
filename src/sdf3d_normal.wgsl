// Copyright © Michael Winkelmann <michael@winkelmann.site>
// SPDX-License-Identifier: AGPL-3.0-or-later

fn sdf3d_normal(p: vec3<f32>, eps: f32) -> vec3<f32> {
    let v1 = vec3( 1.0,-1.0,-1.0);
    let v2 = vec3(-1.0,-1.0, 1.0);
    let v3 = vec3(-1.0, 1.0,-1.0);
    let v4 = vec3( 1.0, 1.0, 1.0);
    return v1*sdf3d(p + v1*eps) + v2*sdf3d(p + v2*eps) + v3*sdf3d(p + v3*eps) + v4*sdf3d(p + v4*eps);
}
