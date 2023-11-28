
use euclid::Vector3D;

use crate::utils::*;

use crate::sdf3d;

pub type Grid3DResolution = Vector3D<u16, euclid::UnknownUnit>;

pub struct Grid3D<T> {
    pub data: Vec<T>,
    size: Grid3DResolution,
}

impl<T: Clone + Default> Grid3D<T> {
    pub fn new(x: u16, y: u16, z: u16) -> Self {
        let len = x as usize*y as usize*z as usize;
        let mut vec = Vec::with_capacity(len);
        vec.resize(len, T::default());

        Self {
            data: vec,
            size: Vector3D::new(x,y,z)
        }
    }

    pub fn get(&self, x: u16, y: u16, z: u16) -> &T {
        &self.data[self.index(x,y,z)]
    }

    pub fn get_mut(&mut self, x: u16, y: u16, z: u16) -> &mut T {
        let index = self.index(x, y, z);
        &mut self.data[index]
    }

    pub fn index(&self, x: u16, y: u16, z: u16) -> usize {
        ((x as usize) * self.size_y() as usize + y as usize)*(self.size_z() as usize) + z as usize
    }

    pub fn n_cells(&self) -> usize {
        self.size_x() as usize * self.size_y() as usize * self.size_z() as usize
    }

    pub fn size_x(&self) -> u16 {
        self.size.x
    }

    pub fn size_y(&self) -> u16 {
        self.size.y
    }

    pub fn size_z(&self) -> u16 {
        self.size.z
    }

    pub fn cell_size(&self, bounds: &Bounds3D) -> Vec3D {
        let v = Vec3D::new(f64::from(self.size_x()-1), f64::from(self.size_y()-1), f64::from(self.size_z()-1));
        bounds.size().component_div(v)
    }
}



pub struct Cell {
    data: [f32; 8],
    pub bounds: Bounds3D,
    pub x: u16,
    pub y: u16,
    pub z: u16
}


impl Cell {
    pub fn from_indices(grid: &Grid3D<f32>, bounds: &Bounds3D, x: u16, y: u16, z: u16) -> Self {
        Self {
            data: [*grid.get(x,y,z), 
                   *grid.get(x+1,y,z),
                   *grid.get(x,y+1,z), 
                   *grid.get(x+1,y+1,z), 
                   *grid.get(x,y,z+1), 
                   *grid.get(x+1,y,z+1), 
                   *grid.get(x,y+1,z+1),
                   *grid.get(x+1,y+1,z+1)],
            bounds: Self::cell_bounds(grid, bounds, x, y, z),
            x,
            y,
            z,
        }
    }

    pub fn new(data: [f32; 8], bounds: &Bounds3D, x: u16, y: u16, z: u16) -> Self {
        Self {
            data,
            bounds: *bounds,
            x,
            y,
            z,
        }
    }

    pub fn center(&self) -> Vec3D {
        self.bounds.center()
    }

    pub fn average_value(&self) -> f32 {
        let mut avg = 0.0_f32;
        for a in &self.data {
            avg += a;
        }
        avg / 8.0
    }

    fn cell_bounds(grid: &Grid3D<f32>, bounds: &Bounds3D, x: u16, y: u16, z: u16) -> Bounds3D {
        let size = grid.cell_size(bounds);
        let min = *bounds.min() + Vec3D::new(size.x * x as f64,size.y * y as f64,size.z * z as f64);
        Bounds3D::min_max(min, min + size)
    }

    pub fn get(&self, dx: u8, dy: u8, dz: u8) -> f32 {
        self.data[(((dz * 2) + dy)*2 + dx) as usize]
    }
    
    pub fn sign_changes(&self) -> (bool, bool, bool, bool) {
        (
            self.get(1,0,0) > 0.0,
            self.get(0,1,0) > 0.0,
            self.get(0,0,1) > 0.0,
            self.get(0,0,0) > 0.0,
        )
    }

    pub fn sign_changes_u8(&self) -> u8 {
        let changes = self.sign_changes();
        (changes.0 as u8) | ((changes.1 as u8) << 1) | ((changes.2 as u8) << 2) | ((changes.3 as u8) << 3)
    }

    pub fn try_fetch_interpolated_pos(&self) -> Option<Vec3D> {
        let mut changes = Vec::new();
        
        let min = self.bounds.min();
        let size = self.bounds.size();
        
        for dx in 0..2 {
            for dy in 0..2 {
                if (self.get(dx, dy, 0) > 0.0) != (self.get(dx, dy, 1) > 0.0) {
                    changes.push(Vec3D::new(
                        min.x + size.x * (dx as f64), 
                        min.y + size.y * (dy as f64),
                        min.z + size.z * adapt(self.get(dx, dy, 0).into(), self.get(dx, dy, 1).into())
                    ))
                }
            }
        }

        for dx in 0..2 {
            for dz in 0..2 {
                if (self.get(dx, 0, dz) > 0.0) != (self.get(dx, 1, dz) > 0.0) {
                    changes.push(Vec3D::new(
                        min.x + size.x * (dx as f64), 
                        min.y + size.y * adapt(self.get(dx, 0, dz).into(), self.get(dx, 1, dz).into()),
                        min.z + size.z * (dz as f64)
                    ))
                }
            }
        }

        for dy in 0..2 {
            for dz in 0..2 {
                if (self.get(0, dy, dz) > 0.0) != (self.get(1, dy, dz) > 0.0) {
                    changes.push(Vec3D::new(
                        min.x + size.x * adapt(self.get(0, dy, dz).into(), self.get(1, dy, dz).into()),
                        min.y + size.y * (dy as f64), 
                        min.z + size.z * (dz as f64)
                    ))
                }
            }
        }
        if changes.len() <= 1 {
            return None;
        }

        let mut avg = Vec3D::zero();
        for change in &changes {
            avg += *change;
        }

        Some(avg * (1.0 / (changes.len() as f64)))
    }

