use std::collections::HashMap;

use arrayvec::ArrayVec;
use sv_parser::{HierarchicalInstance, SyntaxTree};

use crate::{Input, canonicalize_input, extract_id_and_ports, get_ports};

#[derive(Debug)]
pub struct Gate<'a> {
    name: &'a str,
    inputs: ArrayVec<&'a str, 7>,
    y: &'a str,
}

pub fn parse_gate<'a>(
    syntax_tree: &'a SyntaxTree,
    instance: &'a HierarchicalInstance,
) -> Option<Gate<'a>> {
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
        panic!("inputs smaller than 2 for gate")
    }

    Some(Gate {
        name,
        inputs,
        y: y?,
    })
}

#[derive(Debug)]
pub struct CanonicalGate<'a> {
    pub name: &'a str,
    pub inputs: ArrayVec<Input<'a>, 7>,
    pub y: &'a str,
}

/// To ignore not gates
pub fn canonicalize_gate<'a>(
    nand: &Gate<'a>,
    nots_by_output: &HashMap<&'a str, &'a str>,
) -> CanonicalGate<'a> {
    CanonicalGate {
        name: nand.name,
        inputs: nand
            .inputs
            .iter()
            .map(|input| canonicalize_input(input, nots_by_output))
            .collect(),
        y: nand.y,
    }
}
