use std::collections::HashMap;

use sv_parser::{HierarchicalInstance, SyntaxTree};

use crate::{Input, canonicalize_input, extract_id_and_ports, get_ports};

pub fn parse_dffr_cc<'a>(
    syntax_tree: &'a SyntaxTree,
    instance: &'a HierarchicalInstance,
) -> DffrCc<'a> {
    let (id, named) = extract_id_and_ports(syntax_tree, instance);

    let mut d = None;
    let mut clk = None;
    let mut clk_n = None;
    let mut r_n = None;
    let mut q = None;
    let mut q_n = None;

    for (id, expression) in get_ports(syntax_tree, named) {
        match id {
            "d" => d = expression,
            "clk" => clk = expression,
            "clk_n" => clk_n = expression,
            "r_n" => r_n = expression,
            "q" => q = expression,
            "q_n" => q_n = expression,
            _ => panic!(),
        }
    }

    DffrCc {
        d: d.unwrap(),
        name: id,
        clk: clk.unwrap(),
        clk_n: clk_n.unwrap(),
        q,
        q_n,
        r_n: r_n.unwrap(),
    }
}

#[derive(Debug)]
pub struct DffrCc<'a> {
    pub name: &'a str,
    pub d: &'a str,
    pub clk: &'a str,
    pub clk_n: &'a str,
    pub r_n: &'a str,
    pub q: Option<&'a str>,
    pub q_n: Option<&'a str>,
}

#[derive(Debug)]
pub struct CanonicalDffrCc<'a> {
    pub name: &'a str,
    pub d: Input<'a>,
    pub clk: Input<'a>,
    pub clk_n: Input<'a>,
    pub r_n: Input<'a>,
    pub q: Option<&'a str>,
    pub q_n: Option<&'a str>,
}

/// To ignore not gates
pub fn canonicalize_dffr_cc<'a>(
    dffr_cc: &DffrCc<'a>,
    nots_by_output: &HashMap<&'a str, &'a str>,
) -> CanonicalDffrCc<'a> {
    CanonicalDffrCc {
        name: dffr_cc.name,
        clk: canonicalize_input(dffr_cc.clk, nots_by_output),
        clk_n: canonicalize_input(dffr_cc.clk_n, nots_by_output),
        d: canonicalize_input(dffr_cc.d, nots_by_output),
        r_n: canonicalize_input(dffr_cc.r_n, nots_by_output),
        q: dffr_cc.q,
        q_n: dffr_cc.q_n,
    }
}

impl CanonicalDffrCc<'_> {
    pub fn generate_code(&self) -> String {
        let name = self.name;

        if self.q.is_none() && self.q_n.is_none() {
            return String::new();
        }

        let d = self.d.generate_code();
        let clk = self.clk.generate_code();
        let clk_n = self.clk_n.generate_code();
        let r_n = self.r_n.generate_code();

        let mut output =
            format!("let {name}_output = self.{name}.update({d}, {clk}, {clk_n}, {r_n});\n");

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
