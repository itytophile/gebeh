use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use sv_parser::{
    Expression, Identifier, InstanceIdentifier, ListOfPortConnections, Locate, NamedPortConnection,
    Primary, RefNode, SyntaxTree, parse_sv,
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
        let locate = get_locate_from_identifier(id);
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
                get_name_from_identifier(&syntax_tree, id)
            });
            let locate = get_locate_from_identifier(&id.nodes.0);
            let id = syntax_tree.get_str(locate).unwrap();
            println!("=> {} {id} {expression:?}", locate.line);
        }
    }
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
