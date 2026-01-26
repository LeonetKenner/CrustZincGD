const MEM_SIZE: usize = 65536;
const NUM_REGS: usize = 12;

pub const REG_A: usize = 0;
pub const REG_B: usize = 1;
pub const REG_C: usize = 2;
pub const REG_D: usize = 3;
pub const REG_IP: usize = 4;
pub const REG_SS: usize = 5;
pub const REG_SO: usize = 6;
pub const REG_MS: usize = 7;
pub const REG_MO: usize = 8;
pub const REG_I: usize = 9;
pub const REG_O: usize = 10;
pub const REG_ST: usize = 11;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum StepResult {
    Continue,
    Halt,
    Input,
    Output(u16),
}

#[derive(Debug, Clone, Copy)]
#[repr(u16)]
enum Opcode {
    Mov = 0,
    Add = 1,
    Sub = 2,
    Mul = 3,
    And = 4,
    Or = 5,
    Xor = 6,
    Not = 7,
    Jmp = 8,
    Jml = 9,
    Jmle = 10,
    Jmb = 11,
    Jmbe = 12,
    Jme = 13,
    Jmne = 14,
    Save = 15,
    Load = 16,
    Push = 17,
    Pop = 18,
    Halt = 19,
    Shl = 20,
    Shr = 21,
    Int = 22,
    Dsave = 23, 
}

impl From<u16> for Opcode {
    fn from(op: u16) -> Self {
        match op {
            0 => Opcode::Mov,
            1 => Opcode::Add,
            2 => Opcode::Sub,
            3 => Opcode::Mul,
            4 => Opcode::And,
            5 => Opcode::Or,
            6 => Opcode::Xor,
            7 => Opcode::Not,
            8 => Opcode::Jmp,
            9 => Opcode::Jml,
            10 => Opcode::Jmle,
            11 => Opcode::Jmb,
            12 => Opcode::Jmbe,
            13 => Opcode::Jme,
            14 => Opcode::Jmne,
            15 => Opcode::Save,
            16 => Opcode::Load,
            17 => Opcode::Push,
            18 => Opcode::Pop,
            19 => Opcode::Halt,
            20 => Opcode::Shl,
            21 => Opcode::Shr,
            22 => Opcode::Int,
            23 => Opcode::Dsave,
            _ => Opcode::Halt,
        }
    }
}

pub struct Emulator {
    regs: [u16; NUM_REGS],
    ram: [u8; MEM_SIZE],
    is_signed: bool,
}

impl Default for Emulator {
    fn default() -> Self {
        Emulator {
            regs: [0; NUM_REGS],
            ram: [0; MEM_SIZE],
            is_signed: false,
        }
    }
}

impl Emulator {
    pub fn new() -> Self {
        let mut emu = Emulator::default();
        emu.reset();
        emu
    }

    pub fn mem(&self) -> &[u8] { &self.ram }
    pub fn mem_mut(&mut self) -> &mut [u8] { &mut self.ram }
    pub fn regs(&self) -> &[u16] { &self.regs }
    pub fn regs_mut(&mut self) -> &mut [u16] { &mut self.regs }

    pub fn reset(&mut self) {
        self.regs = [0; NUM_REGS];
        self.ram = [0; MEM_SIZE];
        self.regs[REG_SS] = 16384; 
        self.regs[REG_MS] = 32768; 
        self.is_signed = false;
    }

    fn read_reg(&self, idx: u16) -> u16 {
        self.regs[idx as usize]
    }

    fn write_reg(&mut self, idx: u16, val: u16) {
        self.regs[idx as usize] = val;
        if idx as usize == REG_ST {
            self.is_signed = (val & 1) == 1;
        }
    }

    fn read_mem_u16(&self, addr: usize) -> u16 {
        if addr + 1 >= MEM_SIZE { return 0; }
        u16::from_le_bytes([self.ram[addr], self.ram[addr + 1]])
    }

