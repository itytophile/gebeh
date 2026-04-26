use arrayvec::ArrayVec;
use indexmap::IndexSet;
use std::collections::HashMap;
use std::env;
use std::hash::Hash;
use std::path::PathBuf;
use sv_parser::{
    Description, Expression, HierarchicalInstance, Identifier, InstanceIdentifier,
    ListOfPortConnections, ListOfPortConnectionsNamed, Locate, ModuleDeclaration,
    ModuleOrGenerateItem, NamedPortConnection, NonPortModuleItem, Primary, RefNode, SyntaxTree,
    parse_sv,
};

use crate::{
    and::{And, CanonicalAnd, canonicalize_and, parse_and},
    dffr::{CanonicalDffr, Dffr, canonicalize_dffr, parse_dffr},
    nand::{CanonicalNand, Nand, canonicalize_nand, parse_nand},
    nor_latch::{CanonicalNorLatch, NorLatch, canonicalize_nor_latch, parse_nor_latch},
    not::{Not, parse_not},
};

mod and;
mod dffr;
mod nand;
mod nor_latch;
mod not;

fn main() {
    let args: Vec<String> = env::args().collect();

    // The path of SystemVerilog source file
    let path = PathBuf::from(&args[1]);
    // The list of defined macros
    let defines = HashMap::new();
    // The list of include paths
    let includes: Vec<PathBuf> = Vec::new();

    // Parse
    let result = parse_sv(&path, &defines, &includes, false, false);

    let (syntax_tree, _) = result.unwrap();

    let instances: Vec<_> = get_instances(&syntax_tree).collect();

    // the end of what's interesting (yeah it's called input it's strange)
    let input = &args[2];

    let nots_by_output: HashMap<_, _> = instances
        .iter()
        .filter_map(|instance| {
            if let Instance::Not(not) = instance {
                Some(not)
            } else {
                None
            }
        })
        .map(|not| (not.y, not.input))
        .collect();

    let canonical_instances: Vec<_> = instances
        .iter()
        .filter_map(|instance| match instance {
            Instance::Dffr(dffr) => Some(CanonicalInstance::Dffr(canonicalize_dffr(
                dffr,
                &nots_by_output,
            ))),
            Instance::Not(_) => None,
            Instance::NorLatch(nor_latch) => Some(CanonicalInstance::NorLatch(
                canonicalize_nor_latch(nor_latch, &nots_by_output),
            )),
            Instance::Nand(nand) => Some(CanonicalInstance::Nand(canonicalize_nand(
                nand,
                &nots_by_output,
            ))),
            Instance::And(and) => Some(CanonicalInstance::And(canonicalize_and(
                and,
                &nots_by_output,
            ))),
        })
        .collect();

    let canonical_instances_by_output: HashMap<_, _> = canonical_instances
        .iter()
        .flat_map(|instance| match instance {
            CanonicalInstance::Dffr(canonical_dffr) => canonical_dffr
                .q
                .into_iter()
                .chain(canonical_dffr.q_n)
                .map(|output| (output, instance))
                .collect::<ArrayVec<_, 2>>(),
            CanonicalInstance::NorLatch(canonical_nor_latch) => canonical_nor_latch
                .q
                .into_iter()
                .chain(canonical_nor_latch.q_n)
                .map(|output| (output, instance))
                .collect(),
            CanonicalInstance::Nand(canonical_nand) => {
                core::iter::once((canonical_nand.y, instance)).collect()
            }
            CanonicalInstance::And(canonical_and) => {
                core::iter::once((canonical_and.y, instance)).collect()
            }
        })
        .collect();

    let input = canonicalize_input(input, &nots_by_output);

    let mut already_seen = IndexSet::new();

    dfs(
        input.name,
        &canonical_instances_by_output,
        &mut already_seen,
    );

    for declaration in already_seen
        .iter()
        .rev()
        .filter_map(|instance| instance.0.generate_declaration())
    {
        println!("{declaration}")
    }

    for instance in already_seen.iter().rev() {
        println!("{}", instance.0.generate_code())
    }
}