    pub fn try_fetch_interpolated_vertex(&self, sdf: &dyn sdf3d::DistanceFunction) -> Option<Vertex> {
        let p = self.try_fetch_interpolated_pos()?;
        Some(Vertex { pos: p, normal: sdf.normal(&p).normalize() })
    }
}


impl Grid3D<f32> {
    pub fn for_each_cell(&mut self, bounds: &Bounds3D, mut f: impl FnMut(&Cell)) {
        for z in 0..self.size_z()-1 {
            for y in 0..self.size_y()-1 {
                for x in 0..self.size_x()-1 {
                    f(&Cell::from_indices(self, bounds, x, y, z));
                }
            }
        }
    }
}



#[derive(Clone, Default)]
pub struct VertexCell(u32);

impl VertexCell {
    pub fn new(vertex_id: usize, cell: &Cell) -> Self {
        Self(((vertex_id as u32) << 4) | cell.sign_changes_u8() as u32)
    }

    pub fn vertex_id(&self) -> u32 {
        self.0 >> 4
    }

    pub fn vertex(&self, vertices: &[Vertex]) -> Vertex {
        vertices[self.vertex_id() as usize]
    }

    pub fn changes(&self) -> (bool,bool,bool,bool) {
        (self.0 & 1 != 0, self.0 & 2 != 0, self.0 & 4 != 0, self.0 & 8 != 0) 
    }

    pub fn is_empty(&self) -> bool {
        self.0 & 15 == 0
    }
}



impl Grid3D<VertexCell> {
    pub fn for_each_cell(&self, mut f: impl FnMut(&VertexCell,u16,u16,u16)) {
        for x in 0..self.size_x() {
            for y in 0..self.size_y() {
                for z in 0..self.size_z() {
                    f(self.get(x,y,z), x, y, z);
                }
            }
        }
    }

    pub fn insert_vertex(&mut self, cell: &Cell, vertex: &Vertex, vertices: &mut Vec<Vertex>) {
        let vertex_cell = VertexCell::new(vertices.len(), cell);
        *self.get_mut(cell.x, cell.y, cell.z) = vertex_cell;
        vertices.push(*vertex);
    }

    pub fn fetch_triangles(&mut self, vertices: &Vec<Vertex>) -> Vec<Triangle<Vertex>> {
        let mut triangles = Vec::new();

        self.for_each_cell(|cell, x, y, z|{
            let changes = cell.changes();
            if changes.0 != changes.3 && y > 0 && z > 0 {
                let quad = Quad(
                    self.get(x, y - 1, z - 1).vertex(vertices),
                    self.get(x, y, z - 1).vertex(vertices),
                    self.get(x, y, z).vertex(vertices),
                    self.get(x, y - 1, z).vertex(vertices),
                ).swap(changes.0);
                let tris = quad.make_triangles();
                triangles.push(tris.0);
                triangles.push(tris.1);
            }
     
            if changes.1 != changes.3 && x > 0 && z > 0 {
                let quad = Quad(
                    self.get(x - 1, y,z - 1).vertex(vertices),
                    self.get(x, y, z - 1).vertex(vertices),
                    self.get(x, y, z).vertex(vertices),
                    self.get(x - 1, y,z).vertex(vertices),
                ).swap(!changes.1);
                let tris = quad.make_triangles();
                triangles.push(tris.0);
                triangles.push(tris.1);
            }
    
            if changes.2 != changes.3 && x > 0 && y > 0 {
                let quad = Quad(
                    self.get(x - 1, y-1,z).vertex(vertices),
                    self.get(x, y-1,z).vertex(vertices),
                    self.get(x, y,z).vertex(vertices),
                    self.get(x - 1, y,z).vertex(vertices),
                ).swap(changes.2);
                let tris = quad.make_triangles();
                triangles.push(tris.0);
                triangles.push(tris.1);
            }
        });

        triangles
    }

    pub fn fetch_triangles_indices(&self) -> Vec<Triangle<u32>> {
        let mut indices = Vec::new();
        self.for_each_cell(|cell, x, y, z|{
            let changes = cell.changes();
            if changes.0 != changes.3 && y > 0 && z > 0 {
                let quad = Quad(
                    self.get(x, y - 1, z - 1).vertex_id(),
                    self.get(x, y, z - 1).vertex_id(),
                    self.get(x, y, z).vertex_id(),
                    self.get(x, y - 1, z).vertex_id(),
                ).swap(changes.0);
                let tris = quad.make_triangles();
                indices.push(tris.0);
                indices.push(tris.1);
            }
     
            if changes.1 != changes.3 && x > 0 && z > 0 {
                let quad = Quad(
                    self.get(x - 1, y,z - 1).vertex_id(),
                    self.get(x, y,z - 1).vertex_id(),
                    self.get(x, y,z).vertex_id(),
                    self.get(x - 1, y,z).vertex_id(),
                ).swap(!changes.1);
                let tris = quad.make_triangles();
                indices.push(tris.0);
                indices.push(tris.1);
            }
    
            if changes.2 != changes.3 && x > 0 && y > 0 {
                let quad = Quad(
                    self.get(x - 1, y-1,z).vertex_id(),
                    self.get(x, y-1,z).vertex_id(),
                    self.get(x, y,z).vertex_id(),
                    self.get(x - 1, y,z).vertex_id(),
                ).swap(changes.2);
                let tris = quad.make_triangles();
                indices.push(tris.0);
                indices.push(tris.1);
            }
        });

        indices
    }

}
