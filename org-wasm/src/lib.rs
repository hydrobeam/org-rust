use org_exporter::Exporter;
use org_exporter::Html;
use org_exporter::Org;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_u32(a: u32);
}

// We want to avoid creating an entirely new string
// on every repearse: re-use the same buffer to save on allocations.
#[wasm_bindgen]
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
        Org::export_buf(s, &mut self.string_buf).unwrap();
        JsValue::from_str(&self.string_buf)
    }

    #[wasm_bindgen]
    pub fn to_html(&mut self, s: &str) -> JsValue {
        self.string_buf.clear();
        Html::export_buf(s, &mut self.string_buf).unwrap();
        JsValue::from_str(&self.string_buf)
    }
}

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
