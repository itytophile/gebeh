use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use sv_parser::{
    Description, Expression, HierarchicalInstance, Identifier, InstanceIdentifier,
    ListOfPortConnections, ListOfPortConnectionsNamed, Locate, ModuleDeclaration,
    ModuleOrGenerateItem, NamedPortConnection, NonPortModuleItem, Primary, RefNode, SyntaxTree,
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

    for dffr in get_instances(&syntax_tree) {
        println!("{dffr:?}");
    }
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
        } else if name.starts_with("dmg_not_x") {
            Some(Instance::Not(parse_not(syntax_tree, instance)?))
        } else {
            None
        }
    })
}

fn parse_not<'a>(
    syntax_tree: &'a SyntaxTree,
    instance: &'a HierarchicalInstance,
) -> Option<Not<'a>> {
    let (id, named) = extract_id_and_ports(syntax_tree, instance);

    let mut input = None;
    let mut y = None;

    for (id, expression) in get_port_ids(syntax_tree, named) {
        match id {
            "y" => y = expression,
            "in" => input = expression,
            _ => panic!(),
        }
    }

    let Some(y) = y else {
        eprintln!("No y for {id}");
        return None;
    };

    Some(Not {
        name: id,
        input: input.unwrap(),
        y,
    })
}

fn parse_dffr<'a>(syntax_tree: &'a SyntaxTree, instance: &'a HierarchicalInstance) -> Dffr<'a> {
    let (id, named) = extract_id_and_ports(syntax_tree, instance);

    let mut d = None;
    let mut clk = None;
    let mut r_n = None;
    let mut q = None;
    let mut q_n = None;

    for (id, expression) in get_port_ids(syntax_tree, named) {
        match id {
            "d" => d = expression,
            "clk" => clk = expression,
            "r_n" => r_n = expression,
            "q" => q = expression,
            "q_n" => q_n = expression,
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
}

fn get_port_ids<'a>(
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

#[derive(Debug)]
struct Not<'a> {
    name: &'a str,
    input: &'a str,
    y: &'a str,
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
