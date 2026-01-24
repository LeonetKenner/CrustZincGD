use std::collections::HashMap;

const REG_NAMES: &[&str] = &[
    "A", "B", "C", "D", "IP", "SS", "SO", "MS", "MO", "I", "O", "ST"
];

fn get_opcode_val(op: &str) -> Option<u16> {
    match op.to_lowercase().as_str() {
        "mov" => Some(0),
        "add" => Some(1),
        "sub" => Some(2),
        "mul" => Some(3),
        "and" => Some(4),
        "or" => Some(5),
        "xor" => Some(6),
        "not" => Some(7),
        "jmp" => Some(8),
        "jml" => Some(9),
        "jmle" => Some(10),
        "jmb" => Some(11),
        "jmbe" => Some(12),
        "jme" => Some(13),
        "jmne" => Some(14),
        "save" => Some(15),
        "load" => Some(16),
        "push" => Some(17),
        "pop" => Some(18),
        "halt" => Some(19),
        "shl" => Some(20),
        "shr" => Some(21),
        "int" => Some(22),
        "dsave" => Some(23),
        _ => None,
    }
}

fn get_param_mapping(op: &str) -> Option<&'static [usize]> {
    match op.to_lowercase().as_str() {
        "mov" | "mul" | "save" | "load" => Some(&[0, 1]),
        "not" | "jmp" => Some(&[0, 2]),
        "push" | "pop" | "int" => Some(&[0]),
        "halt" => Some(&[]),
        _ => Some(&[0, 1, 2]),
    }
}

#[derive(Debug)]
enum Arg {
    Register(u16),
    Number(u16),
    Label(String),
}

struct ParsedInstruction {
    opcode: String,
    args: Vec<Arg>,
}

pub struct Assembler {
    instructions: Vec<ParsedInstruction>,
    data_bytes: Vec<u8>,
    static_data: HashMap<String, u16>,
    constants: HashMap<String, u16>,
}

impl Assembler {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            data_bytes: Vec::new(),
            static_data: HashMap::new(),
            constants: HashMap::new(),
        }
    }

    fn resolve_reg(&self, token: &str) -> Option<u16> {
        REG_NAMES.iter().position(|&r| r == token.to_uppercase()).map(|i| i as u16)
    }

    fn parse_number(&self, token: &str) -> Option<u16> {
        let clean = token.replace(',', ""); 
        
        let (num_str, radix) = if clean.starts_with("0x") {
            (&clean[2..], 16)
        } else if clean.starts_with("0b") {
            (&clean[2..], 2)
        } else if clean.starts_with("0o") {
            (&clean[2..], 8)
        } else {
            (clean.as_str(), 10)
        };
        u16::from_str_radix(num_str, radix).ok()
    }

    fn emit_le_u16(val: u16, out: &mut Vec<u8>) {
        out.push((val & 0xFF) as u8);
        out.push((val >> 8) as u8);
    }
    
    fn emit_le_u32(val: u32, out: &mut Vec<u8>) {
        out.push((val & 0xFF) as u8);
        out.push(((val >> 8) & 0xFF) as u8);
        out.push(((val >> 16) & 0xFF) as u8);
        out.push(((val >> 24) & 0xFF) as u8);
    }

    pub fn assemble(&mut self, source: &str) -> Vec<u8> {
        for line in source.lines() {
            let clean_line = line.split(&[';', '/'][..]).next().unwrap_or("").trim().replace(',', " ");
            let parts: Vec<&str> = clean_line.split_whitespace().collect();

            if parts.is_empty() { continue; }

            let command = parts[0];

            if command == "label" {
                if parts.len() < 2 { continue; }
                let name = parts[1].replace(":", "");
                
                self.static_data.insert(name, self.instructions.len() as u16);

            } else if command == "const" {
                if parts.len() < 3 { continue; }
                let name = parts[1].replace(":", "");
                if let Some(val) = self.parse_number(parts.last().unwrap()) {
                    self.constants.insert(name, val);
                }

            } else if ["db", "dw", "dd"].contains(&command) {
                for token in &parts[1..] {
                    let val = self.parse_number(token)
                        .or_else(|| self.constants.get(*token).copied())
                        .unwrap_or(0);
                    
                    match command {
                        "db" => self.data_bytes.push(val as u8),
                        "dw" => Self::emit_le_u16(val, &mut self.data_bytes),
                        "dd" => Self::emit_le_u32(val as u32, &mut self.data_bytes),
                        _ => {}
                    }
                }

            } else if get_opcode_val(command).is_some() {
                let mut args = Vec::new();
                for token in &parts[1..] {
                    if let Some(reg) = self.resolve_reg(token) {
                        args.push(Arg::Register(reg));
                    } else if let Some(val) = self.parse_number(token) {
                        args.push(Arg::Number(val));
                    } else if let Some(val) = self.constants.get(*token) {
                        args.push(Arg::Number(*val));
                    } else {
                        let clean_lbl = token.replace(",", "");
                        args.push(Arg::Label(clean_lbl));
                    }
                }
                
                self.instructions.push(ParsedInstruction {
                    opcode: command.to_string(),
                    args
                });
            }
        }

        let bss_offset = ((self.instructions.len() + 1) * 8) as u16;
        self.static_data.insert("bss".to_string(), bss_offset);

        let mut final_bytes = Vec::new();

        for instr in &self.instructions {
            let op_val = get_opcode_val(&instr.opcode).unwrap();
            let mapping = get_param_mapping(&instr.opcode).unwrap_or(&[]);
            
            let mut flags = 0u16;
            let mut params = [0u16; 3];

            for (i, arg) in instr.args.iter().enumerate() {
                if i >= mapping.len() { break; }
                let target_slot = mapping[i]; 

                match arg {
                    Arg::Register(r) => {
                        params[target_slot] = *r;
                    },
                    Arg::Number(n) => {
                        flags |= 1 << target_slot;
                        params[target_slot] = *n;
                    },
                    Arg::Label(name) => {
                        flags |= 1 << target_slot;
                        params[target_slot] = *self.static_data.get(name).unwrap_or(&0);
                    }
                }
            }

            let instr_word = (flags << 13) | op_val;

            Self::emit_le_u16(instr_word, &mut final_bytes);
            Self::emit_le_u16(params[0], &mut final_bytes);
            Self::emit_le_u16(params[1], &mut final_bytes);
            Self::emit_le_u16(params[2], &mut final_bytes);
        }

        let halt_op = get_opcode_val("halt").unwrap();
        Self::emit_le_u16(halt_op, &mut final_bytes);
        Self::emit_le_u16(0, &mut final_bytes);
        Self::emit_le_u16(0, &mut final_bytes);
        Self::emit_le_u16(0, &mut final_bytes);

        final_bytes.extend_from_slice(&self.data_bytes);

        final_bytes
    }
}

pub fn assemble(source: &str) -> Vec<u8> {
    let mut asm = Assembler::new();
    asm.assemble(source)
}