use scte35_editor::core::{OutputFormat, ParseSettings, Scte35Document, patch_meta};
use scte35_editor::io::InputFormat;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    let input_text = use_state(String::new);
    let input_format = use_state(|| InputFormatUi::Auto);
    let strict = use_state(|| false);
    let no_crc = use_state(|| false);
    let output = use_state(|| None::<String>);
    let status = use_state(|| "Ready".to_string());
    let status_is_error = use_state(|| false);
    let flash_active = use_state(|| false);
    let flash_token = use_state(|| 0u32);
    let error_field_path = use_state(|| None::<String>);
    let tree_items = use_state(Vec::<TreeItem>::new);
    let selected_path = use_state(|| None::<String>);
    let edit_value = use_state(String::new);
    let current_doc_json = use_state(|| None::<String>);

    let on_input = {
        let input_text = input_text.clone();
        Callback::from(move |e: InputEvent| {
            let value = e
                .target_dyn_into::<web_sys::HtmlTextAreaElement>()
                .map(|input| input.value())
                .unwrap_or_default();
            input_text.set(value);
        })
    };

    let on_format_change = {
        let input_format = input_format.clone();
        Callback::from(move |e: Event| {
            let value = e
                .target_dyn_into::<web_sys::HtmlSelectElement>()
                .map(|select| select.value())
                .unwrap_or_default();
            input_format.set(InputFormatUi::from_str(&value));
        })
    };

    let on_strict_toggle = {
        let strict = strict.clone();
        Callback::from(move |e: Event| {
            let checked = e
                .target_dyn_into::<web_sys::HtmlInputElement>()
                .map(|input| input.checked())
                .unwrap_or(false);
            strict.set(checked);
        })
    };

    let on_no_crc_toggle = {
        let no_crc = no_crc.clone();
        Callback::from(move |e: Event| {
            let checked = e
                .target_dyn_into::<web_sys::HtmlInputElement>()
                .map(|input| input.checked())
                .unwrap_or(false);
            no_crc.set(checked);
        })
    };

    let on_parse = {
        let input_text = input_text.clone();
        let input_format = input_format.clone();
        let strict = strict.clone();
        let no_crc = no_crc.clone();
        let output = output.clone();
        let status = status.clone();
        let status_is_error = status_is_error.clone();
        let flash_active = flash_active.clone();
        let flash_token = flash_token.clone();
        let error_field_path = error_field_path.clone();
        let tree_items = tree_items.clone();
        let selected_path = selected_path.clone();
        let edit_value = edit_value.clone();
        let current_doc_json = current_doc_json.clone();
        Callback::from(move |_| {
            let input = (*input_text).clone();
            let format = (*input_format).into();
            let result = Scte35Document::parse(
                &input,
                format,
                ParseSettings {
                    validate_crc: !*no_crc,
                    strict: *strict,
                },
            )
            .and_then(|doc| {
                let json = doc.render(OutputFormat::Json)?;
                Ok((doc, json))
            })
            .and_then(|(doc, json)| render_output(&doc).map(|rendered| (doc, json, rendered)));
            match result {
                Ok((doc, json, rendered)) => {
                    let items = build_tree_items(&doc);
                    if let Some(first) = items.first() {
                        selected_path.set(first.path.clone());
                        edit_value.set(first.value.clone().unwrap_or_default());
                    } else {
                        selected_path.set(None);
                        edit_value.set(String::new());
                    }
                    tree_items.set(items);
                    current_doc_json.set(Some(json));
                    output.set(Some(rendered));
                    status.set("Parsed".to_string());
                    status_is_error.set(false);
                    error_field_path.set(None);
                }
                Err(err) => {
                    output.set(None);
                    tree_items.set(Vec::new());
                    selected_path.set(None);
                    edit_value.set(String::new());
                    current_doc_json.set(None);
                    status.set(err);
                    status_is_error.set(true);
                    flash_active.set(true);
                    flash_token.set(*flash_token + 1);
                    error_field_path.set(None);
                }
            }
        })
    };

    let on_new_time_signal = {
        let output = output.clone();
        let status = status.clone();
        let status_is_error = status_is_error.clone();
        let flash_active = flash_active.clone();
        let flash_token = flash_token.clone();
        let error_field_path = error_field_path.clone();
        let tree_items = tree_items.clone();
        let selected_path = selected_path.clone();
        let edit_value = edit_value.clone();
        let current_doc_json = current_doc_json.clone();
        Callback::from(move |_| {
            match Scte35Document::new_default_time_signal()
                .and_then(|doc| {
                    let json = doc.render(OutputFormat::Json)?;
                    Ok((doc, json))
                })
                .and_then(|(doc, json)| render_output(&doc).map(|rendered| (doc, json, rendered)))
            {
                Ok((doc, json, rendered)) => {
                    output.set(Some(rendered));
                    status.set("New time_signal created".to_string());
                    status_is_error.set(false);
                    error_field_path.set(None);
                    let items = build_tree_items(&doc);
                    if let Some(first) = items.first() {
                        selected_path.set(first.path.clone());
                        edit_value.set(first.value.clone().unwrap_or_default());
                    } else {
                        selected_path.set(None);
                        edit_value.set(String::new());
                    }
                    tree_items.set(items);
                    current_doc_json.set(Some(json));
                }
                Err(err) => {
                    output.set(None);
                    tree_items.set(Vec::new());
                    selected_path.set(None);
                    edit_value.set(String::new());
                    current_doc_json.set(None);
                    status.set(err);
                    status_is_error.set(true);
                    flash_active.set(true);
                    flash_token.set(*flash_token + 1);
                    error_field_path.set(None);
                }
            }
        })
    };

    let on_select = {
        let selected_path = selected_path.clone();
        let edit_value = edit_value.clone();
        Callback::from(move |item: TreeItem| {
            selected_path.set(item.path.clone());
            edit_value.set(item.value.unwrap_or_default());
        })
    };

    let on_edit_value = {
        let edit_value = edit_value.clone();
        Callback::from(move |e: InputEvent| {
            let value = e
                .target_dyn_into::<web_sys::HtmlInputElement>()
                .map(|input| input.value())
                .unwrap_or_default();
            edit_value.set(value);
        })
    };

    let on_apply = {
        let current_doc_json = current_doc_json.clone();
        let selected_path = selected_path.clone();
        let edit_value = edit_value.clone();
        let no_crc = no_crc.clone();
        let strict = strict.clone();
        let output = output.clone();
        let status = status.clone();
        let status_is_error = status_is_error.clone();
        let flash_active = flash_active.clone();
        let flash_token = flash_token.clone();
        let error_field_path = error_field_path.clone();
        let tree_items = tree_items.clone();
        Callback::from(move |_| {
            let path = selected_path.as_deref().unwrap_or("");
            if path.is_empty() {
                status.set("Select a path to edit".to_string());
                status_is_error.set(true);
                flash_active.set(true);
                flash_token.set(*flash_token + 1);
                error_field_path.set(None);
                return;
            }
            let Some(json) = current_doc_json.as_ref() else {
                status.set("No document loaded".to_string());
                status_is_error.set(true);
                flash_active.set(true);
                flash_token.set(*flash_token + 1);
                error_field_path.set(None);
                return;
            };
            let parsed = Scte35Document::parse(
                json,
                InputFormat::Json,
                ParseSettings {
                    validate_crc: !*no_crc,
                    strict: *strict,
                },
            );
            let mut doc = match parsed {
                Ok(doc) => doc,
                Err(err) => {
                    status.set(err);
                    status_is_error.set(true);
                    flash_active.set(true);
                    flash_token.set(*flash_token + 1);
                    error_field_path.set(None);
                    return;
                }
            };
            let changes = vec![(path.to_string(), (*edit_value).clone())];
            if let Err(err) = doc.apply_sets(&changes) {
                status.set(err);
                status_is_error.set(true);
                flash_active.set(true);
                flash_token.set(*flash_token + 1);
                error_field_path.set(Some(path.to_string()));
                return;
            }
            let json = match doc.render(OutputFormat::Json) {
                Ok(json) => json,
                Err(err) => {
                    status.set(err);
                    status_is_error.set(true);
                    flash_active.set(true);
                    flash_token.set(*flash_token + 1);
                    error_field_path.set(Some(path.to_string()));
                    return;
                }
            };
            let rendered = match render_output(&doc) {
                Ok(rendered) => rendered,
                Err(err) => {
                    status.set(err);
                    status_is_error.set(true);
                    flash_active.set(true);
                    flash_token.set(*flash_token + 1);
                    error_field_path.set(Some(path.to_string()));
                    return;
                }
            };
            let items = build_tree_items(&doc);
            tree_items.set(items);
            current_doc_json.set(Some(json));
            output.set(Some(rendered));
            status.set("Applied".to_string());
            status_is_error.set(false);
            error_field_path.set(None);
        })
    };

    let on_create = {
        let current_doc_json = current_doc_json.clone();
        let selected_path = selected_path.clone();
        let edit_value = edit_value.clone();
        let no_crc = no_crc.clone();
        let strict = strict.clone();
        let output = output.clone();
        let status = status.clone();
        let status_is_error = status_is_error.clone();
        let flash_active = flash_active.clone();
        let flash_token = flash_token.clone();
        let error_field_path = error_field_path.clone();
        let tree_items = tree_items.clone();
        Callback::from(move |_| {
            let path = selected_path.as_deref().unwrap_or("");
            if path.is_empty() {
                status.set("Select a path to create".to_string());
                status_is_error.set(true);
                flash_active.set(true);
                flash_token.set(*flash_token + 1);
                error_field_path.set(None);
                return;
            }
            let Some(json) = current_doc_json.as_ref() else {
                status.set("No document loaded".to_string());
                status_is_error.set(true);
                flash_active.set(true);
                flash_token.set(*flash_token + 1);
                error_field_path.set(None);
                return;
            };
            let parsed = Scte35Document::parse(
                json,
                InputFormat::Json,
                ParseSettings {
                    validate_crc: !*no_crc,
                    strict: *strict,
                },
            );
            let mut doc = match parsed {
                Ok(doc) => doc,
                Err(err) => {
                    status.set(err);
                    status_is_error.set(true);
                    flash_active.set(true);
                    flash_token.set(*flash_token + 1);
                    error_field_path.set(None);
                    return;
                }
            };
            let value = (*edit_value).clone();
            let mut create_path = path.to_string();
            if let Some(indexed) = expand_add_path(&doc, &create_path) {
                create_path = indexed;
            }
            let changes = vec![(create_path.clone(), value)];
            if let Err(err) = doc.apply_sets(&changes) {
                status.set(err);
                status_is_error.set(true);
                flash_active.set(true);
                flash_token.set(*flash_token + 1);
                error_field_path.set(Some(create_path));
                return;
            }
            let json = match doc.render(OutputFormat::Json) {
                Ok(json) => json,
                Err(err) => {
                    status.set(err);
                    status_is_error.set(true);
                    flash_active.set(true);
                    flash_token.set(*flash_token + 1);
                    error_field_path.set(None);
                    return;
                }
            };
            let rendered = match render_output(&doc) {
                Ok(rendered) => rendered,
                Err(err) => {
                    status.set(err);
                    status_is_error.set(true);
                    flash_active.set(true);
                    flash_token.set(*flash_token + 1);
                    error_field_path.set(None);
                    return;
                }
            };
            let items = build_tree_items(&doc);
            tree_items.set(items);
            current_doc_json.set(Some(json));
            output.set(Some(rendered));
            status.set(format!("Created via {create_path}"));
            status_is_error.set(false);
            error_field_path.set(None);
        })
    };

    let on_delete = {
        let current_doc_json = current_doc_json.clone();
        let selected_path = selected_path.clone();
        let no_crc = no_crc.clone();
        let strict = strict.clone();
        let output = output.clone();
        let status = status.clone();
        let status_is_error = status_is_error.clone();
        let flash_active = flash_active.clone();
        let flash_token = flash_token.clone();
        let error_field_path = error_field_path.clone();
        let tree_items = tree_items.clone();
        Callback::from(move |_| {
            let path = selected_path.as_deref().unwrap_or("");
            if path.is_empty() {
                status.set("Select a path to delete".to_string());
                status_is_error.set(true);
                flash_active.set(true);
                flash_token.set(*flash_token + 1);
                error_field_path.set(None);
                return;
            }
            let Some(target) = delete_target_from_path(path) else {
                status.set("Selected path is not deletable".to_string());
                status_is_error.set(true);
                flash_active.set(true);
                flash_token.set(*flash_token + 1);
                error_field_path.set(Some(path.to_string()));
                return;
            };
            let Some(json) = current_doc_json.as_ref() else {
                status.set("No document loaded".to_string());
                status_is_error.set(true);
                flash_active.set(true);
                flash_token.set(*flash_token + 1);
                error_field_path.set(None);
                return;
            };
            let parsed = Scte35Document::parse(
                json,
                InputFormat::Json,
                ParseSettings {
                    validate_crc: !*no_crc,
                    strict: *strict,
                },
            );
            let mut doc = match parsed {
                Ok(doc) => doc,
                Err(err) => {
                    status.set(err);
                    status_is_error.set(true);
                    flash_active.set(true);
                    flash_token.set(*flash_token + 1);
                    error_field_path.set(None);
                    return;
                }
            };
            if let Err(err) = doc.delete_target(&target) {
                status.set(err);
                status_is_error.set(true);
                flash_active.set(true);
                flash_token.set(*flash_token + 1);
                error_field_path.set(Some(target));
                return;
            }
            let json = match doc.render(OutputFormat::Json) {
                Ok(json) => json,
                Err(err) => {
                    status.set(err);
                    status_is_error.set(true);
                    flash_active.set(true);
                    flash_token.set(*flash_token + 1);
                    error_field_path.set(None);
                    return;
                }
            };
            let rendered = match render_output(&doc) {
                Ok(rendered) => rendered,
                Err(err) => {
                    status.set(err);
                    status_is_error.set(true);
                    flash_active.set(true);
                    flash_token.set(*flash_token + 1);
                    error_field_path.set(None);
                    return;
                }
            };
            let items = build_tree_items(&doc);
            tree_items.set(items);
            current_doc_json.set(Some(json));
            output.set(Some(rendered));
            status.set(format!("Deleted {target}"));
            status_is_error.set(false);
            error_field_path.set(None);
        })
    };

    {
        let flash_active = flash_active.clone();
        let flash_token = flash_token.clone();
        use_effect_with(*flash_token, move |_| {
            flash_active.set(true);
            if let Some(window) = web_sys::window() {
                let callback = Closure::once(move || {
                    flash_active.set(false);
                });
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    callback.as_ref().unchecked_ref(),
                    1600,
                );
                callback.forget();
            } else {
                flash_active.set(false);
            }
            || {}
        });
    }

    let selected_meta = selected_path.as_ref().and_then(|path| patch_meta(path));

    html! {
        <div class="app">
            <header class="app__header">
                <h1>{"scte35-editor"}</h1>
                <div class={
                    classes!(
                        "status",
                        if *status_is_error { "status--error" } else { "status--ok" },
                        if *flash_active { "status--flash" } else { "" }
                    )
                }>
                    {format!("Status: {}", &*status)}
                </div>
            </header>
            <main class="app__main">
                <section class="panel panel--tree">
                    <h2>{"Tree"}</h2>
                    <div class="tree">
                        { for tree_items.iter().cloned().map(|item| {
                            let on_select = on_select.clone();
                            let error_field_path = error_field_path.clone();
                            let is_selected = selected_path
                                .as_ref()
                                .map(|path| item.path.as_ref() == Some(path))
                                .unwrap_or(false);
                            let is_error = error_field_path
                                .as_ref()
                                .and_then(|path| item.path.as_ref().map(|item_path| item_path == path))
                                .unwrap_or(false);
                            let class = if is_selected {
                                if is_error {
                                    "tree__item tree__item--selected tree__item--error"
                                } else {
                                    "tree__item tree__item--selected"
                                }
                            } else if is_error {
                                "tree__item tree__item--error"
                            } else {
                                "tree__item"
                            };
                            let label = item.label.clone();
                            html! {
                                <button class={class} onclick={Callback::from(move |_| on_select.emit(item.clone()))}>
                                    {label}
                                </button>
                            }
                        })}
                    </div>
                </section>
                <section class="panel panel--editor">
                    <h2>{"Field Editor"}</h2>
                    <div class="controls">
                        <label>
                            {"Input format "}
                            <select onchange={on_format_change}>
                                { for InputFormatUi::all().into_iter().map(|item| {
                                    let value = item.as_str();
                                    html! { <option value={value}>{value}</option> }
                                })}
                            </select>
                        </label>
                        <label>
                            <input type="checkbox" onchange={on_strict_toggle} />
                            {" strict"}
                        </label>
                        <label>
                            <input type="checkbox" onchange={on_no_crc_toggle} />
                            {" no_crc"}
                        </label>
                    </div>
                    <textarea
                        class="input"
                        rows="8"
                        placeholder="Paste JSON / base64 / hex here"
                        value={(*input_text).clone()}
                        oninput={on_input}
                    />
                    <div class="actions">
                        <button onclick={on_parse}>{"Parse"}</button>
                        <button onclick={on_new_time_signal}>{"New time_signal"}</button>
                        <button onclick={on_apply}>{"Apply"}</button>
                        <button onclick={on_create}>{"Create"}</button>
                        <button onclick={on_delete}>{"Delete"}</button>
                    </div>
                    <div class="editor">
                        <label>
                            {"Selected path"}
                            <input
                                class="input"
                                type="text"
                                value={selected_path.as_deref().unwrap_or("").to_string()}
                                readonly={true}
                            />
                        </label>
                        <label>
                            {"Value"}
                            <input
                                class={classes!(
                                    "input",
                                    if error_field_path
                                        .as_ref()
                                        .and_then(|path| selected_path.as_ref().map(|selected| selected == path))
                                        .unwrap_or(false)
                                    {
                                        "input--error"
                                    } else {
                                        ""
                                    }
                                )}
                                type="text"
                                value={(*edit_value).clone()}
                                oninput={on_edit_value}
                                placeholder="Edit value here"
                            />
                        </label>
                        <div class="meta">
                            <span>
                                {format!("Type: {}", selected_meta.map(|m| format!("{:?}", m.value_type)).unwrap_or_else(|| "-".to_string()))}
                            </span>
                            <span>
                                {format!("Constraints: {}", selected_meta.map(|m| m.constraints).unwrap_or("-"))}
                            </span>
                            <span>
                                {format!("Example: {}", selected_meta.map(|m| m.example).unwrap_or("-"))}
                            </span>
                        </div>
                    </div>
                </section>
                <section class="panel panel--output">
                    <h2>{"Output"}</h2>
                    <pre>{output.as_deref().unwrap_or("No output yet.")}</pre>
                </section>
            </main>
        </div>
    }
}

