use std::collections::HashMap;

use sv_parser::{HierarchicalInstance, SyntaxTree};

use crate::{Input, canonicalize_input, extract_id_and_ports, get_ports};

pub fn parse_drlatch_ee<'a>(
    syntax_tree: &'a SyntaxTree,
    instance: &'a HierarchicalInstance,
) -> DrlatchEe<'a> {
    let (id, named) = extract_id_and_ports(syntax_tree, instance);

    let mut d = None;
    let mut ena = None;
    let mut r_n = None;
    let mut q = None;
    let mut q_n = None;

    for (id, expression) in get_ports(syntax_tree, named) {
        match id {
            "d" => d = expression,
            "ena" => ena = expression,
            "r_n" => r_n = expression,
            "q" => q = expression,
            "q_n" => q_n = expression,
            "ena_n" => {
                // ignored because inverse of ena
            }
            _ => panic!(),
        }
    }

    DrlatchEe {
        d: d.unwrap(),
        name: id,
        ena: ena.unwrap(),
        q,
        q_n,
        r_n: r_n.unwrap(),
    }
}

#[derive(Debug)]
pub struct DrlatchEe<'a> {
    pub name: &'a str,
    pub d: &'a str,
    pub ena: &'a str,
    pub r_n: &'a str,
    pub q: Option<&'a str>,
    pub q_n: Option<&'a str>,
}

#[derive(Debug)]
pub struct CanonicalDrlatchEe<'a> {
    pub name: &'a str,
    pub d: Input<'a>,
    pub ena: Input<'a>,
    pub r_n: Input<'a>,
    pub q: Option<&'a str>,
    pub q_n: Option<&'a str>,
}

/// To ignore not gates
pub fn canonicalize_drlatch_ee<'a>(
    drlatch_ee: &DrlatchEe<'a>,
    nots_by_output: &HashMap<&'a str, &'a str>,
) -> CanonicalDrlatchEe<'a> {
    CanonicalDrlatchEe {
        name: drlatch_ee.name,
        ena: canonicalize_input(drlatch_ee.ena, nots_by_output),
        d: canonicalize_input(drlatch_ee.d, nots_by_output),
        r_n: canonicalize_input(drlatch_ee.r_n, nots_by_output),
        q: drlatch_ee.q,
        q_n: drlatch_ee.q_n,
    }
}

impl CanonicalDrlatchEe<'_> {
    pub fn generate_code(&self) -> String {
        let name = self.name;

        if self.q.is_none() && self.q_n.is_none() {
            return String::new();
        }

        let d = self.d.generate_code();
        let ena = self.ena.generate_code();
        let r_n = self.r_n.generate_code();

        let mut output = format!("let {name}_output = self.{name}.update({d}, {ena}, {r_n});\n");

        if let Some(q) = self.q {
            output += &format!("let {q} = {name}_output;\n");
        }

        if let Some(q_n) = self.q_n {
            output += &format!("let {q_n} = !{name}_output;\n");
        }

        output
    }

    pub fn generate_declaration(&self) -> String {
        format!("{}: DffrCc,", self.name)
    }
}
