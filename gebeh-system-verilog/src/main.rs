use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use sv_parser::{
    EscapedIdentifier, InstanceIdentifier, Locate, RefNode, SimpleIdentifier, SyntaxTree, parse_sv,
    unwrap_node,
};

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

    // &SyntaxTree is iterable
    for node in &syntax_tree {
        if let RefNode::HierarchicalInstance(instance) = node {

            let (name, connections) = &instance.nodes;
            let (InstanceIdentifier { nodes: (id,) }, _) = &name.nodes;
            let (locate, _) = match id {
                sv_parser::Identifier::SimpleIdentifier(simple_identifier) => {
                    &simple_identifier.nodes
                }
                sv_parser::Identifier::EscapedIdentifier(escaped_identifier) => {
                    &escaped_identifier.nodes
                }
            };
            let id = syntax_tree.get_str(locate).unwrap();
            println!("{} {id}", locate.line);
        }
    }
}

fn get_identifier(node: RefNode) -> Option<Locate> {
    // unwrap_node! can take multiple types
    match unwrap_node!(node, SimpleIdentifier, EscapedIdentifier) {
        Some(
            RefNode::SimpleIdentifier(SimpleIdentifier { nodes: (locate, _) })
            | RefNode::EscapedIdentifier(EscapedIdentifier { nodes: (locate, _) }),
        ) => Some(*locate),
        _ => None,
    }
}
