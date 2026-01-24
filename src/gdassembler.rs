use godot::classes::Node;
use godot::prelude::*;

use crate::neozasm::assemble as assemblenz;

#[derive(GodotClass)]
#[class(base=Node, init)]
struct AssemblrNode {
    #[base]
    base: Base<Node>,
}

#[godot_api]
impl AssemblrNode {
    #[func]
    fn assemble(&mut self, source: String) -> PackedByteArray {
        let result: Vec<u8> = assemblenz(&source);
        PackedByteArray::from(result.as_slice())
    }
}