#[derive(Clone, Debug)]
struct TreeItem {
    label: String,
    path: Option<String>,
    value: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum InputFormatUi {
    Auto,
    Json,
    Base64,
    Hex,
}

impl InputFormatUi {
    fn all() -> [InputFormatUi; 4] {
        [Self::Auto, Self::Json, Self::Base64, Self::Hex]
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Json => "json",
            Self::Base64 => "base64",
            Self::Hex => "hex",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "json" => Self::Json,
            "base64" => Self::Base64,
            "hex" => Self::Hex,
            _ => Self::Auto,
        }
    }
}

impl From<InputFormatUi> for InputFormat {
    fn from(value: InputFormatUi) -> Self {
        match value {
            InputFormatUi::Auto => InputFormat::Auto,
            InputFormatUi::Json => InputFormat::Json,
            InputFormatUi::Base64 => InputFormat::Base64,
            InputFormatUi::Hex => InputFormat::Hex,
        }
    }
}

fn render_output(document: &Scte35Document) -> Result<String, String> {
    let json_output = document.render(OutputFormat::Json)?;
    let json_value: serde_json::Value = serde_json::from_str(&json_output)
        .map_err(|err| format!("failed to parse rendered json output: {err}"))?;
    let hex_output = document.render(OutputFormat::Hex)?;
    let base64_output = document.render(OutputFormat::Base64)?;
    serde_json::to_string_pretty(&serde_json::json!({
        "json": json_value,
        "hex": hex_output,
        "base64": base64_output
    }))
    .map_err(|err| format!("failed to serialize output: {err}"))
}