fn dfs<'a>(
    current: &'a str,
    canonical_instances_by_output: &HashMap<&'a str, &'a CanonicalInstance>,
    already_seen: &mut IndexSet<RefEquality<CanonicalInstance<'a>>>,
) {
    let Some(instance) = canonical_instances_by_output.get(current) else {
        return;
    };

    if already_seen.insert(RefEquality(*instance)) {
        for input in instance.get_inputs() {
            dfs(input, canonical_instances_by_output, already_seen);
        }
    }
}

fn canonicalize_input<'a>(input: &'a str, nots_by_output: &HashMap<&'a str, &'a str>) -> Input<'a> {
    let mut input = Input {
        name: input,
        is_inverted: false,
    };

    while let Some(inverted) = nots_by_output.get(input.name) {
        input = Input {
            name: inverted,
            is_inverted: !input.is_inverted,
        };
    }

    input
}

fn get_instances<'a>(syntax_tree: &'a SyntaxTree) -> impl Iterator<Item = Instance<'a>> {
    let a = syntax_tree.into_iter().next().unwrap();
    let RefNode::SourceText(a) = a else { panic!() };
    let Description::ModuleDeclaration(a) = &a.nodes.2[0] else {
        panic!()
    };
    let ModuleDeclaration::Ansi(a) = a.as_ref() else {
        panic!()
    };
    a.nodes.2.iter().filter_map(move |a| {
        let NonPortModuleItem::ModuleOrGenerateItem(a) = a else {
            panic!()
        };
        let ModuleOrGenerateItem::Module(a) = a.as_ref() else {
            return None;
        };
        let (module_id, _, instances, _) = &a.nodes.1.nodes;
        let name = get_name_from_identifier(syntax_tree, &module_id.nodes.0);
        let instance = &instances.nodes.0;

        if name == "dmg_dffr" {
            Some(Instance::Dffr(parse_dffr(syntax_tree, instance)))
        } else if name == "dmg_nor_latch" {
            Some(Instance::NorLatch(parse_nor_latch(syntax_tree, instance)))
        } else if name.starts_with("dmg_not_x") {
            Some(Instance::Not(parse_not(syntax_tree, instance)?))
        } else if name == "dmg_nand_latch" {
            None
        } else if name.starts_with("dmg_nand") {
            Some(Instance::Nand(parse_nand(syntax_tree, instance)?))
        } else if name.starts_with("dmg_and") {
            Some(Instance::And(parse_and(syntax_tree, instance)?))
        } else {
            None
        }
    })
}

fn get_ports<'a>(
    syntax_tree: &'a SyntaxTree,
    ports: &'a ListOfPortConnectionsNamed,
) -> impl Iterator<Item = (&'a str, Option<&'a str>)> {
    core::iter::once(&ports.nodes.0.nodes.0)
        .chain(ports.nodes.0.nodes.1.iter().map(|(_, port)| port))
        .map(|a| {
            let NamedPortConnection::Identifier(a) = a else {
                panic!()
            };
            let (_, _, id, expression) = &a.nodes;
            let expression = expression.as_ref().unwrap().nodes.1.as_ref().map(|expr| {
                let Expression::Primary(expr) = expr else {
                    panic!()
                };
                let Primary::Hierarchical(expr) = expr.as_ref() else {
                    panic!()
                };
                let (_, id, _) = &expr.nodes;
                let (_, _, id) = &id.nodes;
                get_name_from_identifier(syntax_tree, id)
            });
            let locate = get_locate_from_identifier(&id.nodes.0);
            (syntax_tree.get_str(locate).unwrap(), expression)
        })
}

fn extract_id_and_ports<'a>(
    syntax_tree: &'a SyntaxTree,
    instance: &'a HierarchicalInstance,
) -> (&'a str, &'a sv_parser::ListOfPortConnectionsNamed) {
    let (name, connections) = &instance.nodes;
    let (InstanceIdentifier { nodes: (id,) }, _) = &name.nodes;
    let locate = get_locate_from_identifier(id);
    let id = syntax_tree.get_str(locate).unwrap();
    let ListOfPortConnections::Named(named) = connections.nodes.1.as_ref().unwrap() else {
        panic!();
    };
    (id, named)
}

