use sv_parser::{HierarchicalInstance, SyntaxTree};

use crate::{extract_id_and_ports, get_ports};

#[derive(Debug)]
pub struct NorLatch<'a> {
    name: &'a str,
    s: &'a str,
    r: &'a str,
    q: Option<&'a str>,
    q_n: Option<&'a str>,
}

pub fn parse_nor_latch<'a>(
    syntax_tree: &'a SyntaxTree,
    instance: &'a HierarchicalInstance,
) -> NorLatch<'a> {
    let (id, named) = extract_id_and_ports(syntax_tree, instance);

    let mut s = None;
    let mut r = None;
    let mut q = None;
    let mut q_n = None;

    for (id, expression) in get_ports(syntax_tree, named) {
        match id {
            "s" => s = expression,
            "r" => r = expression,
            "q" => q = expression,
            "q_n" => q_n = expression,
            _ => panic!(),
        }
    }

    NorLatch {
        name: id,
        s: s.unwrap(),
        r: r.unwrap(),
        q,
        q_n,
    }
}
