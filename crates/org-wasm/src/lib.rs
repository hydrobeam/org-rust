use org_exporter::ConfigOptions;
use org_exporter::Exporter;
use org_exporter::Html;
use org_exporter::Org;
use org_parser::{parse_org, Expr, Node, NodeID, NodePool};
use wasm_bindgen::prelude::*;

use js_sys::Int32Array;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_u32(a: u32);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_vec(a: Vec<i32>);
}

// We want to avoid creating an entirely new string
// on every repearse: re-use the same buffer to save on allocations.
#[wasm_bindgen]
#[derive(Default)]
pub struct WasmExport {
    string_buf: String,
}

// Using JsValue instead of returning a String
// prevents copying all of the output in and out of wasm memory

#[wasm_bindgen]
impl WasmExport {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        WasmExport {
            string_buf: String::new(),
        }
    }

    #[wasm_bindgen]
    pub fn to_org(&mut self, s: &str) -> JsValue {
        self.string_buf.clear();
        match Org::export_buf(s, &mut self.string_buf, ConfigOptions::default()) {
            Ok(_) => JsValue::from_str(&self.string_buf),
            Err(e) => JsValue::from_str(&e.to_string()),
        }
    }

    #[wasm_bindgen]
    pub fn to_html(&mut self, s: &str) -> JsValue {
        self.string_buf.clear();

        match Html::export_buf(s, &mut self.string_buf, ConfigOptions::default()) {
            Ok(_) => JsValue::from_str(&self.string_buf),
            Err(e) => JsValue::from_str(&e.to_string()),
        }
    }
}

struct SyntaxNode {
    kind: i32,
    begin: i32,
    end: i32,
}

impl From<&Node<'_>> for SyntaxNode {
    fn from(value: &Node) -> Self {
        SyntaxNode {
            kind: expr_to_kind(&value.obj),
            begin: value.start as i32,
            end: value.end as i32,
        }
    }
}

fn expr_to_kind(val: &Expr) -> i32 {
    match val {
        Expr::Root(_) => 0,
        Expr::Italic(_) => 1,
        Expr::Bold(_) => 2,
        Expr::Entity(_) => 3,
        Expr::Emoji(_) => 4,
        Expr::Target(_) => 5,
        Expr::Macro(_) => 6,
        Expr::Underline(_) => 7,
        Expr::Verbatim(_) => 8,
        Expr::Code(_) => 9,
        Expr::Comment(_) => 10,
        Expr::InlineSrc(_) => 11,
        Expr::StrikeThrough(_) => 12,
        Expr::PlainLink(_) => 13,
        Expr::ExportSnippet(_) => 14,
        Expr::Keyword(_) | Expr::MacroDef(_) | Expr::Affiliated(_) => 15,
        Expr::Block(_) | Expr::LatexEnv(_) => 16,
        Expr::RegularLink(_) => 17,
        Expr::Table(_) | Expr::TableRow(_) | Expr::TableCell(_) => 18,
        Expr::Paragraph(_) => 19,
        Expr::Plain(_) => 20,
        Expr::PlainList(_) | Expr::Item(_) => 21,
        Expr::Heading(_) => 22,
        Expr::Drawer(_) => 23,
        Expr::FootnoteDef(_) => 24,
        Expr::FootnoteRef(_) => 25,
        _ => 26,
    }
}

fn postfix_translate(pool: &NodePool, curr_id: NodeID, build_vec: &mut Vec<i32>) -> i32 {
    let curr_node = &pool[curr_id];
    let temp = SyntaxNode::from(curr_node);

    let mut num_tot_children = 1;
    if let Some(children) = curr_node.obj.children() {
        for child_id in children {
            num_tot_children += postfix_translate(pool, *child_id, build_vec);
        }
    }

    build_vec.push(temp.kind);
    build_vec.push(temp.begin);
    build_vec.push(temp.end);
    build_vec.push(num_tot_children * 4);
    num_tot_children
}

#[wasm_bindgen]
pub fn syntaxable_entites(s: &str) -> Int32Array {
    let r = parse_org(s).pool;

    let mut build_vec: Vec<i32> = Vec::with_capacity(r.inner_vec.len() * 4);
    postfix_translate(&r, r.root_id(), &mut build_vec);
    Int32Array::from(build_vec.as_slice())
}
// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Root -> Paragraph -> Italic -> text

// text -> italic -> para -> root

// |1,4|     0,5|       0,5|     0, 5|
