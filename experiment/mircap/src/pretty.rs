use crate::image::{ModuleImage, Operand};

pub fn format_module(image: &ModuleImage) -> String {
    let mut out = String::new();
    out.push_str(&format!("module {} {}\n", image.module.id, image.module.name));
    out.push_str(&format!("types: {}\n", image.types.len()));
    out.push_str(&format!("symbols: {}\n", image.symbols.len()));
    for function in &image.functions {
        out.push_str(&format!("function {} symbol:{} blocks:{}\n", function.id, function.symbol, function.blocks.len()));
    }
    out
}

pub fn format_operand(operand: &Operand) -> String {
    match operand {
        Operand::Value(id) => format!("v:{id}"),
        Operand::ImmI32(value) => format!("i:{value}"),
        Operand::ImmU32(value) => format!("u:{value}"),
        Operand::Block(id) => format!("b:{id}"),
        Operand::Function(id) => format!("f:{id}"),
        Operand::Symbol(id) => format!("s:{id}"),
        Operand::Type(id) => format!("t:{id}"),
    }
}

