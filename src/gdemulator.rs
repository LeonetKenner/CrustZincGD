use crate::emulator::{self as emu_module, StepResult};
use godot::classes::Node;
use godot::prelude::*;
use std::time::Instant;

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
        Self {
            base: base,
            emu: emu_module::Emulator::default(),
        }
    }
}

#[godot_api]
impl EmulatorNode {
    
    #[signal]
    fn screen_update_requested();

    #[signal]
    fn video_mode_changed(mode: i32);

    #[signal]
    fn input_requested();
    
    #[signal]
    fn paused(); // ADDED THIS
    
    #[signal]
    fn halted();

    #[func]
    fn load_program(&mut self, program: PackedByteArray) {
        self.emu.load_program(program.as_slice());
    }

    #[func]
    fn reset(&mut self) {
        self.emu.reset();
    }

    #[func]
    fn step(&mut self) -> i32 {
        match self.emu.step() {
            StepResult::Continue => 0,
            StepResult::Halt => {
                self.base_mut().emit_signal("halted", &[]);
                1
            },
            StepResult::Input => {
                self.base_mut().emit_signal("input_requested", &[]);
                2 
            },
            StepResult::Output(code) => {
                match code {
                    2 => { self.base_mut().emit_signal("screen_update_requested", &[]); },
                    3 => { 
                        let mode = self.emu.regs()[crate::emulator::REG_A] as i32;
                        self.base_mut().emit_signal("video_mode_changed", &[mode.to_variant()]); 
                    },
                    4 => { // ADDED THIS
                        self.base_mut().emit_signal("paused", &[]);
                        return 3;
                    },
                    _ => {}
                }
                0 
            }
        }
    }

    #[func]
    fn execute_batch(&mut self, max_steps: i32) -> i32 {
        for _ in 0..max_steps {
            match self.emu.step() {
                StepResult::Continue => continue, 
                StepResult::Halt => {
                    self.base_mut().emit_signal("halted", &[]);
                    return 1;
                },
                StepResult::Input => {
                    self.base_mut().emit_signal("input_requested", &[]);
                    return 2;
                },
                StepResult::Output(code) => {
                    match code {
                        2 => { self.base_mut().emit_signal("screen_update_requested", &[]); },
                        3 => { 
                            let mode = self.emu.regs()[crate::emulator::REG_A] as i32;
                            self.base_mut().emit_signal("video_mode_changed", &[mode.to_variant()]); 
                        },
                        4 => {
                            self.base_mut().emit_signal("paused", &[]);
                            return 3;
                        },
                        _ => {}
                    }
                }
            }
        }
        0 
    }

    #[func]
    fn send_input(&mut self, key_code: i32) {
        self.emu.regs_mut()[crate::emulator::REG_B] = key_code as u16;
    }

    #[func]
    fn get_memory_range(&mut self, start_addr: i32, length: i32) -> PackedByteArray {
        let mem = self.emu.mem();
        let start = start_addr as usize;
        let end = start + (length as usize);

        if start >= mem.len() || end > mem.len() {
            godot_print!("Error: Memory access out of bounds");
            return PackedByteArray::new();
        }

        PackedByteArray::from(&mem[start..end])
    }
    
    #[func]
    fn set_register(&mut self, index: i32, value: i32) {
        if index >= 0 && index < 12 {
            self.emu.regs_mut()[index as usize] = value as u16;
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
        let program_slice = program.as_slice();
        let mut total_time = 0.0;

        for _ in 0..n_tests {
            self.emu.reset();
            self.emu.load_program(program_slice);

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