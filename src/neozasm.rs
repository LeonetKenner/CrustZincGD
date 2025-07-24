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
    let s = s.trim();

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
        return resolve_expr(lhs.trim(), symbols).wrapping_sub(resolve_expr(rhs.trim(), symbols));
    }

    if let Some(reg) = reg_index(s) {
        return reg;
    }

    panic!("Invalid operand '{}'", s);
}

fn resolve_operand(s: &str, symbols: &HashMap<String, u16>) -> (u16, bool) {
    let s = s.trim();

    if let Ok(n) = s.parse::<u16>() {
        return (n, true);
    }

    if let Some((lhs, rhs)) = s.split_once('+') {
        let lhs_trim = lhs.trim();
        let rhs_trim = rhs.trim();

        if let Some(reg) = reg_index(lhs_trim) {
            let offset = resolve_expr(rhs_trim, symbols);
            if offset > 15 {
                panic!("Offset too large (max 15): {}", offset);
            }
            return ((offset << 12) | reg, false);
        } else if let Some(reg) = reg_index(rhs_trim) {
            let offset = resolve_expr(lhs_trim, symbols);
            if offset > 15 {
                panic!("Offset too large (max 15): {}", offset);
            }
            return ((offset << 12) | reg, false);
        }
    }

    if let Some((lhs, rhs)) = s.split_once('-') {
        let lhs_trim = lhs.trim();
        let rhs_trim = rhs.trim();

        if let Some(reg) = reg_index(lhs_trim) {
            let offset = resolve_expr(rhs_trim, symbols);
            if offset > 15 {
                panic!("Offset too large (max 15): {}", offset);
            }
            let encoded = ((16 - offset) << 12) | reg;
            return (encoded, false);
        } else if let Some(reg) = reg_index(rhs_trim) {
            let offset = resolve_expr(lhs_trim, symbols);
            if offset > 15 {
                panic!("Offset too large (max 15): {}", offset);
            }
            let encoded = ((16 - offset) << 12) | reg;
            return (encoded, false);
        }
    }

    if let Some(reg) = reg_index(s) {
        return (reg, false);
    }

    if symbols.contains_key(s) || s.contains('+') || s.contains('-') {
        return (resolve_expr(s, symbols), true);
    }

    panic!("Invalid operand '{}'", s);
}

pub fn assemble(source: &str) -> Vec<u16> {
    let opcodes = HashMap::from([
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
            let label = line
                .trim_end_matches(':')
                .trim()
                .strip_prefix("label ")
                .unwrap_or_else(|| line.trim_end_matches(':').trim())
                .to_string();
            labels.insert(label, lines.len() as u16);
        } else {
            lines.push((i + 1, line.to_string()));
        }
    }

    labels.extend(consts.iter().map(|(k, &v)| (k.clone(), v)));

    let mut result = vec![];

    for (lineno, line) in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let name = parts[0];
        let opcode_num = *opcodes
            .get(name)
            .unwrap_or_else(|| panic!("Unknown instruction '{}' on line {}", name, lineno));
        let opcode = opcode_num - 1;

        let joined = parts[1..].join("");
        let args: Vec<String> = joined
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let (mut a, mut b, mut c, mut f) = (0, 0, 0, 0);

        match name {
            "mov" => {
                assert_eq!(args.len(), 2);
                let (av, ai) = resolve_operand(&args[0], &labels);
                let (bv, _) = resolve_operand(&args[1], &labels);
                a = av;
                b = bv;
                if ai {
                    f |= 1;
                }
            }
            "add" | "sub" | "and" | "or" | "xor" | "shl" | "shr" => {
                assert_eq!(args.len(), 3);
                let (av, ai) = resolve_operand(&args[0], &labels);
                let (bv, bi) = resolve_operand(&args[1], &labels);
                let (cv, _) = resolve_operand(&args[2], &labels);
                a = av;
                b = bv;
                c = cv;
                if ai {
                    f |= 1;
                }
                if bi {
                    f |= 2;
                }
            }
            "mul" => {
                assert_eq!(args.len(), 2);
                let (av, ai) = resolve_operand(&args[0], &labels);
                let (bv, bi) = resolve_operand(&args[1], &labels);
                a = av;
                b = bv;
                if ai {
                    f |= 1;
                }
                if bi {
                    f |= 2;
                }
            }
            "not" => {
                assert_eq!(args.len(), 2);
                let (av, ai) = resolve_operand(&args[0], &labels);
                let (cv, _) = resolve_operand(&args[1], &labels);
                a = av;
                c = cv;
                if ai {
                    f |= 1;
                }
            }
            "jmp" => {
                assert_eq!(args.len(), 1);
                let (cv, ci) = resolve_operand(&args[0], &labels);
                c = cv;
                if ci {
                    f |= 4;
                }
            }
            "jml" | "jmle" | "jmb" | "jmbe" | "jme" | "jmne" => {
                assert_eq!(args.len(), 3);
                let (av, ai) = resolve_operand(&args[0], &labels);
                let (bv, bi) = resolve_operand(&args[1], &labels);
                let (cv, ci) = resolve_operand(&args[2], &labels);
                a = av;
                b = bv;
                c = cv;
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
                assert_eq!(args.len(), 1);
                let (av, ai) = resolve_operand(&args[0], &labels);
                a = av;
                if ai {
                    f |= 1;
                }
            }
            "load" => {
                assert_eq!(args.len(), 1);
                let (cv, ci) = resolve_operand(&args[0], &labels);
                c = cv;
                if ci {
                    f |= 4;
                }
            }
            "pop" => {
                assert_eq!(args.len(), 1);
                let (av, _) = resolve_operand(&args[0], &labels);
                a = av;
            }
            "halt" => continue,
            _ => panic!("Unknown instruction '{}' on line {}", name, lineno),
        }

        let header = (f << 13) | opcode;
        result.extend_from_slice(&[header, a, b, c]);
    }

    let halt_opcode = (opcodes["halt"] - 1) & 0x1FFF;
    result.extend_from_slice(&[halt_opcode, 0, 0, 0]);

    result
}
