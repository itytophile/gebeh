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

    let d = d.unwrap();

    Dffr {
        name: id,
        clk: clk.unwrap(),
        q,
        r_n: r_n.unwrap(),
        inner_type: if let Some(q_n) = q_n
            && q_n == d
        {
            DffrType::Toggle { q_n }
        } else {
            DffrType::Normal { d, q_n }
        },
    }
}

#[derive(Debug)]
pub enum DffrType<'a> {
    Normal { d: &'a str, q_n: Option<&'a str> },
    Toggle { q_n: &'a str },
}

#[derive(Debug)]
pub enum CanonicalDffrType<'a> {
    Normal { d: Input<'a>, q_n: Option<&'a str> },
    Toggle { q_n: &'a str },
}

#[derive(Debug)]
pub struct Dffr<'a> {
    pub name: &'a str,
    pub clk: &'a str,
    pub r_n: &'a str,
    pub q: Option<&'a str>,
    pub inner_type: DffrType<'a>,
}

#[derive(Debug)]
pub struct CanonicalDffr<'a> {
    pub name: &'a str,
    pub clk: Input<'a>,
    pub r_n: Input<'a>,
    pub q: Option<&'a str>,
    pub inner_type: CanonicalDffrType<'a>,
}

/// To ignore not gates
pub fn canonicalize_dffr<'a>(
    dffr: &Dffr<'a>,
    nots_by_output: &HashMap<&'a str, &'a str>,
) -> CanonicalDffr<'a> {
    CanonicalDffr {
        name: dffr.name,
        clk: canonicalize_input(dffr.clk, nots_by_output),
        r_n: canonicalize_input(dffr.r_n, nots_by_output),
        q: dffr.q,
        inner_type: match dffr.inner_type {
            DffrType::Normal { d, q_n } => CanonicalDffrType::Normal {
                d: canonicalize_input(d, nots_by_output),
                q_n,
            },
            DffrType::Toggle { q_n } => CanonicalDffrType::Toggle { q_n },
        },
    }
}

impl CanonicalDffr<'_> {
    pub fn generate_code(&self) -> String {
        let name = self.name;

        match &self.inner_type {
            CanonicalDffrType::Normal { d, q_n } => {
                if self.q.is_none() && q_n.is_none() {
                    return String::new();
                }

                let d = d.generate_code();
                let clk = self.clk.generate_code();
                let r_n = self.r_n.generate_code();

                let mut output =
                    format!("let {name}_output = self.{name}.update({d}, {clk}, {r_n});\n");

                if let Some(q) = self.q {
                    output += &format!("let {q} = {name}_output;\n");
                }

                if let Some(q_n) = q_n {
                    output += &format!("let {q_n} = !{name}_output;\n");
                }

                output
            }
            CanonicalDffrType::Toggle { q_n } => {
                let clk = self.clk.generate_code();
                let r_n = self.r_n.generate_code();

                let mut output = format!("let {name}_output = self.{name}.update({clk}, {r_n});\n");

                if let Some(q) = self.q {
                    output += &format!("let {q} = {name}_output;\n");
                }

                output += &format!("let {q_n} = !{name}_output;\n");

                output
            }
        }
    }

    pub fn generate_declaration(&self) -> String {
        match self.inner_type {
            CanonicalDffrType::Normal { .. } => format!("{}: Dffr,", self.name),
            CanonicalDffrType::Toggle { .. } => format!("{}: DffrToggle,", self.name),
        }
    }
}
