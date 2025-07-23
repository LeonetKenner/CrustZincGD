use std::collections::HashMap;

fn reg_index(s: &str) -> Option<u16> {
    match s {
        "A" => Some(0),
        "B" => Some(1),
        "C" => Some(2),
        "D" => Some(3),
        "IP" => Some(4),
        "SS" => Some(5),
        "SO" => Some(6),
        "MS" => Some(7),
        "MO" => Some(8),
        "I" => Some(9),
        "O" => Some(10),
        "ST" => Some(11),
        _ => None,
    }
}

fn resolve_expr(s: &str, symbols: &HashMap<String, u16>) -> u16 {
    if let Ok(n) = s.parse::<u16>() {
        return n;
    }
    if let Some(&val) = symbols.get(s) {
        return val;
    }
    if let Some((lhs, rhs)) = s.split_once('+') {
        return resolve_expr(lhs.trim(), symbols) + resolve_expr(rhs.trim(), symbols);
    }
    if let Some((lhs, rhs)) = s.split_once('-') {
        return resolve_expr(lhs.trim(), symbols) - resolve_expr(rhs.trim(), symbols);
    }

    panic!("Unknown constant or label: '{}'", s);
}

// New function to handle register+constant expressions
fn resolve_reg_expr(s: &str, symbols: &HashMap<String, u16>) -> Option<(u16, u16)> {
    if let Some((lhs, rhs)) = s.split_once('+') {
        let lhs = lhs.trim();
        let rhs = rhs.trim();

        // Try lhs as register, rhs as constant/expression
        if let Some(reg_idx) = reg_index(lhs) {
            let offset = resolve_expr(rhs, symbols);
            return Some((reg_idx, offset));
        }

        // Try rhs as register, lhs as constant/expression
        if let Some(reg_idx) = reg_index(rhs) {
            let offset = resolve_expr(lhs, symbols);
            return Some((reg_idx, offset));
        }
    }

    if let Some((lhs, rhs)) = s.split_once('-') {
        let lhs = lhs.trim();
        let rhs = rhs.trim();

        // Only support register - constant (not constant - register)
        if let Some(reg_idx) = reg_index(lhs) {
            let offset = resolve_expr(rhs, symbols);
            // Use wrapping_sub to handle negative offsets properly
            return Some((reg_idx, (-(offset as i32)) as u16));
        }
    }

    None
}

fn resolve_operand(s: &str, symbols: &HashMap<String, u16>) -> (u16, u16, bool) {
    // Check if it's a plain number
    if let Ok(n) = s.parse::<u16>() {
        return (n, 0, true); // (value, offset, is_immediate)
    }

    // Check if it's a plain register
    if let Some(reg) = reg_index(s) {
        return (reg, 0, false); // (reg_index, offset, is_immediate)
    }

    // Check if it's a register+constant expression
    if let Some((reg_idx, offset)) = resolve_reg_expr(s, symbols) {
        return (reg_idx, offset, false); // Register with offset
    }

    // Check if it's a pure constant/label expression
    if symbols.contains_key(s) || s.contains('+') || s.contains('-') {
        let val = resolve_expr(s, symbols);
        return (val, 0, true); // (value, offset, is_immediate)
    }

    panic!("Invalid operand '{}'", s);
}

