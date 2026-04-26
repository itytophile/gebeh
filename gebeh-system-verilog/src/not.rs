use sv_parser::{HierarchicalInstance, SyntaxTree};

use crate::{extract_id_and_ports, get_ports};

pub fn parse_not<'a>(
    syntax_tree: &'a SyntaxTree,
    instance: &'a HierarchicalInstance,
) -> Option<Not<'a>> {
    let (id, named) = extract_id_and_ports(syntax_tree, instance);

    let mut input = None;
    let mut y = None;

    for (id, expression) in get_ports(syntax_tree, named) {
        match id {
            "y" => y = expression,
            "in" => input = expression,
            _ => panic!(),
        }
    }

    Some(Not {
        name: id,
        input: input.unwrap(),
        y: y?,
    })
}

#[derive(Debug)]
pub struct Not<'a> {
    pub name: &'a str,
    pub input: &'a str,
    pub y: &'a str,
}
