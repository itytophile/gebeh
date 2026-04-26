use std::collections::HashMap;

use sv_parser::{HierarchicalInstance, SyntaxTree};

use crate::{Input, canonicalize_input, extract_id_and_ports, get_ports};

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

#[derive(Debug)]
pub struct CanonicalNorLatch<'a> {
    pub name: &'a str,
    pub s: Input<'a>,
    pub r: Input<'a>,
    pub q: Option<&'a str>,
    pub q_n: Option<&'a str>,
}

/// To ignore not gates
pub fn canonicalize_nor_latch<'a>(
    nor_latch: &NorLatch<'a>,
    nots_by_output: &HashMap<&'a str, &'a str>,
) -> CanonicalNorLatch<'a> {
    CanonicalNorLatch {
        name: nor_latch.name,
        s: canonicalize_input(nor_latch.s, nots_by_output),
        r: canonicalize_input(nor_latch.r, nots_by_output),
        q: nor_latch.q,
        q_n: nor_latch.q_n,
    }
}

impl CanonicalNorLatch<'_> {
    pub fn generate_code(&self) -> String {
        let name = self.name;

        if self.q.is_none() && self.q_n.is_none() {
            return String::new();
        }

        let s = self.s.generate_code();
        let r = self.r.generate_code();

        let mut output = format!("let {name}_output = self.{name}.update({s}, {r});\n");

        if let Some(q) = self.q {
            output += &format!("let {q} = name_output;\n");
        }

        if let Some(q_n) = self.q_n {
            output += &format!("let {q_n} = !name_output;\n");
        }

        output
    }
}
