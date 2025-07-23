use godot::prelude::*;
use godot::classes::Node;

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
        let result: Vec<u16> = assemblenz(&source);

        let mut byte_vec = Vec::with_capacity(result.len() * 2);
        for word in result {
            byte_vec.push((word & 0xFF) as u8);         // Lower byte
            byte_vec.push((word >> 8) as u8);           // Upper byte
        }

        PackedByteArray::from(byte_vec)
    }
}
