//! Extract the supported Serde wire schema using the Rust syntax tree.
//! Unsupported serialization features fail closed, rather than disappearing.
use quote::ToTokens;
use serde_json::{json, Map, Value};
use std::{env, fs, path::Path};
use syn::{
    parse::Parse, punctuated::Punctuated, visit::Visit, Attribute, Fields, GenericArgument, Item,
    PathArguments, Token, Type,
};

fn fail(message: impl std::fmt::Display) -> ! {
    panic!("contract schema: {message}")
}

fn attrs(attributes: &[Attribute]) -> Map<String, Value> {
    let mut result = Map::new();
    for attribute in attributes.iter().filter(|a| a.path().is_ident("serde")) {
        attribute
            .parse_nested_meta(|meta| {
                let name = meta
                    .path
                    .get_ident()
                    .unwrap_or_else(|| fail("qualified Serde attribute"))
                    .to_string();
                let value = if meta.input.peek(Token![=]) {
                    json!(meta.value()?.parse::<syn::LitStr>()?.value())
                } else if meta.input.peek(syn::token::Paren) {
                    fail(format!("unsupported nested Serde attribute {name}"))
                } else {
                    json!(true)
                };
                if ![
                    "rename",
                    "rename_all",
                    "default",
                    "deny_unknown_fields",
                    "skip_serializing_if",
                ]
                .contains(&name.as_str())
                {
                    fail(format!("unsupported Serde attribute {name}"));
                }
                result.insert(name, value);
                Ok(())
            })
            .unwrap_or_else(|e| fail(e));
    }
    result
}

fn ty(value: &Type) -> Value {
    match value {
        Type::Path(value) => {
            let segment = value.path.segments.last().unwrap();
            let name = segment.ident.to_string();
            let arguments: Vec<Value> = match &segment.arguments {
                PathArguments::None => vec![],
                PathArguments::AngleBracketed(args) => args
                    .args
                    .iter()
                    .map(|a| match a {
                        GenericArgument::Type(t) => ty(t),
                        _ => fail("unsupported generic argument"),
                    })
                    .collect(),
                _ => fail("unsupported path arguments"),
            };
            json!({"name":name,"args":arguments})
        }
        Type::Tuple(t) if t.elems.is_empty() => json!({"name":"void","args":[]}),
        _ => fail(format!("unsupported wire type {}", value.to_token_stream())),
    }
}

fn fields(value: &Fields) -> Value {
    let named = match value {
        Fields::Named(named) => named,
        _ => fail("non-object wire struct"),
    };
    Value::Array(named.named.iter().map(|f| json!({
        "name": f.ident.as_ref().unwrap().to_string(), "type":ty(&f.ty), "serde":attrs(&f.attrs),
    })).collect())
}

struct Record {
    row: syn::Ident,
    input: syn::Ident,
    fields: Fields,
}
impl Parse for Record {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let row = input.parse()?;
        input.parse::<Token![,]>()?;
        let name = input.parse()?;
        input.parse::<Token![,]>()?;
        let content;
        let brace_token = syn::braced!(content in input);
        let named = content.parse_terminated(syn::Field::parse_named, Token![,])?;
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        Ok(Self {
            row,
            input: name,
            fields: Fields::Named(syn::FieldsNamed { brace_token, named }),
        })
    }
}

fn serializable(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .filter(|a| a.path().is_ident("derive"))
        .any(|a| {
            a.parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
                .unwrap()
                .iter()
                .any(|p| p.is_ident("Serialize"))
        })
}

#[derive(Default)]
struct Handlers {
    registered: Vec<String>,
}
impl<'ast> Visit<'ast> for Handlers {
    fn visit_macro(&mut self, value: &'ast syn::Macro) {
        if value.path.segments.last().unwrap().ident == "generate_handler" {
            let names = value
                .parse_body_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
                .unwrap();
            self.registered.extend(
                names
                    .iter()
                    .map(|p| p.segments.last().unwrap().ident.to_string()),
            );
        }
        syn::visit::visit_macro(self, value);
    }
}

