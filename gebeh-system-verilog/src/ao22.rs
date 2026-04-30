use crate::gate::CanonicalGate;

pub fn generate_code(gate: &CanonicalGate) -> String {
    let name = gate.y;

    format!(
        "let {name} = {} && {} || {} && {};",
        gate.inputs[0].generate_code(),
        gate.inputs[1].generate_code(),
        gate.inputs[2].generate_code(),
        gate.inputs[3].generate_code(),
    )
}
