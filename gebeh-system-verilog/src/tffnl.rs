use std::collections::HashMap;

use sv_parser::{HierarchicalInstance, SyntaxTree};

use crate::{Input, canonicalize_input, extract_id_and_ports, get_ports};

pub fn parse_tffnl<'a>(
    syntax_tree: &'a SyntaxTree,
    instance: &'a HierarchicalInstance,
) -> Tffnl<'a> {
    let (id, named) = extract_id_and_ports(syntax_tree, instance);

    let mut d = None;
    let mut l = None;
    let mut tclk_n = None;
    let mut q = None;
    let mut q_n = None;

    for (id, expression) in get_ports(syntax_tree, named) {
        match id {
            "d" => d = expression,
            "l" => l = expression,
            "tclk_n" => tclk_n = expression,
            "q" => q = expression,
            "q_n" => q_n = expression,
            _ => panic!(),
        }
    }

    Tffnl {
        d: d.unwrap(),
        name: id,
        l: l.unwrap(),
        tclk_n: tclk_n.unwrap(),
        q,
        q_n,
    }
}

#[derive(Debug)]
pub struct Tffnl<'a> {
    pub name: &'a str,
    pub d: &'a str,
    pub l: &'a str,
    pub tclk_n: &'a str,
    pub q: Option<&'a str>,
    pub q_n: Option<&'a str>,
}

#[derive(Debug)]
pub struct CanonicalTffnl<'a> {
    pub name: &'a str,
    pub d: Input<'a>,
    pub l: Input<'a>,
    pub tclk_n: Input<'a>,
    pub q: Option<&'a str>,
    pub q_n: Option<&'a str>,
}

/// To ignore not gates
pub fn canonicalize_tffnl<'a>(
    tffnl: &Tffnl<'a>,
    nots_by_output: &HashMap<&'a str, &'a str>,
) -> CanonicalTffnl<'a> {
    CanonicalTffnl {
        name: tffnl.name,
        d: canonicalize_input(tffnl.d, nots_by_output),
        l: canonicalize_input(tffnl.l, nots_by_output),
        tclk_n: canonicalize_input(tffnl.tclk_n, nots_by_output),
        q: tffnl.q,
        q_n: tffnl.q_n,
    }
}

impl CanonicalTffnl<'_> {
    pub fn generate_code(&self) -> String {
        let name = self.name;

        if self.q.is_none() && self.q_n.is_none() {
            return String::new();
        }

        let d = self.d.generate_code();
        let l = self.l.generate_code();
        let tclk_n = self.tclk_n.generate_code();

        let mut output = format!("let {name}_output = self.{name}.update({d}, {l}, {tclk_n});\n");

        if let Some(q) = self.q {
            output += &format!("let {q} = {name}_output;\n");
        }

        if let Some(q_n) = self.q_n {
            output += &format!("let {q_n} = !{name}_output;\n");
        }

        output
    }

    pub fn generate_declaration(&self) -> String {
        format!("{}: Tffnl,", self.name)
    }
}
