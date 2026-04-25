use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use sv_parser::{
    Description, Expression, Identifier, InstanceIdentifier, ListOfPortConnections, Locate,
    ModuleDeclaration, ModuleOrGenerateItem, NamedPortConnection, NonPortModuleItem, Primary,
    RefNode, SyntaxTree, parse_sv,
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

    for dffr in get_instances(&syntax_tree) {
        println!("{dffr:?}");
    }
}

fn get_instances<'a>(syntax_tree: &'a SyntaxTree) -> impl Iterator<Item = Dffr<'a>> {
    let a = syntax_tree.into_iter().next().unwrap();
    let RefNode::SourceText(a) = a else { panic!() };
    let Description::ModuleDeclaration(a) = &a.nodes.2[0] else {
        panic!()
    };
    let ModuleDeclaration::Ansi(a) = a.as_ref() else {
        panic!()
    };
    a.nodes
        .2
        .iter()
        .filter_map(move |a| {
            let NonPortModuleItem::ModuleOrGenerateItem(a) = a else {
                panic!()
            };
            let ModuleOrGenerateItem::Module(a) = a.as_ref() else {
                return None;
            };
            let (module_id, _, instances, _) = &a.nodes.1.nodes;
            let name = get_name_from_identifier(syntax_tree, &module_id.nodes.0);
            if name != "dmg_dffr" {
                return None;
            }
            println!("{name}");
            let instance = &instances.nodes.0;
            let (name, connections) = &instance.nodes;
            let (InstanceIdentifier { nodes: (id,) }, _) = &name.nodes;
            let locate = get_locate_from_identifier(id);
            let id = syntax_tree.get_str(locate).unwrap();
            let ListOfPortConnections::Named(named) = connections.nodes.1.as_ref().unwrap() else {
                panic!();
            };
            Some((id, named))
        })
        .map(move |(id, named)| {
            let mut d = None;
            let mut clk = None;
            let mut r_n = None;
            let mut q = None;
            let mut q_n = None;

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
                    get_name_from_identifier(syntax_tree, id)
                });
                let locate = get_locate_from_identifier(&id.nodes.0);
                let id = syntax_tree.get_str(locate).unwrap();

                match id {
                    "d" => d = *expression,
                    "clk" => clk = *expression,
                    "r_n" => r_n = *expression,
                    "q" => q = *expression,
                    "q_n" => q_n = *expression,
                    _ => panic!(),
                }
            }

            Dffr {
                d: d.unwrap(),
                name: id,
                clk: clk.unwrap(),
                q,
                q_n,
                r_n: r_n.unwrap(),
            }
        })
}

#[derive(Debug)]
struct Dffr<'a> {
    name: &'a str,
    d: &'a str,
    clk: &'a str,
    r_n: &'a str,
    q: Option<&'a str>,
    q_n: Option<&'a str>,
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
