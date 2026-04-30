use arrayvec::ArrayVec;
use indexmap::IndexSet;
use petgraph::{algo::toposort, graph::DiGraph};
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
    dffr::{CanonicalDffr, CanonicalDffrType, Dffr, canonicalize_dffr, parse_dffr},
    dffr_cc::{CanonicalDffrCc, DffrCc, canonicalize_dffr_cc, parse_dffr_cc},
    gate::{CanonicalGate, Gate, canonicalize_gate, parse_gate},
    nor_latch::{CanonicalNorLatch, NorLatch, canonicalize_nor_latch, parse_nor_latch},
    not::{Not, parse_not},
};

mod and;
mod dffr;
mod dffr_cc;
mod gate;
mod nand;
mod nor;
mod nor_latch;
mod not;
mod or;

// pafu = !wy_match
// vena_n = !hclk
// avet = ppu_4mhz
// nyva = bg/win cycle counter (3 bits) third bit
// laxu = bg/win cycle counter (3 bits) first bit
// wx_clk pafu,vena_n,sprite_x_match,vbl,mode3,lcd_x0,lcd_x1,lcd_x2,lcd_x3,lcd_x4,lcd_x5,lcd_x6,avet,nyva,laxu

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

    let notable_ports: Vec<&str> = args
        .get(3)
        .map(|arg| arg.split(',').collect())
        .unwrap_or_default();

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
            Instance::Nand(nand) => Some(CanonicalInstance::Nand(canonicalize_gate(
                nand,
                &nots_by_output,
            ))),
            Instance::And(and) => Some(CanonicalInstance::And(canonicalize_gate(
                and,
                &nots_by_output,
            ))),
            Instance::Or(or) => Some(CanonicalInstance::Or(canonicalize_gate(
                or,
                &nots_by_output,
            ))),
            Instance::Nor(nor) => Some(CanonicalInstance::Nor(canonicalize_gate(
                nor,
                &nots_by_output,
            ))),
            Instance::DffrCc(dffr_cc) => Some(CanonicalInstance::DffrCc(canonicalize_dffr_cc(
                dffr_cc,
                &nots_by_output,
            ))),
        })
        .collect();

    let canonical_instances_by_output: HashMap<_, _> = canonical_instances
        .iter()
        .flat_map(|instance| {
            instance
                .get_outputs()
                .into_iter()
                .map(move |output| (output, instance))
        })
        .collect();

    let input = canonicalize_input(input, &nots_by_output);

    let mut already_seen = IndexSet::new();

    dfs(
        input.name,
        &canonical_instances_by_output,
        &mut already_seen,
        &notable_ports,
    );

    let mut digraph = DiGraph::<&CanonicalInstance, ()>::new();

    let mut indexes = HashMap::new();

    for node in &already_seen {
        indexes.insert(node.0.get_name(), digraph.add_node(node.0));
    }

    for node in &already_seen {
        for input in node.0.get_inputs() {
            if let Some(input) = canonical_instances_by_output.get(input)
                && let Some(index_input) = indexes.get(input.get_name())
            {
                digraph.add_edge(*index_input, indexes[node.0.get_name()], ());
            }
        }
    }

    println!("edges: {}", digraph.edge_count());

    let indexes = match toposort(&digraph, None) {
        Ok(lol) => lol,
        Err(err) => {
            println!("cycle: {:?}", digraph.node_weight(err.node_id()).unwrap());
            return;
        }
    };

    for declaration in indexes
        .iter()
        .filter_map(|index| digraph.node_weight(*index).unwrap().generate_declaration())
    {
        println!("{declaration}")
    }

    for index in indexes {
        println!("{}", digraph.node_weight(index).unwrap().generate_code())
    }
}

