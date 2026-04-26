use std::collections::HashMap;

use arrayvec::ArrayVec;
use sv_parser::{HierarchicalInstance, SyntaxTree};

use crate::{Input, canonicalize_input, extract_id_and_ports, get_ports};

#[derive(Debug)]
pub struct Nand<'a> {
    name: &'a str,
    inputs: ArrayVec<&'a str, 7>,
    y: &'a str,
}

pub fn parse_nand<'a>(
    syntax_tree: &'a SyntaxTree,
    instance: &'a HierarchicalInstance,
) -> Option<Nand<'a>> {
    let (name, named) = extract_id_and_ports(syntax_tree, instance);
    let mut inputs = ArrayVec::new();

    let mut y = None;

    for (id, expression) in get_ports(syntax_tree, named) {
        if id.starts_with("in") {
            inputs.push(expression.unwrap());
        } else if id == "y" {
            y = expression
        } else {
            panic!("{name} {id}")
        }
    }

    if inputs.len() < 2 {
        panic!("inputs smaller than 2 for nand")
    }

    Some(Nand {
        name,
        inputs,
        y: y?,
    })
}

#[derive(Debug)]
pub struct CanonicalNand<'a> {
    pub name: &'a str,
    pub inputs: ArrayVec<Input<'a>, 7>,
    pub y: &'a str,
}

/// To ignore not gates
pub fn canonicalize_nand<'a>(
    nand: &Nand<'a>,
    nots_by_output: &HashMap<&'a str, &'a str>,
) -> CanonicalNand<'a> {
    CanonicalNand {
        name: nand.name,
        inputs: nand
            .inputs
            .iter()
            .map(|input| canonicalize_input(input, nots_by_output))
            .collect(),
        y: nand.y,
    }
}

impl CanonicalNand<'_> {
    pub fn generate_code(&self) -> String {
        let name = self.y;

        let mut inputs = self.inputs.iter();
        let mut output = format!("let {name} = !({}", inputs.next().unwrap().generate_code());

        for input in inputs {
            output += &format!(" && {}", input.generate_code());
        }

        output + ");\n"
    }
}
