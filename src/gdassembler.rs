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
    fn assemble(&mut self, source: String) -> PackedInt32Array {
        let result: Vec<u16> = assemblenz(&source);
        let result_32: Vec<i32> = result.iter().map(|&x| x as i32).collect();
        PackedInt32Array::from(result_32)
    }
}
