
pub trait Vertex: Sized + Copy {
    
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BasicVertex {
    pub position: glam::Vec3,
}

#[derive(bytemuck::Zeroable, bytemuck::Pod)]
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TexturedVertex {
    pub position: glam::Vec3,
    pub normal: glam::Vec3,
    pub tex_coord: glam::Vec2,
}

impl Vertex for BasicVertex { }

impl Vertex for TexturedVertex { }

#[derive(Clone, Debug)]
pub struct Mesh<V: Vertex> {
    pub vertices: Vec<V>,
    pub indices: Vec<u32>,
}

impl<V: Vertex> Mesh<V> {
    
}
