// Copyright © Michael Winkelmann <michael@winkelmann.site>
// SPDX-License-Identifier: AGPL-3.0-or-later

pub type Scalar = f32;
pub type Vec2D = euclid::Vector2D<Scalar, euclid::UnknownUnit>;
pub type Vec3D = euclid::Vector3D<Scalar, euclid::UnknownUnit>;
use std::fmt;

pub use euclid::approxeq::ApproxEq;

pub enum Axis2D {
    X,
    Y,
}

pub enum Axis3D {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, Debug)]
pub struct Bounds<T: Copy>(T, T);

impl<
        T: Copy
            + std::ops::Sub<Output = T>
            + std::ops::Add<Output = T>
            + std::ops::Mul<Scalar, Output = T>,
    > Bounds<T>
{
    pub fn min_max(min: T, max: T) -> Self {
        Self(min, max)
    }
    pub fn size_center(size: T, center: T) -> Self {
        Self::min_max(center - size * 0.5, size * 0.5 + center)
    }

    pub fn expanded(&self, d: T) -> Self {
        Self(*self.min() - d, *self.max() + d)
    }

    pub fn min(&self) -> &T {
        &self.0
    }
    pub fn max(&self) -> &T {
        &self.1
    }
    pub fn min_mut(&mut self) -> &mut T {
        &mut self.0
    }
    pub fn max_mut(&mut self) -> &mut T {
        &mut self.1
    }

    pub fn size(&self) -> T {
        *self.max() - *self.min()
    }
    pub fn center(&self) -> T {
        (*self.max() + *self.min()) * 0.5
    }
}

pub type Bounds2D = Bounds<Vec2D>;

impl Bounds2D {
    pub fn square(a: f32, center: Vec2D) -> Self {
        let v = Vec2D::new(a, a) * 0.5;
        Self(center - v, center + v)
    }

    pub fn union_(a: &Bounds2D, b: &Bounds2D) -> Self {
        Self(a.min().min(*b.min()), a.max().max(*b.max()))
    }

    pub fn intersection(a: &Bounds2D, b: &Bounds2D) -> Self {
        Self(a.min().max(*b.min()), a.max().min(*b.max()))
    }

    pub fn scaled(&self, scale: Scalar) -> Bounds2D {
        Bounds2D::min_max(self.0, self.0 + self.size() * scale)
    }

    pub fn max_extent(&self) -> Scalar {
        let s = self.size();
        s.x.max(s.y)
    }

    pub fn contains(&self, p: &Vec2D) -> bool {
        p.x >= self.0.x && p.y >= self.0.y && p.x < self.1.x && p.y < self.1.y
    }
}

impl Default for Bounds2D {
    fn default() -> Self {
        Self(
            Vec2D::splat(Scalar::INFINITY),
            Vec2D::splat(Scalar::NEG_INFINITY),
        )
    }
}

pub type Bounds3D = Bounds<Vec3D>;

impl Bounds3D {
    pub fn centered(size: &Vec3D) -> Self {
        let v = *size * 0.5;
        Self(-v, v)
    }

    pub fn projected_z(&self) -> Bounds2D {
        Bounds2D::min_max(self.min().xy(), self.max().xy())
    }

    pub fn cube(a: Scalar, center: &Vec3D) -> Self {
        let v = Vec3D::splat(a) * 0.5;
        Self(*center - v, *center + v)
    }

    pub fn centered_cube(a: Scalar) -> Self {
        let v = Vec3D::splat(a) * 0.5;
        Self(-v, v)
    }

    pub fn max_extent(&self) -> Scalar {
        let s = self.size();
        s.x.max(s.y.max(s.z))
    }

    pub fn union_(a: &Bounds3D, b: &Bounds3D) -> Self {
        Self(a.min().min(*b.min()), a.max().max(*b.max()))
    }

    pub fn contains(&self, p: &Vec3D) -> bool {
        p.x >= self.0.x
            && p.y >= self.0.y
            && p.z >= self.0.z
            && p.x < self.1.x
            && p.y < self.1.y
            && p.z < self.1.z
    }
}

impl Default for Bounds3D {
    fn default() -> Self {
        Self(
            Vec3D::splat(Scalar::INFINITY),
            Vec3D::splat(Scalar::NEG_INFINITY),
        )
    }
}
impl fmt::Display for Bounds3D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?}, {:?})", self.min(), self.max())
    }
}

#[derive(Clone, Default, Copy)]
pub struct Vertex {
    pub pos: Vec3D,
    pub normal: Vec3D,
}

pub struct Edge<T: Copy>(pub T, pub T);

pub type Edge2D = Edge<Vec2D>;
pub type Edge3D = Edge<Vec3D>;

impl<T: Copy> Edge<T> {
    pub fn swap(&self, yes: bool) -> Edge<T> {
        if yes {
            Edge(self.1, self.0)
        } else {
            Edge(self.0, self.1)
        }
    }
}

pub struct Triangle<T: Copy>(pub T, pub T, pub T);

impl Triangle<Vertex> {
    pub fn normal(&self) -> Vec3D {
        (self.2.pos - self.0.pos).cross(self.1.pos - self.0.pos)
    }
}

#[derive(Clone, Default)]
pub struct Quad<T: Copy>(pub T, pub T, pub T, pub T);

impl<T: Copy> Quad<T> {
    pub fn swap(&self, yes: bool) -> Quad<T> {
        if yes {
            Quad(self.3, self.2, self.1, self.0)
        } else {
            Quad(self.0, self.1, self.2, self.3)
        }
    }

    pub fn make_triangles(&self) -> (Triangle<T>, Triangle<T>) {
        (
            Triangle(self.2, self.1, self.0),
            Triangle(self.0, self.3, self.2),
        )
    }
}

pub mod mesh;
pub mod png;
pub mod shader;
pub mod texture;