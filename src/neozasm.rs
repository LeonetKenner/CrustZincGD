use std::collections::HashMap;

pub fn assemble(source: &str) -> Vec<u16> {
    let opcodes = HashMap::from([
        ("mov", 1), ("add", 2), ("sub", 3), ("mul", 4),
        ("and", 5), ("or", 6), ("xor", 7), ("not", 8),
        ("jmp", 9), ("jml", 10), ("jmle", 11), ("jmb", 12),
        ("jmbe", 13), ("jme", 14), ("jmne", 15),
        ("save", 16), ("load", 17), ("push", 18), ("pop", 19),
        ("halt", 20), ("shl", 21), ("shr", 22),
    ]);

    let mut labels = HashMap::new();
    let mut lines = vec![];
    for (i, line) in source.lines().enumerate() {
        let line = line.split(';').next().unwrap_or("").trim();
        if line.is_empty() { continue; }

        if line.ends_with(':') {
            let mut name = line.trim_end_matches(':').trim();
            if let Some(stripped) = name.strip_prefix("label ") {
                name = stripped;
            }
            labels.insert(name.to_string(), lines.len() as u16);
        } else {
            lines.push((i + 1, line.to_string()));
        }
    }

    let reg_index = |s: &str| match s {
        "A" => 0, "B" => 1, "C" => 2, "D" => 3,
        "IP" => 4, "SS" => 5, "SO" => 6, "MS" => 7,
        "MO" => 8, "I" => 9, "O" => 10, "ST" => 11,
        _ => panic!("Invalid register name '{}'.", s),
    };
    let resolve = |s: &str, labels: &HashMap<String, u16>| -> u16 {
        if let Ok(n) = s.parse::<u16>() { return n; }
        if let Some(&v) = labels.get(s) { return v; }
        reg_index(s)
    };

    let mut result = vec![];
    for (lineno, line) in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }
        let name = parts[0];
        let opcode_num = *opcodes.get(name)
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
                if args.len() != 2 { panic!("'mov' needs 2 args at line {}", lineno); }
                a = resolve(&args[0], &labels);
                b = resolve(&args[1], &labels);
                if args[0].parse::<u16>().is_ok() { f |= 1; }
            }
            "add" | "sub" | "and" | "or" | "xor" | "shl" | "shr" => {
                if args.len() != 3 { panic!("'{}' needs 3 args at line {}", name, lineno); }
                a = resolve(&args[0], &labels);
                b = resolve(&args[1], &labels);
                c = resolve(&args[2], &labels);
                if args[0].parse::<u16>().is_ok() { f |= 1; }
                if args[1].parse::<u16>().is_ok() { f |= 2; }
            }
            "mul" => {
                if args.len() != 2 { panic!("'mul' needs 2 args at line {}", lineno); }
                a = resolve(&args[0], &labels);
                b = resolve(&args[1], &labels);
                if args[0].parse::<u16>().is_ok() { f |= 1; }
                if args[1].parse::<u16>().is_ok() { f |= 2; }
            }
            "not" => {
                if args.len() != 2 { panic!("'not' needs 2 args at line {}", lineno); }
                a = resolve(&args[0], &labels);
                c = resolve(&args[1], &labels);
                if args[0].parse::<u16>().is_ok() { f |= 1; }
            }
            "jmp" => {
                if args.len() != 1 { panic!("'jmp' needs 1 arg at line {}", lineno); }
                c = resolve(&args[0], &labels);
                if args[0].parse::<u16>().is_ok() { f |= 4; }
            }
            "jml" | "jmle" | "jmb" | "jmbe" | "jme" | "jmne" => {
                if args.len() != 3 { panic!("'{}' needs 3 args at line {}", name, lineno); }
                a = resolve(&args[0], &labels);
                b = resolve(&args[1], &labels);
                c = resolve(&args[2], &labels);
                if args[0].parse::<u16>().is_ok() { f |= 1; }
                if args[1].parse::<u16>().is_ok() { f |= 2; }
                if args[2].parse::<u16>().is_ok() { f |= 4; }
            }
            "save" | "push" => {
                if args.len() != 1 { panic!("'{}' needs 1 arg at line {}", name, lineno); }
                a = resolve(&args[0], &labels);
                if args[0].parse::<u16>().is_ok() { f |= 1; }
            }
            "load" => {
                if args.len() != 1 { panic!("'load' needs 1 arg at line {}", lineno); }
                c = resolve(&args[0], &labels);
                if args[0].parse::<u16>().is_ok() { f |= 4; }
            }
            "pop" => {
                if args.len() != 1 { panic!("'pop' needs 1 arg at line {}", lineno); }
                a = resolve(&args[0], &labels);
            }
            "halt" => {}
            _ => panic!("Unimplemented instruction '{}' on line {}", name, lineno),
        }

        let header = (f << 13) | opcode;
        result.extend_from_slice(&[header, a, b, c]);
    }

    if let Some(&halt_code) = opcodes.get("halt") {
        let header = (0 << 13) | (halt_code - 1);
        result.extend_from_slice(&[header, 0, 0, 0]);
    }

    result
}
