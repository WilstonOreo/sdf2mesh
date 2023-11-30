// Copyright © Michael Winkelmann <michael@winkelmann.site>
// SPDX-License-Identifier: AGPL-3.0-or-later

// Builtin SDF3D primitives
// mostly translated from https://iquilezles.org/articles/distfunctions/ 

fn sdf3d_box(p: vec3f, b: vec3f) -> f32
{
  let q = abs(p) - 0.5 * b;
  return length(max(q,vec3f(0.0, 0.0, 0.0))) + min(max(q.x,max(q.y,q.z)),0.0);
}

fn sdf3d_cylinder(p: vec3f, h: f32, r: f32) -> f32
{
  let d: vec2f = abs(vec2(length(p.xz),p.y)) - vec2(r,h);
  return min(max(d.x,d.y),0.0) + length(max(d,vec2f()));
}

fn sdf3d_capsule(p: vec3f, a: vec3f, b: vec3f, r: f32) -> f32
{
  let pa = p - a;
  let ba = b - a;
  let h = clamp( dot(pa,ba)/dot(ba,ba), 0.0, 1.0);
  return length( pa - ba*h ) - r;
}

fn sdf3d_sphere(p: vec3f, s: f32) -> f32
{
  return length(p)-s;
}

fn sdf3d_torus(p: vec3f, t: vec2f) -> f32
{
  let q = vec2(length(p.xz)-t.x,p.y);
  return length(q)-t.y;
}