pub fn assemble(source: &str) -> Vec<u16> {
    let opcodes: HashMap<&str, u16> = HashMap::from([
        ("mov", 1),
        ("add", 2),
        ("sub", 3),
        ("mul", 4),
        ("and", 5),
        ("or", 6),
        ("xor", 7),
        ("not", 8),
        ("jmp", 9),
        ("jml", 10),
        ("jmle", 11),
        ("jmb", 12),
        ("jmbe", 13),
        ("jme", 14),
        ("jmne", 15),
        ("save", 16),
        ("load", 17),
        ("push", 18),
        ("pop", 19),
        ("halt", 20),
        ("shl", 21),
        ("shr", 22),
    ]);

    let mut consts = HashMap::new();
    let mut labels = HashMap::new();
    let mut lines = vec![];

    // First pass: collect constants and labels
    for (i, line) in source.lines().enumerate() {
        let line = line.split(';').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("const ") {
            if let Some((key, val)) = rest.split_once(':') {
                let name = key.trim().to_string();
                let value = resolve_expr(val.trim(), &consts);
                consts.insert(name, value);
                continue;
            }
        } else if line.ends_with(':') {
            let label = line.trim_end_matches(':').trim().to_string();
            labels.insert(label, lines.len() as u16);
        } else {
            lines.push((i + 1, line.to_string()));
        }
    }

    // Merge constants into label map for lookup
    labels.extend(consts.iter().map(|(k, &v)| (k.clone(), v)));

    let mut result = vec![];

    for (lineno, line) in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let name = parts[0];
        let opcode_val = *opcodes
            .get(name)
            .unwrap_or_else(|| panic!("Unknown instruction '{}' on line {}", name, lineno));
        let opcode = opcode_val - 1;

        let joined = parts[1..].join("");
        let args: Vec<String> = joined
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let mut a = 0;
        let mut b = 0;
        let mut c = 0;
        let mut f = 0;
        let mut a_offset = 0;
        let mut b_offset = 0;
        let mut c_offset = 0;

        match name {
            "mov" => {
                assert_eq!(args.len(), 2, "'mov' needs 2 args at line {}", lineno);
                let (av, ao, ai) = resolve_operand(&args[0], &labels);
                let (bv, bo, _) = resolve_operand(&args[1], &labels);
                a = av;
                b = bv;
                a_offset = ao;
                b_offset = bo;
                if ai {
                    f |= 1;
                }
            }
            "add" | "sub" | "and" | "or" | "xor" | "shl" | "shr" => {
                assert_eq!(args.len(), 3, "'{}' needs 3 args at line {}", name, lineno);
                let (av, ao, ai) = resolve_operand(&args[0], &labels);
                let (bv, bo, bi) = resolve_operand(&args[1], &labels);
                let (cv, co, _) = resolve_operand(&args[2], &labels);
                a = av;
                b = bv;
                c = cv;
                a_offset = ao;
                b_offset = bo;
                c_offset = co;
                if ai {
                    f |= 1;
                }
                if bi {
                    f |= 2;
                }
            }
            "mul" => {
                assert_eq!(args.len(), 2, "'mul' needs 2 args at line {}", lineno);
                let (av, ao, ai) = resolve_operand(&args[0], &labels);
                let (bv, bo, bi) = resolve_operand(&args[1], &labels);
                a = av;
                b = bv;
                a_offset = ao;
                b_offset = bo;
                if ai {
                    f |= 1;
                }
                if bi {
                    f |= 2;
                }
            }
            "not" => {
                assert_eq!(args.len(), 2, "'not' needs 2 args at line {}", lineno);
                let (av, ao, ai) = resolve_operand(&args[0], &labels);
                let (cv, co, _) = resolve_operand(&args[1], &labels);
                a = av;
                c = cv;
                a_offset = ao;
                c_offset = co;
                if ai {
                    f |= 1;
                }
            }
            "jmp" => {
                assert_eq!(args.len(), 1, "'jmp' needs 1 arg at line {}", lineno);
                let (cv, co, ci) = resolve_operand(&args[0], &labels);
                c = cv;
                c_offset = co;
                if ci {
                    f |= 4;
                }
            }
            "jml" | "jmle" | "jmb" | "jmbe" | "jme" | "jmne" => {
                assert_eq!(args.len(), 3, "'{}' needs 3 args at line {}", name, lineno);
                let (av, ao, ai) = resolve_operand(&args[0], &labels);
                let (bv, bo, bi) = resolve_operand(&args[1], &labels);
                let (cv, co, ci) = resolve_operand(&args[2], &labels);
                a = av;
                b = bv;
                c = cv;
                a_offset = ao;
                b_offset = bo;
                c_offset = co;
                if ai {
                    f |= 1;
                }
                if bi {
                    f |= 2;
                }
                if ci {
                    f |= 4;
                }
            }
            "save" | "push" => {
                assert_eq!(args.len(), 1, "'{}' needs 1 arg at line {}", name, lineno);
                let (av, ao, ai) = resolve_operand(&args[0], &labels);
                a = av;
                a_offset = ao;
                if ai {
                    f |= 1;
                }
            }
            "load" => {
                assert_eq!(args.len(), 1, "'load' needs 1 arg at line {}", lineno);
                let (cv, co, ci) = resolve_operand(&args[0], &labels);
                c = cv;
                c_offset = co;
                if ci {
                    f |= 4;
                }
            }
            "pop" => {
                assert_eq!(args.len(), 1, "'pop' needs 1 arg at line {}", lineno);
                let (av, ao, _) = resolve_operand(&args[0], &labels);
                a = av;
                a_offset = ao;
            }
            "halt" => {
                // Handled at the end
                continue;
            }
            _ => panic!("Unimplemented instruction '{}' on line {}", name, lineno),
        }

        let instr = (f << 13) | opcode;
        // Encode offsets into high bits of operands (you may need to adjust this based on your instruction format)
        let a_encoded = a | ((a_offset & 0xF) << 12);
        let b_encoded = b | ((b_offset & 0xF) << 12);
        let c_encoded = c | ((c_offset & 0xF) << 12);

        result.extend_from_slice(&[instr, a_encoded, b_encoded, c_encoded]);
    }

    // Final halt
    let halt = (0 << 13) | (opcodes["halt"] - 1);
    result.extend_from_slice(&[halt, 0, 0, 0]);

    result
}
