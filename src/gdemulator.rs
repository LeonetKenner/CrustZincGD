use crate::emulator::{self as emu_module, StepResult};
use godot::classes::Node;
use godot::prelude::*;
use std::time::Instant; // Avoid name conflict

#[derive(GodotClass)]
#[class(base=Node)]
struct EmulatorNode {
    #[base]
    base: Base<Node>,

    emu: emu_module::Emulator,
}
#[godot_api]
impl INode for EmulatorNode {
    fn init(base: Base<Node>) -> Self {
        godot_print!("Initializing!");
        godot_print!("Initialized! i think...?");
        Self {
            base: base,
            emu: emu_module::Emulator::default(),
        }
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
    fn reset(&mut self) {
        self.emu.reset();
    }
    #[func]
    fn step(&mut self) -> bool {
        match self.emu.step() {
            StepResult::Continue => true,
            StepResult::Halt => {
                //godot_print!("Resetting...");
                //self.reset();
                false
            }
        }
    }
    #[func]
    fn print_state(&mut self) -> String {
        return self.emu.get_state_string();
    }
    #[func]
    fn benchmark(&mut self, steps: i32) -> f64 {
        let start = Instant::now();
        for _ in 0..steps {
            self.emu.step();
        }
        let elapsed = start.elapsed().as_secs_f64();
        steps as f64 / elapsed
    }
    #[func]
    fn benchmark_multi(&mut self, program: PackedByteArray, iterations: i32, n_tests: i32) -> f64 {
        // Convert PackedByteArray to Vec<u16> like in load_program
        let program_vec: Vec<u16> = program
            .as_slice()
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        let mut total_time = 0.0;

        for _ in 0..n_tests {
            self.emu.reset();
            self.emu.load_program(&program_vec);

            let start = Instant::now();
            for _ in 0..iterations {
                self.emu.step();
            }
            let elapsed = start.elapsed().as_secs_f64();
            total_time += elapsed;
        }

        let avg_time = total_time / n_tests as f64;
        iterations as f64 / avg_time
    }
}
