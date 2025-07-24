/*

╔══════════════════════════════════════════════════════════════════════════════╗
║                         🧠 ZINC Emulator (Rust)                          ║
╚══════════════════════════════════════════════════════════════════════════════╝

╔═══ Architecture Overview ═══════════════════════════════════════════════════╗
║ - 12 Registers:
║   - A, B, C, D      → General purpose
║   - IP              → Instruction pointer (in instruction units, not bytes)
║   - SS, SO          → Stack segment + stack offset
║   - MS, MO          → Memory segment + offset
║   - I, O, ST        → Misc / flag registers
║
║ - 64 KiB Memory: `ram: [u8; 65536]`
║   - Every instruction is 8 bytes: 2 bytes for opcode header, then 3×2-byte args
║
║ - `step()` runs one instruction. `load_program()` loads packed u16 code into memory.
║
║ - `r_i()` resolves arguments:
║   - If `f >> bit` is set, the parameter is treated as an immediate value + offset
║   - Otherwise, it's treated as a register index + optional offset (upper 4 bits)
║   - Offsets can be negative (values > 8 subtract from base reg)
║
║ - Operand encoding:
║   - Low 12 bits = reg or value
║   - High 4 bits = offset (+0 to +7, or -8 to -1)
║
║ - Opcode enum: 22 instructions (mov, add, sub, jmp, push, pop, etc.)
║ - Overflow behavior:
║   - `Add` uses `u32` with overflow detection
║   - `Sub` uses `wrapping_sub` and wraps underflow (e.g., 0 - 1 = 65535)
║   - `Mul` returns low 16 bits into D, sets C to 0
║   - Overflow flag set in REG_O bit 1 (mask `0b10`)
║
║ - No runtime panic in VM logic (everything uses wrapping ops)
║ - `get_state_string()` shows register states
║ - `Opcode::from()` handles unknown opcodes by halting
║
║ You can extend this by modifying:
║   - `Opcode` enum
║   - `step()` match arms
║   - `assemble()` logic (in assembler module)
║
║ Designed for usage with ZASM.
╚════════════════════════════════════════════════════════════════════════════╝

*/

const MEM_SIZE: usize = 65536;
const NUM_REGS: usize = 12;

const REG_A: usize = 0;
const REG_B: usize = 1;
const REG_C: usize = 2;
const REG_D: usize = 3;
const REG_IP: usize = 4;
const REG_SS: usize = 5;
const REG_SO: usize = 6;
const REG_MS: usize = 7;
const REG_MO: usize = 8;
const REG_I: usize = 9;
const REG_O: usize = 10;
const REG_ST: usize = 11;

#[derive(Clone, Copy, PartialEq)]
pub enum StepResult {
    Continue,
    Halt,
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

    pub fn reset(&mut self) {
        self.regs = [0; NUM_REGS];
        self.ram = [0; MEM_SIZE];
        self.regs[REG_SS] = 0x4000;
        self.regs[REG_MS] = 0x8000;
        self.regs[REG_MO] = 0;
        self.regs[REG_I] = 0;
        self.regs[REG_ST] = 0;
        self.is_signed = false;
    }

    fn read_reg(&self, idx: u16) -> u16 {
        self.regs[idx as usize]
    }

    fn write_reg(&mut self, idx: u16, val: u16) {
        self.regs[idx as usize] = val;
        if idx as usize == REG_O {
            self.is_signed = val & 1 != 0;
        }
    }

    fn read_mem_u16(&self, addr: usize) -> u16 {
        if addr + 1 >= MEM_SIZE {
            return 0;
        }
        let lo = self.ram[addr] as u16;
        let hi = self.ram[addr + 1] as u16;
        (hi << 8) | lo
    }

    fn write_mem_u16(&mut self, addr: usize, val: u16) {
        if addr + 1 >= MEM_SIZE {
            return;
        }
        self.ram[addr] = (val & 0xFF) as u8;
        self.ram[addr + 1] = (val >> 8) as u8;
    }

    pub fn load_program(&mut self, program: &[u16]) {
        for (i, word) in program.iter().enumerate() {
            self.write_mem_u16(i * 2, *word);
        }
    }

    pub fn r_i(&self, f: u16, param: u16, bit: u16) -> u16 {
        if (f >> bit) & 1 != 0 {
            let offset = (param >> 12) & 0xF;
            let value = param & 0x0FFF;
            value.wrapping_add(offset)
        } else {
            let reg_idx = param & 0x0FFF;
            let offset = (param >> 12) & 0xF;
            let reg_val = self.read_reg(reg_idx);
            if offset > 8 {
                reg_val.wrapping_sub(16 - offset)
            } else {
                reg_val.wrapping_add(offset)
            }
        }
    }

