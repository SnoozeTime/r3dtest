//! Sprites that will always face the player. It uses the perspective projection matrix

use luminance::linear::M44;
use luminance::shader::program::Uniform;
use luminance_derive::{Semantics, UniformInterface, Vertex};

//
//#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Semantics)]
//pub enum VertexSementics {
//    /// useless. Not empty
//}
//
//#[allow(dead_code)]
//#[derive(Vertex, Debug)]
//#[vertex(sem = "VertexSementics")]
//pub struct Vertex {}

#[derive(Debug, UniformInterface)]
pub struct ShaderInterface {
    pub projection: Uniform<M44>,
    pub view: Uniform<M44>,
    pub model: Uniform<M44>,
}

pub struct BillboardRenderer {}