fn main() {
    let root = env::args()
        .nth(1)
        .unwrap_or_else(|| fail("repository path required"));
    let root = Path::new(&root);
    let mut types = Map::new();
    let mut macros = Map::new();
    let mut paths: Vec<_> = fs::read_dir(root.join("crates/domain/src"))
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .collect();
    paths.sort();
    for path in paths {
        let file = syn::parse_file(&fs::read_to_string(&path).unwrap()).unwrap_or_else(|e| fail(e));
        for item in file.items {
            match item {
                Item::Struct(s)
                    if matches!(s.vis, syn::Visibility::Public(_)) && serializable(&s.attrs) =>
                {
                    types.insert(
                        s.ident.to_string(),
                        json!({"fields":fields(&s.fields), "serde":attrs(&s.attrs)}),
                    );
                }
                Item::Enum(e)
                    if matches!(e.vis, syn::Visibility::Public(_)) && serializable(&e.attrs) =>
                {
                    let variants: Vec<_> = e
                        .variants
                        .iter()
                        .map(|v| {
                            if !matches!(v.fields, Fields::Unit) {
                                fail("unsupported data-carrying wire enum");
                            }
                            json!({"name":v.ident.to_string(),"serde":attrs(&v.attrs)})
                        })
                        .collect();
                    types.insert(
                        e.ident.to_string(),
                        json!({"variants":variants,"serde":attrs(&e.attrs)}),
                    );
                }
                Item::Macro(m)
                    if m.ident
                        .as_ref()
                        .is_some_and(|n| n == "record" || n.to_string().ends_with("_record")) =>
                {
                    // Verify these application macros against a reviewed token fingerprint in Python.
                    macros.insert(
                        m.ident.unwrap().to_string(),
                        json!(m.mac.tokens.to_string()),
                    );
                }
                Item::Macro(m)
                    if m.mac
                        .path
                        .get_ident()
                        .is_some_and(|n| n == "record" || n.to_string().ends_with("_record")) =>
                {
                    let record: Record = syn::parse2(m.mac.tokens).unwrap_or_else(|e| fail(e));
                    let extra = fields(&record.fields).as_array().unwrap().clone();
                    for (name, input) in [(record.row, false), (record.input, true)] {
                        let mut fs = vec![
                            json!({"name":"id", "type":if input {json!({"name":"Option","args":[{"name":"String","args":[]}]})} else {json!({"name":"String","args":[]})}, "serde":{}}),
                        ];
                        if input {
                            fs.push(json!({"name":"expected_revision_id","type":{"name":"Option","args":[{"name":"String","args":[]}]},"serde":{}}));
                            fs.push(json!({"name":"reason","type":{"name":"String","args":[]},"serde":{}}));
                        } else {
                            fs.push(json!({"name":"revision_id","type":{"name":"String","args":[]},"serde":{}}));
                        }
                        fs.extend(extra.clone());
                        types.insert(name.to_string(), json!({"fields":fs,"serde":{}}));
                    }
                }
                _ => (),
            }
        }
    }
    let main =
        syn::parse_file(&fs::read_to_string(root.join("src-tauri/src/main.rs")).unwrap()).unwrap();
    let mut handlers = Handlers::default();
    handlers.visit_file(&main);
    let mut commands = Map::new();
    for item in main.items {
        if let Item::Fn(function) = item {
            if !function
                .attrs
                .iter()
                .any(|a| a.path().segments.last().unwrap().ident == "command")
            {
                continue;
            }
            let command_attribute = function
                .attrs
                .iter()
                .find(|a| a.path().segments.last().unwrap().ident == "command")
                .unwrap();
            if !matches!(command_attribute.meta, syn::Meta::Path(_)) {
                fail("unsupported tauri::command options");
            }
            let mut args = Map::new();
            for argument in function.sig.inputs {
                let argument = match argument {
                    syn::FnArg::Typed(argument) => argument,
                    _ => fail("command self argument"),
                };
                if let Type::Path(path) = &*argument.ty {
                    if ["State", "AppHandle"]
                        .iter()
                        .any(|name| path.path.segments.last().unwrap().ident == name)
                    {
                        continue;
                    }
                }
                let shape = ty(&argument.ty);
                let name = match *argument.pat {
                    syn::Pat::Ident(id) => id.ident.to_string(),
                    _ => fail("command argument pattern"),
                };
                args.insert(name, shape);
            }
            let output = match function.sig.output {
                syn::ReturnType::Type(_, t) => ty(&t),
                _ => fail("missing command return"),
            };
            if output["name"] != "Result" || output["args"][1]["name"] != "AppError" {
                fail("command must return Result<T, AppError>");
            }
            commands.insert(
                function.sig.ident.to_string(),
                json!({"args":args,"return":output["args"][0]}),
            );
        }
    }
    println!(
        "{}",
        json!({"types":types,"macros":macros,"commands":commands,"registered":handlers.registered})
    );
}
