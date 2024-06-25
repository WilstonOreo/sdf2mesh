# sdf2mesh

**sdf2mesh** generates triangle meshes from [SDFs](https://www.wikiwand.com/en/Signed_distance_function) defined as WGSL shaders using dual contouring and [WGPU](https://github.com/gfx-rs/wgpu).

![Cube with letters rendered from SDF](MartinCube.png "Cube")

## TL;DR

This example reads an SDF defined a file `examples/torus.sdf3d`, renders it with resolution 128x128x128 and writes it to `torus.stl`.
The resulting STL file can be viewed in a mesh viewer, like [MeshLab](https://www.meshlab.net/).

```shell
 cargo run sdf2mesh -- --sdf examples/torus.sdf3d  --resolution 128 --mesh torus.stl
```

## How it works

### What is an SDF?

Imagine you have a solid figure, like a sphere or a box. The *signed distance function* (SDF) for a point in space tells you how far away that point is from the nearest point on the boundary of the shape.

Now, the "signed" part is what makes it interesting. It not only tells you the distance but also whether the point is inside or outside the shape. If the point is inside, the distance is negative, and if it's outside, the distance is positive. If the point is exactly on the boundary, the distance is zero.

So, in simple terms, a signed distance function is like a magic function that, for any point in space, tells you both how far away you are from the nearest point on a shape and whether you're inside or outside that shape. It's a useful concept in computer graphics, physics simulations, and other fields where understanding distances to solid figure is important.

### SDF definition

The SDF is defined in an input file with extension `sdf3d`.
The file actually contains WGSL code.
A definition for a torus with radius 0.5 and width 0.2 looks like this:

```wgsl
use sdf3d::*;

fn sdf3d(p: vec3f) -> f32 {
    return sdf3d_torus(p, vec2(0.5, 0.2));
}
```

`use sdf3d::*` imports a set of functions for primitives. The function `sdf3d_torus` is such a predefined primitive.
The `sdf3d` function defines the actual SDF and has a position as input and a distance value as output.
That's all!

### SDF rendering

In order to convert the SDF into a triangle mesh, we need evaluate the SDF for each cell (X,Y,Z) with a certain resolution.
If the signs are different for the corners of the cell, we have to generate triangles.

This process is called [*Dual Contouring*](https://www.cs.wustl.edu/~taoju/research/dualContour.pdf).
You can find a nice visualization of the process [here](https://www.youtube.com/watch?v=B_5VBtpVuLQ).

In order to save memory, this process is done slice by slice instead rendering the SDF into a voxel grid.

### Running sdf2mesh

If we run the app with

```shell
 cargo run -- --sdf examples/torus.sdf3d  --resolution 128 --mesh torus.stl
```

we get the following output:

![Torus rendered from SDF](Torus.png "Torus")

## TODO

* SDF viewer app
* More primitives and built-in functions

## Known issues

* When the SDF is out of bounds, mesh generation might fail
* Crashes with resolutions above 2048 pixels
