use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use sv_parser::{
    Expression, InstanceIdentifier, ListOfPortConnections, NamedPortConnection, Primary, RefNode,
    parse_sv,
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
        let RefNode::HierarchicalInstance(instance) = node else {
            continue;
        };

        let (name, connections) = &instance.nodes;
        let (InstanceIdentifier { nodes: (id,) }, _) = &name.nodes;
        let (locate, _) = match id {
            sv_parser::Identifier::SimpleIdentifier(simple_identifier) => &simple_identifier.nodes,
            sv_parser::Identifier::EscapedIdentifier(escaped_identifier) => {
                &escaped_identifier.nodes
            }
        };
        let id = syntax_tree.get_str(locate).unwrap();
        println!("{} {id}", locate.line);
        let ListOfPortConnections::Named(named) = connections.nodes.1.as_ref().unwrap() else {
            continue;
        };
        for a in named.nodes.0.contents() {
            let NamedPortConnection::Identifier(a) = a else {
                panic!()
            };
            let (_, _, id, expression) = &a.nodes;
            let expression = &expression.as_ref().unwrap().nodes.1.as_ref().map(|expr| {
                let Expression::Primary(expr) = expr else {
                    panic!()
                };
                let Primary::Hierarchical(expr) = expr.as_ref() else {
                    panic!()
                };
                let (_, id, _) = &expr.nodes;
                let (_, _, id) = &id.nodes;
                let (locate, _) = match id {
                    sv_parser::Identifier::SimpleIdentifier(simple_identifier) => {
                        &simple_identifier.nodes
                    }
                    sv_parser::Identifier::EscapedIdentifier(escaped_identifier) => {
                        &escaped_identifier.nodes
                    }
                };
                syntax_tree.get_str(locate).unwrap()
            });
            let (locate, _) = match &id.nodes.0 {
                sv_parser::Identifier::SimpleIdentifier(simple_identifier) => {
                    &simple_identifier.nodes
                }
                sv_parser::Identifier::EscapedIdentifier(escaped_identifier) => {
                    &escaped_identifier.nodes
                }
            };
            let id = syntax_tree.get_str(locate).unwrap();
            println!("=> {} {id} {expression:?}", locate.line);
        }
    }
}
