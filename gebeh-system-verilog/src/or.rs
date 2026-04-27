use crate::gate::CanonicalGate;

pub fn generate_code(gate: &CanonicalGate) -> String {
    let name = gate.y;

    let mut inputs = gate.inputs.iter();
    let mut output = format!("let {name} = {}", inputs.next().unwrap().generate_code());

    for input in inputs {
        output += &format!(" || {}", input.generate_code());
    }

    output + ";\n"
}