fn dfs<'a>(
    current: &'a str,
    canonical_instances_by_output: &HashMap<&'a str, &'a CanonicalInstance>,
    already_seen: &mut IndexSet<RefEquality<CanonicalInstance<'a>>>,
    notable_ports: &[&str],
) {
    let Some(instance) = canonical_instances_by_output.get(current) else {
        return;
    };

    if already_seen.insert(RefEquality(*instance)) {
        for input in instance.get_inputs() {
            if notable_ports.contains(&input) {
                continue;
            }
            dfs(
                input,
                canonical_instances_by_output,
                already_seen,
                notable_ports,
            );
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
        } else if name == "dmg_dffr_cc" {
            Some(Instance::DffrCc(parse_dffr_cc(syntax_tree, instance)))
        } else if name == "dmg_nor_latch" {
            Some(Instance::NorLatch(parse_nor_latch(syntax_tree, instance)))
        } else if name.starts_with("dmg_not_x") {
            Some(Instance::Not(parse_not(syntax_tree, instance)?))
        } else if name == "dmg_nand_latch" {
            None
        } else if name.starts_with("dmg_nand") {
            Some(Instance::Nand(parse_gate(syntax_tree, instance)?))
        } else if name.starts_with("dmg_and") {
            Some(Instance::And(parse_gate(syntax_tree, instance)?))
        } else if name.starts_with("dmg_or") {
            Some(Instance::Or(parse_gate(syntax_tree, instance)?))
        } else if name.starts_with("dmg_nor") {
            Some(Instance::Nor(parse_gate(syntax_tree, instance)?))
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
    DffrCc(DffrCc<'a>),
    Not(Not<'a>),
    NorLatch(NorLatch<'a>),
    Nand(Gate<'a>),
    And(Gate<'a>),
    Or(Gate<'a>),
    Nor(Gate<'a>),
}

#[derive(Debug)]
enum CanonicalInstance<'a> {
    Dffr(CanonicalDffr<'a>),
    DffrCc(CanonicalDffrCc<'a>),
    NorLatch(CanonicalNorLatch<'a>),
    Nand(CanonicalGate<'a>),
    And(CanonicalGate<'a>),
    Or(CanonicalGate<'a>),
    Nor(CanonicalGate<'a>),
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

impl<'a, T> Clone for RefEquality<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T> Copy for RefEquality<'a, T> {}

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

type Inputs<'a> = ArrayVec<&'a str, 8>;
type Outputs<'a> = ArrayVec<&'a str, 2>;

impl<'a> CanonicalInstance<'a> {
    fn get_inputs(&self) -> Inputs<'_> {
        match self {
            CanonicalInstance::Dffr(canonical_dffr) => {
                let mut inputs: Inputs = [canonical_dffr.clk.name, canonical_dffr.r_n.name]
                    .into_iter()
                    .collect();

                if let CanonicalDffrType::Normal { d, .. } = &canonical_dffr.inner_type {
                    inputs.push(d.name);
                }

                inputs
            }
            CanonicalInstance::DffrCc(canonical_dffr) => [
                canonical_dffr.clk.name,
                canonical_dffr.clk_n.name,
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
            CanonicalInstance::Nand(gate)
            | CanonicalInstance::And(gate)
            | CanonicalInstance::Or(gate)
            | CanonicalInstance::Nor(gate) => gate.inputs.iter().map(|input| input.name).collect(),
        }
    }

    fn get_outputs(&self) -> Outputs<'_> {
        match self {
            CanonicalInstance::Dffr(canonical_dffr) => canonical_dffr
                .q
                .into_iter()
                .chain(match &canonical_dffr.inner_type {
                    CanonicalDffrType::Normal { q_n, .. } => *q_n,
                    CanonicalDffrType::Toggle { q_n } => Some(*q_n),
                })
                .collect(),
            CanonicalInstance::DffrCc(dffr_cc) => {
                dffr_cc.q.into_iter().chain(dffr_cc.q_n).collect()
            }
            CanonicalInstance::NorLatch(canonical_nor_latch) => canonical_nor_latch
                .q
                .into_iter()
                .chain(canonical_nor_latch.q_n)
                .collect(),
            CanonicalInstance::Nand(gate)
            | CanonicalInstance::And(gate)
            | CanonicalInstance::Or(gate)
            | CanonicalInstance::Nor(gate) => core::iter::once(gate.y).collect(),
        }
    }

    fn get_name(&self) -> &'a str {
        match self {
            CanonicalInstance::Dffr(canonical_dffr) => canonical_dffr.name,
            CanonicalInstance::DffrCc(dffr_cc) => dffr_cc.name,
            CanonicalInstance::NorLatch(canonical_nor_latch) => canonical_nor_latch.name,
            CanonicalInstance::Nand(gate)
            | CanonicalInstance::And(gate)
            | CanonicalInstance::Or(gate)
            | CanonicalInstance::Nor(gate) => gate.name,
        }
    }

    fn generate_code(&self) -> String {
        match self {
            CanonicalInstance::Dffr(canonical_dffr) => canonical_dffr.generate_code(),
            CanonicalInstance::NorLatch(canonical_nor_latch) => canonical_nor_latch.generate_code(),
            CanonicalInstance::Nand(gate) => nand::generate_code(gate),
            CanonicalInstance::And(gate) => and::generate_code(gate),
            CanonicalInstance::Or(gate) => or::generate_code(gate),
            CanonicalInstance::DffrCc(dffr_cc) => dffr_cc.generate_code(),
            CanonicalInstance::Nor(gate) => nor::generate_code(gate),
        }
    }

    fn generate_declaration(&self) -> Option<String> {
        match self {
            CanonicalInstance::Dffr(canonical_dffr) => Some(canonical_dffr.generate_declaration()),
            CanonicalInstance::DffrCc(dffr_cc) => Some(dffr_cc.generate_declaration()),
            CanonicalInstance::NorLatch(canonical_nor_latch) => {
                Some(canonical_nor_latch.generate_declaration())
            }
            CanonicalInstance::Nand(_)
            | CanonicalInstance::And(_)
            | CanonicalInstance::Or(_)
            | CanonicalInstance::Nor(_) => None,
        }
    }
}
