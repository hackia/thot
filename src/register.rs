use crate::ast::Level;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegBase {
    Ka,
    Ib,
    Da,
    Ba,
    Si,
    Di,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegReg {
    Ds,
    Es,
    Ss,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegKind {
    General(RegBase),
    Segment(SegReg),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegSpec {
    pub kind: RegKind,
    pub level: Level,
}

fn level_from_prefix(prefix: &str) -> Option<Level> {
    match prefix {
        "m" => Some(Level::Medium),
        "h" => Some(Level::High),
        "v" => Some(Level::Very),
        "e" => Some(Level::Extreme),
        "x" => Some(Level::Xenith),
        _ => None,
    }
}

pub fn parse_register(name: &str) -> RegSpec {
    let (level, base_name) = match name.len() {
        2 => (Level::Base, name),
        3 => {
            let (prefix, rest) = name.split_at(1);
            let level = level_from_prefix(prefix)
                .unwrap_or_else(|| panic!("Unknown register prefix: %{name}"));
            (level, rest)
        }
        _ => panic!("Unknown register: %{name}"),
    };

    let kind = match base_name {
        "ka" => RegKind::General(RegBase::Ka),
        "ib" => RegKind::General(RegBase::Ib),
        "da" => RegKind::General(RegBase::Da),
        "ba" => RegKind::General(RegBase::Ba),
        "si" => RegKind::General(RegBase::Si),
        "di" => RegKind::General(RegBase::Di),
        "ds" => RegKind::Segment(SegReg::Ds),
        "es" => RegKind::Segment(SegReg::Es),
        "ss" => RegKind::Segment(SegReg::Ss),
        _ => panic!("Unknown register: %{name}"),
    };

    if matches!(kind, RegKind::Segment(_)) && level != Level::Base {
        panic!("Segment registers cannot be prefixed: %{name}");
    }

    RegSpec { kind, level }
}

pub fn parse_general_register(name: &str) -> RegSpec {
    let spec = parse_register(name);
    if !matches!(spec.kind, RegKind::General(_)) {
        panic!("Expected a general register, found %{name}");
    }
    spec
}

pub fn reg_code(base: RegBase) -> u8 {
    match base {
        RegBase::Ka => 0,
        RegBase::Ib => 1,
        RegBase::Da => 2,
        RegBase::Ba => 3,
        RegBase::Si => 6,
        RegBase::Di => 7,
    }
}

pub fn seg_code(seg: SegReg) -> u8 {
    match seg {
        SegReg::Es => 0,
        SegReg::Ss => 2,
        SegReg::Ds => 3,
    }
}

pub fn modrm_imm(base: RegBase, op: u8) -> u8 {
    0xC0 | (op << 3) | reg_code(base)
}

pub fn modrm_reg_reg(dest: RegBase, src: RegBase) -> u8 {
    0xC0 | (reg_code(src) << 3) | reg_code(dest)
}

pub fn modrm_mov_reg_rm(dest: RegBase, src: RegBase) -> u8 {
    0xC0 | (reg_code(dest) << 3) | reg_code(src)
}

pub fn ensure_same_level(
    context: &str,
    left: &str,
    left_level: Level,
    right: &str,
    right_level: Level,
) {
    if left_level != right_level {
        panic!(
            "Size mismatch in {}: %{} ({}) vs %{} ({})",
            context, left, left_level, right, right_level
        );
    }
}

pub fn ensure_supported_level(context: &str, reg: &str, level: Level) {
    if level > Level::High {
        panic!(
            "Unsupported register size in {}: %{} ({})",
            context, reg, level
        );
    }
}

pub fn channel_max(level: Level) -> u128 {
    let bits = (level.bits() / 2) as u32;
    if bits >= 128 {
        u128::MAX
    } else {
        (1u128 << bits) - 1
    }
}

pub fn ensure_helix_fits(
    context: &str,
    reg: &str,
    level: Level,
    ra: u128,
    apophis: u128,
) {
    let max = channel_max(level);
    if ra > max || apophis > max {
        panic!(
            "Overflow in {} for %{} ({}): ra={} apophis={} (max per channel = {})",
            context, reg, level, ra, apophis, max
        );
    }
}

pub fn ensure_number_fits(context: &str, reg: &str, level: Level, value: i32) {
    let bits = level.bits() as u32;
    if bits >= 32 {
        return;
    }
    let max: i128 = (1i128 << (bits - 1)) - 1;
    let min: i128 = -(1i128 << (bits - 1));
    let v = value as i128;
    if v < min || v > max {
        panic!(
            "Overflow in {} for %{} ({}): value={} (min={} max={})",
            context, reg, level, value, min, max
        );
    }
}
