use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "onDebug")]
    pub fn on_debug(kind: String, msg: String);
    #[wasm_bindgen(js_name = "onInput")]
    pub fn on_input();
    #[wasm_bindgen(js_name = "onOpen")]
    pub fn on_open();
    #[wasm_bindgen(js_name = "onCommit")]
    pub fn on_commit(text: String);
    #[wasm_bindgen(js_name = "onCancel")]
    pub fn on_cancel();
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = "invoke", catch)]
    pub async fn target_invoke(cmd: String, args: JsValue) -> Result<JsValue, JsValue>;
}

pub fn emit_debug_trace(kind: &str, msg: String) {
    on_debug(kind.into(), msg);
}