fn expand_add_path(doc: &Scte35Document, path: &str) -> Option<String> {
    if path.contains('[') {
        return None;
    }
    let (kind, field) = path.split_once('.')?;
    let index = match kind {
        "segmentation" => count_descriptors(doc, DescriptorKind::Segmentation),
        "avail" => count_descriptors(doc, DescriptorKind::Avail),
        "dtmf" => count_descriptors(doc, DescriptorKind::Dtmf),
        "time" => count_descriptors(doc, DescriptorKind::Time),
        "audio" => count_descriptors(doc, DescriptorKind::Audio),
        "unknown" => count_descriptors(doc, DescriptorKind::Unknown),
        _ => return None,
    };
    Some(format!("{kind}[{index}].{field}"))
}

fn count_descriptors(doc: &Scte35Document, kind: DescriptorKind) -> usize {
    doc.section()
        .splice_descriptors
        .iter()
        .filter(|descriptor| match (descriptor, kind) {
            (scte35::SpliceDescriptor::Segmentation(_), DescriptorKind::Segmentation)
            | (scte35::SpliceDescriptor::Avail(_), DescriptorKind::Avail)
            | (scte35::SpliceDescriptor::Dtmf(_), DescriptorKind::Dtmf)
            | (scte35::SpliceDescriptor::Time(_), DescriptorKind::Time)
            | (scte35::SpliceDescriptor::Audio(_), DescriptorKind::Audio)
            | (scte35::SpliceDescriptor::Unknown { .. }, DescriptorKind::Unknown) => true,
            _ => false,
        })
        .count()
}

