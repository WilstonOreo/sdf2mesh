use std::io::Write;

use crate::*;

pub struct STLWriter<'a> {
    writer: &'a mut dyn Write,
}

impl<'a> STLWriter<'a> {
    pub fn new(mut w: &'a mut dyn Write) -> Self {
        writeln!(&mut w, "solid").unwrap();

        Self { writer: w }
    }

    pub fn write_triangle(&mut self, tri: &Triangle<Vertex>) -> std::io::Result<()> {
        let n = tri.normal();
        writeln!(&mut self.writer, "facet normal {} {} {}", n.x, n.y, n.z)?;
        writeln!(&mut self.writer, "\touter loop")?;
        writeln!(
            &mut self.writer,
            "\t\tvertex {} {} {}",
            tri.0.pos.x, tri.0.pos.y, tri.0.pos.z
        )?;
        writeln!(
            &mut self.writer,
            "\t\tvertex {} {} {}",
            tri.1.pos.x, tri.1.pos.y, tri.1.pos.z
        )?;
        writeln!(
            &mut self.writer,
            "\t\tvertex {} {} {}",
            tri.2.pos.x, tri.2.pos.y, tri.2.pos.z
        )?;
        writeln!(&mut self.writer, "\tendloop")?;
        writeln!(&mut self.writer, "endfacet")?;
        Ok(())
    }
}

impl<'a> Drop for STLWriter<'a> {
    fn drop(&mut self) {
        writeln!(self.writer, "endsolid").unwrap();
    }
}

pub struct PLYWriter<'a> {
    writer: &'a mut dyn Write,
}

impl<'a> PLYWriter<'a> {
    pub fn new(mut w: &'a mut dyn Write) -> std::io::Result<Self> {
        writeln!(&mut w, "ply")?;
        writeln!(&mut w, "format ascii 1.0")?;
        writeln!(&mut w, "comment written by rust-sdf")?;

        Ok(Self { writer: w })
    }

    pub fn header_element_vertex3d(&mut self, len: usize) -> std::io::Result<()> {
        writeln!(&mut self.writer, "element vertex {len}")?;
        writeln!(&mut self.writer, "property float x")?;
        writeln!(&mut self.writer, "property float y")?;
        writeln!(&mut self.writer, "property float z")?;
        writeln!(&mut self.writer, "property float nx")?;
        writeln!(&mut self.writer, "property float ny")?;
        writeln!(&mut self.writer, "property float nz")?;
        Ok(())
    }

    pub fn header_element_vertex3d_with_colors(&mut self, len: usize) -> std::io::Result<()> {
        self.header_element_vertex3d(len)?;
        writeln!(&mut self.writer, "property uchar red")?;
        writeln!(&mut self.writer, "property uchar green")?;
        writeln!(&mut self.writer, "property uchar blue")?;
        Ok(())
    }

    pub fn header_element_face(&mut self, len: usize) -> std::io::Result<()> {
        writeln!(&mut self.writer, "element face {len}")?;
        writeln!(&mut self.writer, "property list uchar int vertex_index")?;
        Ok(())
    }

    pub fn header_end(&mut self) -> std::io::Result<()> {
        writeln!(&mut self.writer, "end_header")?;
        Ok(())
    }

    pub fn vertex(&mut self, v: &Vertex) -> std::io::Result<()> {
        writeln!(
            &mut self.writer,
            "{} {} {} {} {} {}",
            v.pos.x, v.pos.y, v.pos.z, v.normal.x, v.normal.y, v.normal.z
        )?;
        Ok(())
    }

    pub fn vertices(&mut self, v: &Vec<Vertex>) -> std::io::Result<()> {
        for vertex in v {
            self.vertex(vertex)?;
        }
        Ok(())
    }

    pub fn vertex_color<T: std::fmt::Display>(
        &mut self,
        v: &Vertex,
        color: &(T, T, T),
    ) -> std::io::Result<()> {
        writeln!(
            &mut self.writer,
            "{} {} {} {} {} {} {} {} {}",
            v.pos.x,
            v.pos.y,
            v.pos.z,
            v.normal.x,
            v.normal.y,
            v.normal.z,
            color.0,
            color.1,
            color.2
        )?;
        Ok(())
    }

    pub fn tri_face(&mut self, tri: &Triangle<u32>) -> std::io::Result<()> {
        writeln!(&mut self.writer, "3 {} {} {}", tri.0, tri.1, tri.2)?;
        Ok(())
    }