    pub fn step(&mut self) -> StepResult {
        let ip = self.read_reg(REG_IP as u16);
        let addr = ip as usize * 8;
        if addr + 6 >= MEM_SIZE {
            return StepResult::Halt;
        }

        let instr = self.read_mem_u16(addr);
        let f = (instr >> 13) & 0x7;
        let opcode = instr & 0x1FFF;
        let a = self.read_mem_u16(addr + 2);
        let b = self.read_mem_u16(addr + 4);
        let c = self.read_mem_u16(addr + 6);

        self.write_reg(REG_IP as u16, ip.wrapping_add(1));

        let va = self.r_i(f, a, 0);
        let vb = self.r_i(f, b, 1);
        let vc = self.r_i(f, c, 2);
        let op = Opcode::from(opcode);

        match op {
            Opcode::Mov => {
                let target_reg = b & 0xFFF;
                self.write_reg(target_reg, va);
            }
            Opcode::Add => {
                let target_reg = c & 0xFFF;
                let res = va as u32 + vb as u32;
                let max = if self.is_signed { 32767 } else { 65535 };
                if res > max {
                    self.write_reg(target_reg, 0);
                    self.write_reg(REG_O as u16, self.regs[REG_O] | 2);
                } else {
                    self.write_reg(target_reg, res as u16);
                    self.write_reg(REG_O as u16, self.regs[REG_O] & !2);
                }
            }
            Opcode::Sub => {
                let target_reg = c & 0xFFF;
                let res = va.wrapping_sub(vb);
                self.write_reg(target_reg, res);
            }
            Opcode::Mul => {
                let res = (va as u32) * (vb as u32);
                if res > 0xFFFF {
                    self.write_reg(REG_C as u16, 0);
                    self.write_reg(REG_D as u16, 0);
                } else {
                    self.write_reg(REG_C as u16, 0);
                    self.write_reg(REG_D as u16, res as u16);
                }
            }
            Opcode::And => {
                let target_reg = c & 0xFFF;
                self.write_reg(target_reg, va & vb);
            }
            Opcode::Or => {
                let target_reg = c & 0xFFF;
                self.write_reg(target_reg, va | vb);
            }
            Opcode::Xor => {
                let target_reg = c & 0xFFF;
                self.write_reg(target_reg, va ^ vb);
            }
            Opcode::Not => {
                let target_reg = b & 0xFFF;
                self.write_reg(target_reg, !va);
            }
            Opcode::Jmp => self.write_reg(REG_IP as u16, vc),
            Opcode::Jml => {
                if va < vb {
                    self.write_reg(REG_IP as u16, vc)
                }
            }
            Opcode::Jmle => {
                if va <= vb {
                    self.write_reg(REG_IP as u16, vc)
                }
            }
            Opcode::Jmb => {
                if va > vb {
                    self.write_reg(REG_IP as u16, vc)
                }
            }
            Opcode::Jmbe => {
                if va >= vb {
                    self.write_reg(REG_IP as u16, vc)
                }
            }
            Opcode::Jme => {
                if va == vb {
                    self.write_reg(REG_IP as u16, vc)
                }
            }
            Opcode::Jmne => {
                if va != vb {
                    self.write_reg(REG_IP as u16, vc)
                }
            }
            Opcode::Save => {
                let addr = self.regs[REG_MS].wrapping_add(self.regs[REG_IP]) as usize;
                self.write_mem_u16(addr, va);
            }
            Opcode::Load => {
                let addr = self.regs[REG_MS].wrapping_add(self.regs[REG_IP]) as usize;
                let val = self.read_mem_u16(addr);
                let target_reg = a & 0xFFF;
                self.write_reg(target_reg, val);
            }
            Opcode::Push => {
                let addr = self.regs[REG_SS].wrapping_add(self.regs[REG_SO]) as usize;
                self.write_mem_u16(addr, va);
                self.regs[REG_SO] = self.regs[REG_SO].wrapping_add(2);
            }
            Opcode::Pop => {
                self.regs[REG_SO] = self.regs[REG_SO].wrapping_sub(2);
                let addr = self.regs[REG_SS].wrapping_add(self.regs[REG_SO]) as usize;
                let val = self.read_mem_u16(addr);
                let target_reg = a & 0xFFF;
                self.write_reg(target_reg, val);
            }
            Opcode::Halt => return StepResult::Halt,
            Opcode::Shl => {
                let target_reg = c & 0xFFF;
                self.write_reg(target_reg, va << (vb & 15));
            }
            Opcode::Shr => {
                let target_reg = c & 0xFFF;
                self.write_reg(target_reg, va >> (vb & 15));
            }
        }

        StepResult::Continue
    }

    pub fn get_state_string(&self) -> String {
        format!(
            "A  = {:#06X} ({})\nB  = {:#06X} ({})\nC  = {:#06X} ({})\nD  = {:#06X} ({})\nIP = {:#06X} ({})\nSS = {:#06X} ({})\nSO = {:#06X} ({})\nMS = {:#06X} ({})\nMO = {:#06X} ({})\nI  = {:#06X} ({})\nO  = {:#06X} ({})\nST = {:#06X} ({})",
            self.regs[REG_A], self.regs[REG_A], self.regs[REG_B], self.regs[REG_B],
            self.regs[REG_C], self.regs[REG_C], self.regs[REG_D], self.regs[REG_D],
            self.regs[REG_IP], self.regs[REG_IP], self.regs[REG_SS], self.regs[REG_SS],
            self.regs[REG_SO], self.regs[REG_SO], self.regs[REG_MS], self.regs[REG_MS],
            self.regs[REG_MO], self.regs[REG_MO], self.regs[REG_I], self.regs[REG_I],
            self.regs[REG_O], self.regs[REG_O], self.regs[REG_ST], self.regs[REG_ST],
        )
    }
}