fn parse_index(path: &str, prefix: &str) -> Option<usize> {
    let rest = path.strip_prefix(prefix)?;
    let rest = rest.strip_prefix('[')?;
    let mut parts = rest.splitn(2, ']');
    let index_str = parts.next()?;
    let index = index_str.parse::<usize>().ok()?;
    Some(index)
}

fn delete_target_from_path(path: &str) -> Option<String> {
    if let Some(index) = parse_index(path, "splice_insert.component") {
        return Some(format!("splice_insert.component[{index}]"));
    }
    if let Some(index) = parse_index(path, "splice_schedule.component") {
        return Some(format!("splice_schedule.component[{index}]"));
    }
    if let Some(index) = parse_index(path, "segmentation") {
        return Some(format!("segmentation[{index}]"));
    }
    if let Some(index) = parse_index(path, "avail") {
        return Some(format!("avail[{index}]"));
    }
    if let Some(index) = parse_index(path, "dtmf") {
        return Some(format!("dtmf[{index}]"));
    }
    if let Some(index) = parse_index(path, "time") {
        return Some(format!("time[{index}]"));
    }
    if let Some(index) = parse_index(path, "audio") {
        return Some(format!("audio[{index}]"));
    }
    if let Some(index) = parse_index(path, "unknown") {
        return Some(format!("unknown[{index}]"));
    }
    None
}