    pub fn tri_faces(&mut self, tri_faces: &Vec<Triangle<u32>>) -> std::io::Result<()> {
        for face in tri_faces {
            self.tri_face(face)?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct TriangleMesh {
    vertices: Vec<Vertex>,
    triangle_indices: Vec<Triangle<u32>>,
}

impl TriangleMesh {
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.triangle_indices.clear();
    }

    pub fn fetch_triangles(&self) -> Vec<Triangle<Vertex>> {
        let mut triangles = Vec::with_capacity(self.triangle_indices.len());
        for t in &self.triangle_indices {
            triangles.push(Triangle(
                self.vertices[t.0 as usize],
                self.vertices[t.1 as usize],
                self.vertices[t.2 as usize],
            ));
        }
        triangles
    }

    pub fn write_stl_to_file(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        use std::fs::File;

        let mut f = std::io::BufWriter::new(File::create(path)?);
        let mut stl_writer = STLWriter::new(&mut f);

        let triangles = self.fetch_triangles();

        for triangle in &triangles {
            stl_writer.write_triangle(triangle)?;
        }

        Ok(())
    }

    pub fn write_to_file(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        match path.as_ref().extension().unwrap_or_default().to_str().unwrap_or_default() {
            "stl" => self.write_stl_to_file(path)?,
            "ply" => self.write_ply_to_file(path)?,
            ext => log::error!("Unknown file extension: {ext}")
        }
        Ok(())
    }

    pub fn write_ply_to_file(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        use std::fs::File;
        // Write vertices and faces to PLY
        let mut ply_f = std::io::BufWriter::new(File::create(path)?);
        let mut ply_writer = PLYWriter::new(&mut ply_f)?;

        ply_writer.header_element_vertex3d(self.vertices.len())?;
        ply_writer.header_element_face(self.triangle_indices.len())?;
        ply_writer.header_end()?;

        ply_writer.vertices(&self.vertices)?;
        ply_writer.tri_faces(&self.triangle_indices)
    }
}

pub struct VertexListItem {
    pub cell: (u16, u16, u16),
    pub sign_changes: (bool, bool, bool, bool),
    pub vertex: Vertex,
}

impl VertexListItem {
    pub fn index(&self) -> u64 {
        Self::compute_index(self.cell.0, self.cell.1, self.cell.2)
    }

    pub fn compute_index(x: u16, y: u16, z: u16) -> u64 {
        x as u64 | ((y as u64) << 16) | ((z as u64) << 32)
    }
}

#[derive(Default)]
pub struct VertexList(Vec<VertexListItem>);

impl VertexList {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn insert(
        &mut self,
        cell: (u16, u16, u16),
        sign_changes: (bool, bool, bool, bool),
        vertex: Vertex,
    ) {
        self.0.push(VertexListItem {
            cell,
            sign_changes,
            vertex,
        });
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn fetch_vertices(&self) -> Vec<Vertex> {
        let mut vertices = Vec::with_capacity(self.len());
        for vertex_item in &self.0 {
            vertices.push(vertex_item.vertex);
        }
        vertices
    }

    pub fn fetch_triangle_indices(&self) -> Vec<Triangle<u32>> {
        let mut indices = Vec::with_capacity(self.0.len() * 2);

        for vertex_item in &self.0 {
            let changes = vertex_item.sign_changes;
            let x = vertex_item.cell.0;
            let y = vertex_item.cell.1;
            let z = vertex_item.cell.2;

            if changes.0 != changes.3 && y > 0 && z > 0 {
                let quad = Quad(
                    self.vertex_index(x, y - 1, z - 1),
                    self.vertex_index(x, y, z - 1),
                    self.vertex_index(x, y, z),
                    self.vertex_index(x, y - 1, z),
                )
                .swap(changes.0);
                let tris = quad.make_triangles();
                indices.push(tris.0);
                indices.push(tris.1);
            }

            if changes.1 != changes.3 && x > 0 && z > 0 {
                let quad = Quad(
                    self.vertex_index(x - 1, y, z - 1),
                    self.vertex_index(x, y, z - 1),
                    self.vertex_index(x, y, z),
                    self.vertex_index(x - 1, y, z),
                )
                .swap(!changes.1);
                let tris = quad.make_triangles();
                indices.push(tris.0);
                indices.push(tris.1);
            }

            if changes.2 != changes.3 && x > 0 && y > 0 {
                let quad = Quad(
                    self.vertex_index(x - 1, y - 1, z),
                    self.vertex_index(x, y - 1, z),
                    self.vertex_index(x, y, z),
                    self.vertex_index(x - 1, y, z),
                )
                .swap(changes.2);
                let tris = quad.make_triangles();
                indices.push(tris.0);
                indices.push(tris.1);
            }
        }

        indices
    }

    fn vertex_index(&self, x: u16, y: u16, z: u16) -> u32 {
        self.0
            .binary_search_by_key(&VertexListItem::compute_index(x, y, z), |item| item.index())
            .unwrap() as u32
    }
}

impl From<VertexList> for TriangleMesh {
    fn from(l: VertexList) -> Self {
        TriangleMesh {
            vertices: l.fetch_vertices(),
            triangle_indices: l.fetch_triangle_indices(),
        }
    }
}
