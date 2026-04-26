use std::collections::HashMap;

use sv_parser::{HierarchicalInstance, SyntaxTree};

use crate::{Input, canonicalize_input, extract_id_and_ports, get_ports};

pub fn parse_dffr<'a>(syntax_tree: &'a SyntaxTree, instance: &'a HierarchicalInstance) -> Dffr<'a> {
    let (id, named) = extract_id_and_ports(syntax_tree, instance);

    let mut d = None;
    let mut clk = None;
    let mut r_n = None;
    let mut q = None;
    let mut q_n = None;

    for (id, expression) in get_ports(syntax_tree, named) {
        match id {
            "d" => d = expression,
            "clk" => clk = expression,
            "r_n" => r_n = expression,
            "q" => q = expression,
            "q_n" => q_n = expression,
            _ => panic!(),
        }
    }

    Dffr {
        d: d.unwrap(),
        name: id,
        clk: clk.unwrap(),
        q,
        q_n,
        r_n: r_n.unwrap(),
    }
}

#[derive(Debug)]
pub struct Dffr<'a> {
    pub name: &'a str,
    pub d: &'a str,
    pub clk: &'a str,
    pub r_n: &'a str,
    pub q: Option<&'a str>,
    pub q_n: Option<&'a str>,
}

#[derive(Debug)]
pub struct CanonicalDffr<'a> {
    pub name: &'a str,
    pub d: Input<'a>,
    pub clk: Input<'a>,
    pub r_n: Input<'a>,
    pub q: Option<&'a str>,
    pub q_n: Option<&'a str>,
}

/// To ignore not gates
pub fn canonicalize_dffr<'a>(
    dffr: &Dffr<'a>,
    nots_by_output: &HashMap<&'a str, &'a str>,
) -> CanonicalDffr<'a> {
    CanonicalDffr {
        name: dffr.name,
        clk: canonicalize_input(dffr.clk, nots_by_output),
        d: canonicalize_input(dffr.d, nots_by_output),
        r_n: canonicalize_input(dffr.r_n, nots_by_output),
        q: dffr.q,
        q_n: dffr.q_n,
    }
}