#[derive(Debug)]
enum Instance<'a> {
    Dffr(Dffr<'a>),
    Not(Not<'a>),
    NorLatch(NorLatch<'a>),
    Nand(Nand<'a>),
    And(And<'a>),
}

#[derive(Debug)]
enum CanonicalInstance<'a> {
    Dffr(CanonicalDffr<'a>),
    NorLatch(CanonicalNorLatch<'a>),
    Nand(CanonicalNand<'a>),
    And(CanonicalAnd<'a>),
}

fn get_locate_from_identifier(id: &Identifier) -> &Locate {
    &match id {
        Identifier::SimpleIdentifier(simple_identifier) => &simple_identifier.nodes,
        Identifier::EscapedIdentifier(escaped_identifier) => &escaped_identifier.nodes,
    }
    .0
}

fn get_name_from_identifier<'a>(syntax_tree: &'a SyntaxTree, id: &Identifier) -> &'a str {
    syntax_tree.get_str(get_locate_from_identifier(id)).unwrap()
}

#[derive(Debug)]
struct Input<'a> {
    name: &'a str,
    is_inverted: bool,
}

impl Input<'_> {
    pub fn generate_code(&self) -> String {
        format!("{}{}", if self.is_inverted { "!" } else { "" }, self.name)
    }
}

struct RefEquality<'a, T>(&'a T);

impl<'a, T> Hash for RefEquality<'a, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let ptr: *const T = self.0;
        ptr.hash(state);
    }
}

impl<'a, T> PartialEq for RefEquality<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        let this: *const T = self.0;
        let other: *const T = other.0;
        this == other
    }
}

impl<'a, T> Eq for RefEquality<'a, T> {}

impl<'a> CanonicalInstance<'a> {
    fn get_inputs(&self) -> ArrayVec<&'a str, 7> {
        match self {
            CanonicalInstance::Dffr(canonical_dffr) => [
                canonical_dffr.clk.name,
                canonical_dffr.d.name,
                canonical_dffr.r_n.name,
            ]
            .into_iter()
            .collect(),
            CanonicalInstance::NorLatch(canonical_nor_latch) => {
                [canonical_nor_latch.r.name, canonical_nor_latch.s.name]
                    .into_iter()
                    .collect()
            }
            CanonicalInstance::Nand(canonical_nand) => canonical_nand
                .inputs
                .iter()
                .map(|input| input.name)
                .collect(),
            CanonicalInstance::And(canonical_and) => canonical_and
                .inputs
                .iter()
                .map(|input| input.name)
                .collect(),
        }
    }

    fn get_name(&self) -> &'a str {
        match self {
            CanonicalInstance::Dffr(canonical_dffr) => canonical_dffr.name,
            CanonicalInstance::NorLatch(canonical_nor_latch) => canonical_nor_latch.name,
            CanonicalInstance::Nand(canonical_nand) => canonical_nand.name,
            CanonicalInstance::And(canonical_and) => canonical_and.name,
        }
    }

    fn generate_code(&self) -> String {
        match self {
            CanonicalInstance::Dffr(canonical_dffr) => canonical_dffr.generate_code(),
            CanonicalInstance::NorLatch(canonical_nor_latch) => canonical_nor_latch.generate_code(),
            CanonicalInstance::Nand(canonical_nand) => canonical_nand.generate_code(),
            CanonicalInstance::And(canonical_and) => canonical_and.generate_code(),
        }
    }

    fn generate_declaration(&self) -> Option<String> {
        match self {
            CanonicalInstance::Dffr(canonical_dffr) => Some(canonical_dffr.generate_declaration()),
            CanonicalInstance::NorLatch(canonical_nor_latch) => {
                Some(canonical_nor_latch.generate_declaration())
            }
            CanonicalInstance::Nand(_) => None,
            CanonicalInstance::And(_) => None,
        }
    }
}
