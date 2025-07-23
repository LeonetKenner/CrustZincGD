use godot::prelude::*;
use godot::classes::Node;
use crate::emulator::{self as emu_module, StepResult}; // Avoid name conflict

#[derive(GodotClass)]
#[class(base=Node)]
struct EmulatorNode {
    #[base]
    base: Base<Node>,

    emu: emu_module::Emulator,
}
#[godot_api]
impl INode for EmulatorNode{
    fn init(base: Base<Node>)->Self{
        godot_print!("Initializing!");
        godot_print!("Initialized! i think...?");
        Self { base: base, emu: emu_module::Emulator::default() }
    }
}
#[godot_api]
impl EmulatorNode {
    #[func] // Makes it accessible from GDScript
    fn load_program(&mut self, program: PackedByteArray) {
        let vec: Vec<u16> = program
            .as_slice()
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        self.emu.load_program(&vec);
    }
    #[func]
    fn reset(&mut self){
        self.emu.reset();
    }
    #[func]
    fn step(&mut self){
        if self.emu.step()==StepResult::Continue {
            godot_warn!("Stepped!");
        }
        else if self.emu.step()==StepResult::Halt{
            godot_warn!("Halted!");
            godot_print!("Resetting...");
            self.reset();
        }
    }
    #[func]
    fn print_state(&mut self){
        godot_print!("{}", self.emu.get_state_string());

    }
}