fn build_tree_items(document: &Scte35Document) -> Vec<TreeItem> {
    let section = document.section();
    let mut items = Vec::new();
    items.push(TreeItem {
        label: format!("table_id: {}", section.table_id),
        path: Some("table_id".to_string()),
        value: Some(section.table_id.to_string()),
    });
    items.push(TreeItem {
        label: format!("pts_adjustment: {}", section.pts_adjustment),
        path: Some("pts_adjustment".to_string()),
        value: Some(section.pts_adjustment.to_string()),
    });
    items.push(TreeItem {
        label: format!("tier: {}", section.tier),
        path: Some("tier".to_string()),
        value: Some(section.tier.to_string()),
    });
    items.push(TreeItem {
        label: format!("cw_index: {}", section.cw_index),
        path: Some("cw_index".to_string()),
        value: Some(section.cw_index.to_string()),
    });
    let command = match &section.splice_command {
        scte35::SpliceCommand::SpliceNull => "splice_null",
        scte35::SpliceCommand::SpliceInsert(_) => "splice_insert",
        scte35::SpliceCommand::TimeSignal(_) => "time_signal",
        scte35::SpliceCommand::SpliceSchedule(_) => "splice_schedule",
        scte35::SpliceCommand::BandwidthReservation(_) => "bandwidth_reservation",
        scte35::SpliceCommand::PrivateCommand(_) => "private_command",
        scte35::SpliceCommand::Unknown => "unknown",
    };
    items.push(TreeItem {
        label: format!("splice_command: {command}"),
        path: Some("splice_command".to_string()),
        value: Some(command.to_string()),
    });

    match &section.splice_command {
        scte35::SpliceCommand::TimeSignal(signal) => {
            let immediate = signal.splice_time.pts_time.is_none();
            let pts_time = signal.splice_time.pts_time.unwrap_or(0);
            items.push(TreeItem {
                label: format!("splice_time.pts_time: {pts_time}"),
                path: Some("splice_time.pts_time".to_string()),
                value: Some(pts_time.to_string()),
            });
            items.push(TreeItem {
                label: format!("splice_time.immediate: {immediate}"),
                path: Some("splice_time.immediate".to_string()),
                value: Some(immediate.to_string()),
            });
        }
        scte35::SpliceCommand::SpliceInsert(insert) => {
            items.push(TreeItem {
                label: format!("splice_insert.splice_event_id: {}", insert.splice_event_id),
                path: Some("splice_insert.splice_event_id".to_string()),
                value: Some(insert.splice_event_id.to_string()),
            });
            items.push(TreeItem {
                label: format!(
                    "splice_insert.cancel: {}",
                    insert.splice_event_cancel_indicator != 0
                ),
                path: Some("splice_insert.cancel".to_string()),
                value: Some((insert.splice_event_cancel_indicator != 0).to_string()),
            });
            items.push(TreeItem {
                label: format!(
                    "splice_insert.out_of_network: {}",
                    insert.out_of_network_indicator != 0
                ),
                path: Some("splice_insert.out_of_network".to_string()),
                value: Some((insert.out_of_network_indicator != 0).to_string()),
            });
            items.push(TreeItem {
                label: format!(
                    "splice_insert.program_splice: {}",
                    insert.program_splice_flag != 0
                ),
                path: Some("splice_insert.program_splice".to_string()),
                value: Some((insert.program_splice_flag != 0).to_string()),
            });
            items.push(TreeItem {
                label: format!(
                    "splice_insert.splice_immediate: {}",
                    insert.splice_immediate_flag != 0
                ),
                path: Some("splice_insert.splice_immediate".to_string()),
                value: Some((insert.splice_immediate_flag != 0).to_string()),
            });
            let insert_pts = insert.splice_time.as_ref().and_then(|time| time.pts_time);
            let insert_immediate = insert_pts.is_none();
            items.push(TreeItem {
                label: format!(
                    "splice_insert.splice_time.pts_time: {}",
                    insert_pts.unwrap_or(0)
                ),
                path: Some("splice_insert.splice_time.pts_time".to_string()),
                value: Some(insert_pts.unwrap_or(0).to_string()),
            });
            items.push(TreeItem {
                label: format!("splice_insert.splice_time.immediate: {insert_immediate}"),
                path: Some("splice_insert.splice_time.immediate".to_string()),
                value: Some(insert_immediate.to_string()),
            });
            let duration_value = insert
                .break_duration
                .as_ref()
                .map(|d| d.duration)
                .unwrap_or(0);
            items.push(TreeItem {
                label: format!("splice_insert.duration: {duration_value}"),
                path: Some("splice_insert.duration".to_string()),
                value: Some(duration_value.to_string()),
            });
            items.push(TreeItem {
                label: "splice_insert.duration.clear".to_string(),
                path: Some("splice_insert.duration.clear".to_string()),
                value: Some("true".to_string()),
            });
            let auto_return = insert
                .break_duration
                .as_ref()
                .map(|d| d.auto_return != 0)
                .unwrap_or(false);
            items.push(TreeItem {
                label: format!("splice_insert.auto_return: {auto_return}"),
                path: Some("splice_insert.auto_return".to_string()),
                value: Some(auto_return.to_string()),
            });
            items.push(TreeItem {
                label: format!(
                    "splice_insert.unique_program_id: {}",
                    insert.unique_program_id
                ),
                path: Some("splice_insert.unique_program_id".to_string()),
                value: Some(insert.unique_program_id.to_string()),
            });
            items.push(TreeItem {
                label: format!("splice_insert.avail_num: {}", insert.avail_num),
                path: Some("splice_insert.avail_num".to_string()),
                value: Some(insert.avail_num.to_string()),
            });
            items.push(TreeItem {
                label: format!("splice_insert.avails_expected: {}", insert.avails_expected),
                path: Some("splice_insert.avails_expected".to_string()),
                value: Some(insert.avails_expected.to_string()),
            });
            for (index, component) in insert.components.iter().enumerate() {
                let pts_time = component
                    .splice_time
                    .as_ref()
                    .and_then(|time| time.pts_time)
                    .unwrap_or(0);
                let immediate = component.splice_time.is_none();
                items.push(TreeItem {
                    label: format!(
                        "splice_insert.component[{index}].tag: {}",
                        component.component_tag
                    ),
                    path: Some(format!("splice_insert.component[{index}].tag")),
                    value: Some(component.component_tag.to_string()),
                });
                items.push(TreeItem {
                    label: format!("splice_insert.component[{index}].pts_time: {pts_time}"),
                    path: Some(format!("splice_insert.component[{index}].pts_time")),
                    value: Some(pts_time.to_string()),
                });
                items.push(TreeItem {
                    label: format!("splice_insert.component[{index}].immediate: {immediate}"),
                    path: Some(format!("splice_insert.component[{index}].immediate")),
                    value: Some(immediate.to_string()),
                });
            }
            items.push(TreeItem {
                label: "splice_insert.component.add".to_string(),
                path: Some("splice_insert.component.add".to_string()),
                value: Some("tag=1,pts=90000".to_string()),
            });
            items.push(TreeItem {
                label: "splice_insert.component.clear".to_string(),
                path: Some("splice_insert.component.clear".to_string()),
                value: Some("true".to_string()),
            });
        }
        scte35::SpliceCommand::SpliceSchedule(schedule) => {
            items.push(TreeItem {
                label: format!(
                    "splice_schedule.splice_event_id: {}",
                    schedule.splice_event_id
                ),
                path: Some("splice_schedule.splice_event_id".to_string()),
                value: Some(schedule.splice_event_id.to_string()),
            });
            items.push(TreeItem {
                label: format!(
                    "splice_schedule.cancel: {}",
                    schedule.splice_event_cancel_indicator != 0
                ),
                path: Some("splice_schedule.cancel".to_string()),
                value: Some((schedule.splice_event_cancel_indicator != 0).to_string()),
            });
            items.push(TreeItem {
                label: format!(
                    "splice_schedule.out_of_network: {}",
                    schedule.out_of_network_indicator != 0
                ),
                path: Some("splice_schedule.out_of_network".to_string()),
                value: Some((schedule.out_of_network_indicator != 0).to_string()),
            });
            let utc_value = schedule.utc_splice_time.unwrap_or(0);
            items.push(TreeItem {
                label: format!("splice_schedule.utc_splice_time: {utc_value}"),
                path: Some("splice_schedule.utc_splice_time".to_string()),
                value: Some(utc_value.to_string()),
            });
            items.push(TreeItem {
                label: "splice_schedule.utc_splice_time.clear".to_string(),
                path: Some("splice_schedule.utc_splice_time.clear".to_string()),
                value: Some("true".to_string()),
            });
            let duration_value = schedule.splice_duration.unwrap_or(0);
            items.push(TreeItem {
                label: format!("splice_schedule.duration: {duration_value}"),
                path: Some("splice_schedule.duration".to_string()),
                value: Some(duration_value.to_string()),
            });
            items.push(TreeItem {
                label: "splice_schedule.duration.clear".to_string(),
                path: Some("splice_schedule.duration.clear".to_string()),
                value: Some("true".to_string()),
            });
            items.push(TreeItem {
                label: format!(
                    "splice_schedule.unique_program_id: {}",
                    schedule.unique_program_id
                ),
                path: Some("splice_schedule.unique_program_id".to_string()),
                value: Some(schedule.unique_program_id.to_string()),
            });
            for (index, component) in schedule.component_list.iter().enumerate() {
                items.push(TreeItem {
                    label: format!(
                        "splice_schedule.component[{index}].tag: {}",
                        component.component_tag
                    ),
                    path: Some(format!("splice_schedule.component[{index}].tag")),
                    value: Some(component.component_tag.to_string()),
                });
                items.push(TreeItem {
                    label: format!(
                        "splice_schedule.component[{index}].splice_mode: {}",
                        component.splice_mode_indicator
                    ),
                    path: Some(format!("splice_schedule.component[{index}].splice_mode")),
                    value: Some(component.splice_mode_indicator.to_string()),
                });
                let duration_value = component.splice_duration.unwrap_or(0);
                items.push(TreeItem {
                    label: format!("splice_schedule.component[{index}].duration: {duration_value}"),
                    path: Some(format!("splice_schedule.component[{index}].duration")),
                    value: Some(duration_value.to_string()),
                });
                let utc_value = component.utc_splice_time.unwrap_or(0);
                items.push(TreeItem {
                    label: format!(
                        "splice_schedule.component[{index}].utc_splice_time: {utc_value}"
                    ),
                    path: Some(format!(
                        "splice_schedule.component[{index}].utc_splice_time"
                    )),
                    value: Some(utc_value.to_string()),
                });
                items.push(TreeItem {
                    label: format!(
                        "splice_schedule.component[{index}].duration_flag: {}",
                        component.duration_flag != 0
                    ),
                    path: Some(format!("splice_schedule.component[{index}].duration_flag")),
                    value: Some((component.duration_flag != 0).to_string()),
                });
            }
            items.push(TreeItem {
                label: "splice_schedule.component.add".to_string(),
                path: Some("splice_schedule.component.add".to_string()),
                value: Some("tag=1,splice_mode=0,duration=120".to_string()),
            });
            items.push(TreeItem {
                label: "splice_schedule.component.clear".to_string(),
                path: Some("splice_schedule.component.clear".to_string()),
                value: Some("true".to_string()),
            });
        }
        _ => {}
    }

    let mut segmentation_index = 0;
    let mut avail_index = 0;
    let mut dtmf_index = 0;
    let mut time_index = 0;
    let mut audio_index = 0;
    let mut unknown_index = 0;

    for descriptor in &section.splice_descriptors {
        match descriptor {
            scte35::SpliceDescriptor::Segmentation(seg) => {
                let index = segmentation_index;
                segmentation_index += 1;
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].event_id: {}",
                        seg.segmentation_event_id
                    ),
                    path: Some(format!("segmentation[{index}].event_id")),
                    value: Some(seg.segmentation_event_id.to_string()),
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].cancel: {}",
                        seg.segmentation_event_cancel_indicator
                    ),
                    path: Some(format!("segmentation[{index}].cancel")),
                    value: Some(seg.segmentation_event_cancel_indicator.to_string()),
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].program: {}",
                        seg.program_segmentation_flag
                    ),
                    path: Some(format!("segmentation[{index}].program")),
                    value: Some(seg.program_segmentation_flag.to_string()),
                });
                let duration_value = seg.segmentation_duration.unwrap_or(0);
                items.push(TreeItem {
                    label: format!("segmentation[{index}].duration: {duration_value}"),
                    path: Some(format!("segmentation[{index}].duration")),
                    value: Some(duration_value.to_string()),
                });
                items.push(TreeItem {
                    label: format!("segmentation[{index}].duration.clear"),
                    path: Some(format!("segmentation[{index}].duration.clear")),
                    value: Some("true".to_string()),
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].delivery_not_restricted: {}",
                        seg.delivery_not_restricted_flag
                    ),
                    path: Some(format!("segmentation[{index}].delivery_not_restricted")),
                    value: Some(seg.delivery_not_restricted_flag.to_string()),
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].web_delivery_allowed: {}",
                        seg.web_delivery_allowed_flag.unwrap_or(false)
                    ),
                    path: Some(format!("segmentation[{index}].web_delivery_allowed")),
                    value: Some(seg.web_delivery_allowed_flag.unwrap_or(false).to_string()),
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].no_regional_blackout: {}",
                        seg.no_regional_blackout_flag.unwrap_or(false)
                    ),
                    path: Some(format!("segmentation[{index}].no_regional_blackout")),
                    value: Some(seg.no_regional_blackout_flag.unwrap_or(false).to_string()),
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].archive_allowed: {}",
                        seg.archive_allowed_flag.unwrap_or(false)
                    ),
                    path: Some(format!("segmentation[{index}].archive_allowed")),
                    value: Some(seg.archive_allowed_flag.unwrap_or(false).to_string()),
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].device_restrictions: {}",
                        seg.device_restrictions.unwrap_or(0)
                    ),
                    path: Some(format!("segmentation[{index}].device_restrictions")),
                    value: Some(seg.device_restrictions.unwrap_or(0).to_string()),
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].upid_type: {}",
                        u8::from(seg.segmentation_upid_type)
                    ),
                    path: Some(format!("segmentation[{index}].upid_type")),
                    value: Some(u8::from(seg.segmentation_upid_type).to_string()),
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].upid: {} bytes",
                        seg.segmentation_upid.len()
                    ),
                    path: Some(format!("segmentation[{index}].upid")),
                    value: Some(format!("0x{}", hex::encode(&seg.segmentation_upid))),
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].type_id: {}",
                        seg.segmentation_type_id
                    ),
                    path: Some(format!("segmentation[{index}].type_id")),
                    value: Some(seg.segmentation_type_id.to_string()),
                });
                items.push(TreeItem {
                    label: format!("segmentation[{index}].segment_num: {}", seg.segment_num),
                    path: Some(format!("segmentation[{index}].segment_num")),
                    value: Some(seg.segment_num.to_string()),
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].segments_expected: {}",
                        seg.segments_expected
                    ),
                    path: Some(format!("segmentation[{index}].segments_expected")),
                    value: Some(seg.segments_expected.to_string()),
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].sub_segment_num: {}",
                        seg.sub_segment_num.unwrap_or(0)
                    ),
                    path: Some(format!("segmentation[{index}].sub_segment_num")),
                    value: Some(seg.sub_segment_num.unwrap_or(0).to_string()),
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].sub_segments_expected: {}",
                        seg.sub_segments_expected.unwrap_or(0)
                    ),
                    path: Some(format!("segmentation[{index}].sub_segments_expected")),
                    value: Some(seg.sub_segments_expected.unwrap_or(0).to_string()),
                });
                items.push(TreeItem {
                    label: format!("segmentation[{index}].sub_segment.clear"),
                    path: Some(format!("segmentation[{index}].sub_segment.clear")),
                    value: Some("true".to_string()),
                });
            }
            scte35::SpliceDescriptor::Avail(avail) => {
                let index = avail_index;
                avail_index += 1;
                items.push(TreeItem {
                    label: format!(
                        "avail[{index}].provider_id: {} bytes",
                        avail.provider_avail_id.len()
                    ),
                    path: Some(format!("avail[{index}].provider_id")),
                    value: Some(format!("0x{}", hex::encode(&avail.provider_avail_id))),
                });
                items.push(TreeItem {
                    label: format!("avail[{index}].identifier: {}", avail.identifier),
                    path: Some(format!("avail[{index}].identifier")),
                    value: Some(avail.identifier.to_string()),
                });
            }
            scte35::SpliceDescriptor::Dtmf(dtmf) => {
                let index = dtmf_index;
                dtmf_index += 1;
                items.push(TreeItem {
                    label: format!("dtmf[{index}].preroll: {}", dtmf.preroll),
                    path: Some(format!("dtmf[{index}].preroll")),
                    value: Some(dtmf.preroll.to_string()),
                });
                items.push(TreeItem {
                    label: format!("dtmf[{index}].chars: {} bytes", dtmf.dtmf_chars.len()),
                    path: Some(format!("dtmf[{index}].chars")),
                    value: Some(format!("0x{}", hex::encode(&dtmf.dtmf_chars))),
                });
                items.push(TreeItem {
                    label: format!("dtmf[{index}].identifier: {}", dtmf.identifier),
                    path: Some(format!("dtmf[{index}].identifier")),
                    value: Some(dtmf.identifier.to_string()),
                });
            }
            scte35::SpliceDescriptor::Time(time) => {
                let index = time_index;
                time_index += 1;
                items.push(TreeItem {
                    label: format!(
                        "time[{index}].tai_seconds: {} bytes",
                        time.tai_seconds.len()
                    ),
                    path: Some(format!("time[{index}].tai_seconds")),
                    value: Some(format!("0x{}", hex::encode(&time.tai_seconds))),
                });
                items.push(TreeItem {
                    label: format!("time[{index}].tai_ns: {} bytes", time.tai_ns.len()),
                    path: Some(format!("time[{index}].tai_ns")),
                    value: Some(format!("0x{}", hex::encode(&time.tai_ns))),
                });
                items.push(TreeItem {
                    label: format!("time[{index}].utc_offset: {} bytes", time.utc_offset.len()),
                    path: Some(format!("time[{index}].utc_offset")),
                    value: Some(format!("0x{}", hex::encode(&time.utc_offset))),
                });
                items.push(TreeItem {
                    label: format!("time[{index}].identifier: {}", time.identifier),
                    path: Some(format!("time[{index}].identifier")),
                    value: Some(time.identifier.to_string()),
                });
            }
            scte35::SpliceDescriptor::Audio(audio) => {
                let index = audio_index;
                audio_index += 1;
                items.push(TreeItem {
                    label: format!(
                        "audio[{index}].components: {} bytes",
                        audio.audio_components.len()
                    ),
                    path: Some(format!("audio[{index}].components")),
                    value: Some(format!("0x{}", hex::encode(&audio.audio_components))),
                });
                items.push(TreeItem {
                    label: format!("audio[{index}].identifier: {}", audio.identifier),
                    path: Some(format!("audio[{index}].identifier")),
                    value: Some(audio.identifier.to_string()),
                });
            }
            scte35::SpliceDescriptor::Unknown {
                tag,
                length: _,
                data,
            } => {
                let index = unknown_index;
                unknown_index += 1;
                items.push(TreeItem {
                    label: format!("unknown[{index}].tag: {}", tag),
                    path: Some(format!("unknown[{index}].tag")),
                    value: Some(tag.to_string()),
                });
                items.push(TreeItem {
                    label: format!("unknown[{index}].data: {} bytes", data.len()),
                    path: Some(format!("unknown[{index}].data")),
                    value: Some(format!("0x{}", hex::encode(data))),
                });
            }
        }
    }

    items.push(TreeItem {
        label: "segmentation.add".to_string(),
        path: Some("segmentation.event_id".to_string()),
        value: Some("1".to_string()),
    });
    items.push(TreeItem {
        label: "avail.add".to_string(),
        path: Some("avail.provider_id".to_string()),
        value: Some("0x41424344".to_string()),
    });
    items.push(TreeItem {
        label: "dtmf.add".to_string(),
        path: Some("dtmf.preroll".to_string()),
        value: Some("10".to_string()),
    });
    items.push(TreeItem {
        label: "time.add".to_string(),
        path: Some("time.tai_seconds".to_string()),
        value: Some("0x000000000001".to_string()),
    });
    items.push(TreeItem {
        label: "audio.add".to_string(),
        path: Some("audio.components".to_string()),
        value: Some("0x1122".to_string()),
    });
    items.push(TreeItem {
        label: "unknown.add".to_string(),
        path: Some("unknown.tag".to_string()),
        value: Some("7".to_string()),
    });

    items
}

#[derive(Copy, Clone, Debug)]
enum DescriptorKind {
    Segmentation,
    Avail,
    Dtmf,
    Time,
    Audio,
    Unknown,
}

fn main() {
    yew::Renderer::<App>::new().render();
}