    fn write_mem_u16(&mut self, addr: usize, val: u16) {
        if addr + 1 >= MEM_SIZE { return; }
        let bytes = val.to_le_bytes();
        self.ram[addr] = bytes[0];
        self.ram[addr + 1] = bytes[1];
    }

    pub fn load_program(&mut self, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            if i < MEM_SIZE {
                self.ram[i] = byte;
            }
        }
    }

    pub fn r_i(&self, f: u16, param: u16, bit: u16) -> u16 {
        if (f >> bit) & 1 != 0 {
            param 
        } else {
            self.read_reg(param) 
        }
    }

    pub fn step(&mut self) -> StepResult {
        let ip = self.read_reg(REG_IP as u16);
        self.write_reg(REG_IP as u16, ip.wrapping_add(1));
        
        let pc_addr = (ip as usize) * 8;
        
        if pc_addr + 6 >= MEM_SIZE { return StepResult::Halt; }

        let instr = self.read_mem_u16(pc_addr);
        let f = (instr >> 13) & 0x7;
        let opcode = instr & 0x1FFF;
        
        let a = self.read_mem_u16(pc_addr + 2);
        let b = self.read_mem_u16(pc_addr + 4);
        let c = self.read_mem_u16(pc_addr + 6);

        let op = Opcode::from(opcode);

        match op {
            Opcode::Mov => {
                let val = self.r_i(f, a, 0);
                self.write_reg(b, val);
            }
            Opcode::Add => {
                let va = if (f & 1) == 1 { a } else { self.read_reg(a) };
                let vb = if (f & 2) == 2 { b } else { self.read_reg(b) };
                
                let res: u32;
                let overflow_limit: u32;

                if self.is_signed {
                    let sa = va as i16 as i32;
                    let sb = vb as i16 as i32;
                    let sum = sa + sb;
                    res = sum as u32;
                    overflow_limit = 32768; 
                } else {
                    res = (va as u32) + (vb as u32);
                    overflow_limit = 65535;
                }

                if res > overflow_limit {
                   let st = self.read_reg(REG_ST as u16);
                   self.write_reg(REG_ST as u16, st | 0b10); 
                }
                
                self.write_reg(c, res as u16);
            }
            Opcode::Sub => {
                let va = self.r_i(f, a, 0);
                let vb = self.r_i(f, b, 1);
                self.write_reg(c, va.wrapping_sub(vb));
            }
            Opcode::Mul => {
                let va = self.r_i(f, a, 0);
                let vb = self.r_i(f, b, 1);
                let res = (va as u32) * (vb as u32);
                self.write_reg(REG_C as u16, (res >> 16) as u16);
                self.write_reg(REG_D as u16, (res & 0xFFFF) as u16);
            }
            Opcode::And => self.write_reg(c, self.r_i(f, a, 0) & self.r_i(f, b, 1)),
            Opcode::Or  => self.write_reg(c, self.r_i(f, a, 0) | self.r_i(f, b, 1)),
            Opcode::Xor => self.write_reg(c, self.r_i(f, a, 0) ^ self.r_i(f, b, 1)),
            Opcode::Not => self.write_reg(c, !self.r_i(f, a, 0)),
            
            Opcode::Jmp => self.write_reg(REG_IP as u16, self.r_i(f, c, 2)),
            Opcode::Jml => if self.r_i(f, a, 0) < self.r_i(f, b, 1) { self.write_reg(REG_IP as u16, self.r_i(f, c, 2)); },
            Opcode::Jmle => if self.r_i(f, a, 0) <= self.r_i(f, b, 1) { self.write_reg(REG_IP as u16, self.r_i(f, c, 2)); },
            Opcode::Jmb => if self.r_i(f, a, 0) > self.r_i(f, b, 1) { self.write_reg(REG_IP as u16, self.r_i(f, c, 2)); },
            Opcode::Jmbe => if self.r_i(f, a, 0) >= self.r_i(f, b, 1) { self.write_reg(REG_IP as u16, self.r_i(f, c, 2)); },
            Opcode::Jme => if self.r_i(f, a, 0) == self.r_i(f, b, 1) { self.write_reg(REG_IP as u16, self.r_i(f, c, 2)); },
            Opcode::Jmne => if self.r_i(f, a, 0) != self.r_i(f, b, 1) { self.write_reg(REG_IP as u16, self.r_i(f, c, 2)); },

            Opcode::Save => {
                let addr = if (f & 1) == 1 { a } else { self.read_reg(a) };
                let val = if ((f >> 1) & 1) == 1 { b } else { self.read_reg(b) };
                self.write_mem_u16(addr as usize, val);
            }
            Opcode::Load => {
                let addr = if (f & 1) == 1 { a } else { self.read_reg(a) };
                let val = self.read_mem_u16(addr as usize);
                self.write_reg(b, val);
            }
            
            Opcode::Push => {
                let sp = self.read_reg(REG_SS as u16).wrapping_add(self.read_reg(REG_SO as u16));
                self.write_mem_u16(sp as usize, self.r_i(f, a, 0));
                self.write_reg(REG_SO as u16, self.read_reg(REG_SO as u16).wrapping_add(2));
            }
            Opcode::Pop => {
                self.write_reg(REG_SO as u16, self.read_reg(REG_SO as u16).wrapping_sub(2));
                let sp = self.read_reg(REG_SS as u16).wrapping_add(self.read_reg(REG_SO as u16));
                let val = self.read_mem_u16(sp as usize);
                self.write_reg(a, val);
            }
            Opcode::Shl => self.write_reg(c, self.r_i(f, a, 0) << (self.r_i(f, b, 1) & 15)),
            Opcode::Shr => self.write_reg(c, self.r_i(f, a, 0) >> (self.r_i(f, b, 1) & 15)),
            
            Opcode::Int => {
                let int_id = self.r_i(f, a, 0);
                match int_id {
                    1 => return StepResult::Input,
                    2 => return StepResult::Output(2),
                    3 => return StepResult::Output(3),
                    4 => return StepResult::Output(4),
                    _ => {}
                }
            }
            Opcode::Dsave => {
                let addr = if (f & 1) == 1 { a } else { self.read_reg(a) };
                let val1 = if ((f >> 1) & 1) == 1 { b } else { self.read_reg(b) };
                let val2 = if ((f >> 2) & 1) == 1 { c } else { self.read_reg(c) };
                
                self.write_mem_u16(addr as usize, val1);
                self.write_mem_u16((addr + 2) as usize, val2);
            }
            Opcode::Halt => return StepResult::Halt,
        }

        StepResult::Continue
    }

    pub fn get_state_string(&self) -> String {
        format!(
            "A  = {:#06X} ({})\n\
             B  = {:#06X} ({})\n\
             C  = {:#06X} ({})\n\
             D  = {:#06X} ({})\n\
             IP = {:#06X} ({})\n\
             SS = {:#06X} ({})\n\
             SO = {:#06X} ({})\n\
             MS = {:#06X} ({})\n\
             VS = {:#06X} ({})\n\
             I  = {:#06X} ({})\n\
             O  = {:#06X} ({})\n\
             ST = {:#06X} ({})",
            self.regs[REG_A], self.regs[REG_A],
            self.regs[REG_B], self.regs[REG_B],
            self.regs[REG_C], self.regs[REG_C],
            self.regs[REG_D], self.regs[REG_D],
            self.regs[REG_IP], self.regs[REG_IP],
            self.regs[REG_SS], self.regs[REG_SS],
            self.regs[REG_SO], self.regs[REG_SO],
            self.regs[REG_MS], self.regs[REG_MS],
            self.regs[REG_MO], self.regs[REG_MO],
            self.regs[REG_I], self.regs[REG_I],
            self.regs[REG_O], self.regs[REG_O],
            self.regs[REG_ST], self.regs[REG_ST],
        )
    }
}