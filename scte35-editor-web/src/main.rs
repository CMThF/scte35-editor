use scte35_editor::core::{
    OutputFormat, ParseSettings, PatchValueType, Scte35Document, patch_meta,
};
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
    let new_menu_open = use_state(|| false);
    let new_template = use_state(|| "time_signal".to_string());
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

    let on_new_menu_toggle = {
        let new_menu_open = new_menu_open.clone();
        Callback::from(move |_| {
            new_menu_open.set(!*new_menu_open);
        })
    };

    let on_new_template_change = {
        let new_template = new_template.clone();
        Callback::from(move |e: Event| {
            let value = e
                .target_dyn_into::<web_sys::HtmlSelectElement>()
                .map(|select| select.value())
                .unwrap_or_else(|| "time_signal".to_string());
            new_template.set(value);
        })
    };

    let on_new_create = {
        let new_template = new_template.clone();
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
            let template = (*new_template).clone();
            let new_doc = match template.as_str() {
                "time_signal" => Scte35Document::new_default_time_signal(),
                "splice_null" => Scte35Document::new_default_splice_null(),
                "splice_insert" => Scte35Document::new_default_splice_insert(),
                "splice_schedule" => Scte35Document::new_default_splice_schedule(),
                _ => Err("Unknown template".to_string()),
            };
            match new_doc
                .and_then(|doc| {
                    let json = doc.render(OutputFormat::Json)?;
                    Ok((doc, json))
                })
                .and_then(|(doc, json)| render_output(&doc).map(|rendered| (doc, json, rendered)))
            {
                Ok((doc, json, rendered)) => {
                    output.set(Some(rendered));
                    status.set(format!("New {template} created"));
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

    let on_create_for = {
        let current_doc_json = current_doc_json.clone();
        let no_crc = no_crc.clone();
        let strict = strict.clone();
        let output = output.clone();
        let status = status.clone();
        let status_is_error = status_is_error.clone();
        let flash_active = flash_active.clone();
        let flash_token = flash_token.clone();
        let error_field_path = error_field_path.clone();
        let tree_items = tree_items.clone();
        let selected_path = selected_path.clone();
        let edit_value = edit_value.clone();
        Callback::from(move |(path, value): (String, String)| {
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
            let mut create_path = path.clone();
            if let Some(indexed) = expand_add_path(&doc, &create_path) {
                create_path = indexed;
            }
            let changes = vec![(create_path.clone(), value.clone())];
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
            if let Some(first) = items.first() {
                selected_path.set(first.path.clone());
                edit_value.set(first.value.clone().unwrap_or_default());
            }
            tree_items.set(items);
            current_doc_json.set(Some(json));
            output.set(Some(rendered));
            status.set(format!("Created via {create_path}"));
            status_is_error.set(false);
            error_field_path.set(None);
        })
    };

    let on_delete_for = {
        let current_doc_json = current_doc_json.clone();
        let no_crc = no_crc.clone();
        let strict = strict.clone();
        let output = output.clone();
        let status = status.clone();
        let status_is_error = status_is_error.clone();
        let flash_active = flash_active.clone();
        let flash_token = flash_token.clone();
        let error_field_path = error_field_path.clone();
        let tree_items = tree_items.clone();
        let selected_path = selected_path.clone();
        let edit_value = edit_value.clone();
        Callback::from(move |target: String| {
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
            if let Some(first) = items.first() {
                selected_path.set(first.path.clone());
                edit_value.set(first.value.clone().unwrap_or_default());
            }
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
    let is_bool_field = selected_meta
        .map(|meta| matches!(meta.value_type, PatchValueType::Bool))
        .unwrap_or(false);
    let is_splice_command_field = selected_meta
        .map(|meta| matches!(meta.value_type, PatchValueType::SpliceCommand))
        .unwrap_or(false);

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
                            let on_create_for = on_create_for.clone();
                            let on_delete_for = on_delete_for.clone();
                            let error_field_path = error_field_path.clone();
                            let is_selected = selected_path
                                .as_ref()
                                .map(|path| item.path.as_ref() == Some(path))
                                .unwrap_or(false);
                            let createable = item
                                .path
                                .as_ref()
                                .map(|path| path.ends_with(".add") || path.contains(".add"))
                                .unwrap_or(false);
                            let delete_target = item
                                .path
                                .as_deref()
                                .and_then(delete_target_from_path);
                            let is_error = error_field_path
                                .as_ref()
                                .and_then(|path| item.path.as_ref().map(|item_path| item_path == path))
                                .unwrap_or(false);
                            let row_class = if item.is_group {
                                "tree__row tree__row--group"
                            } else if item.group.is_some() {
                                "tree__row tree__row--child"
                            } else {
                                "tree__row"
                            };
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
                            let item_for_select = item.clone();
                            html! {
                                <div class={row_class}>
                                    <button class={class} onclick={Callback::from(move |_| on_select.emit(item_for_select.clone()))}>
                                        {label}
                                    </button>
                                    {
                                        if createable {
                                            let path = item.path.clone().unwrap_or_default();
                                            let value = item.value.clone().unwrap_or_default();
                                            html! {
                                                <button
                                                    class="tree__action tree__action--create"
                                                    onclick={Callback::from(move |_| on_create_for.emit((path.clone(), value.clone())))}
                                                >
                                                    {"Create"}
                                                </button>
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                    {
                                        if let Some(target) = delete_target {
                                            html! {
                                                <button
                                                    class="tree__action tree__action--delete"
                                                    onclick={Callback::from(move |_| on_delete_for.emit(target.clone()))}
                                                >
                                                    {"Delete"}
                                                </button>
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                </div>
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
                        <button onclick={on_apply}>{"Apply"}</button>
                        <button onclick={on_new_menu_toggle}>{"New"}</button>
                    </div>
                    {
                        if *new_menu_open {
                            html! {
                                <div class="new-menu">
                                    <label>
                                        {"Template "}
                                        <select onchange={on_new_template_change} value={(*new_template).clone()}>
                                            <option value="time_signal">{"time_signal"}</option>
                                            <option value="splice_null">{"splice_null"}</option>
                                            <option value="splice_insert">{"splice_insert"}</option>
                                            <option value="splice_schedule">{"splice_schedule"}</option>
                                        </select>
                                    </label>
                                    <button onclick={on_new_create}>{"Create"}</button>
                                </div>
                            }
                        } else {
                            html! {}
                        }
                    }
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
                            {
                                if is_bool_field || is_splice_command_field {
                                    let on_change = {
                                        let edit_value = edit_value.clone();
                                        Callback::from(move |e: Event| {
                                            let value = e
                                                .target_dyn_into::<web_sys::HtmlSelectElement>()
                                                .map(|select| select.value())
                                                .unwrap_or_default();
                                            edit_value.set(value);
                                        })
                                    };
                                    let options: Vec<&'static str> = if is_bool_field {
                                        vec!["true", "false"]
                                    } else {
                                        vec![
                                            "splice_null",
                                            "splice_insert",
                                            "time_signal",
                                            "splice_schedule",
                                            "bandwidth_reservation",
                                            "private_command",
                                        ]
                                    };
                                    let selected_value = (*edit_value).clone();
                                    html! {
                                        <select
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
                                            onchange={on_change}
                                            value={selected_value}
                                        >
                                            { for options.into_iter().map(|value| {
                                                html! { <option value={value}>{value}</option> }
                                            })}
                                        </select>
                                    }
                                } else {
                                    html! {
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
                                    }
                                }
                            }
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
    group: Option<String>,
    is_group: bool,
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

fn parse_index_only(path: &str, prefix: &str) -> Option<usize> {
    let rest = path.strip_prefix(prefix)?;
    let rest = rest.strip_prefix('[')?;
    let mut parts = rest.splitn(2, ']');
    let index_str = parts.next()?;
    let index = index_str.parse::<usize>().ok()?;
    let remainder = parts.next().unwrap_or("");
    if remainder.is_empty() {
        Some(index)
    } else {
        None
    }
}

fn delete_target_from_path(path: &str) -> Option<String> {
    if let Some(index) = parse_index_only(path, "splice_insert.component") {
        return Some(format!("splice_insert.component[{index}]"));
    }
    if let Some(index) = parse_index_only(path, "splice_schedule.component") {
        return Some(format!("splice_schedule.component[{index}]"));
    }
    if let Some(index) = parse_index_only(path, "segmentation") {
        return Some(format!("segmentation[{index}]"));
    }
    if let Some(index) = parse_index_only(path, "avail") {
        return Some(format!("avail[{index}]"));
    }
    if let Some(index) = parse_index_only(path, "dtmf") {
        return Some(format!("dtmf[{index}]"));
    }
    if let Some(index) = parse_index_only(path, "time") {
        return Some(format!("time[{index}]"));
    }
    if let Some(index) = parse_index_only(path, "audio") {
        return Some(format!("audio[{index}]"));
    }
    if let Some(index) = parse_index_only(path, "unknown") {
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
        group: None,
        is_group: false,
    });
    items.push(TreeItem {
        label: format!("pts_adjustment: {}", section.pts_adjustment),
        path: Some("pts_adjustment".to_string()),
        value: Some(section.pts_adjustment.to_string()),
        group: None,
        is_group: false,
    });
    items.push(TreeItem {
        label: format!("tier: {}", section.tier),
        path: Some("tier".to_string()),
        value: Some(section.tier.to_string()),
        group: None,
        is_group: false,
    });
    items.push(TreeItem {
        label: format!("cw_index: {}", section.cw_index),
        path: Some("cw_index".to_string()),
        value: Some(section.cw_index.to_string()),
        group: None,
        is_group: false,
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
        group: None,
        is_group: false,
    });

    match &section.splice_command {
        scte35::SpliceCommand::TimeSignal(signal) => {
            let immediate = signal.splice_time.pts_time.is_none();
            let pts_time = signal.splice_time.pts_time.unwrap_or(0);
            items.push(TreeItem {
                label: format!("splice_time.pts_time: {pts_time}"),
                path: Some("splice_time.pts_time".to_string()),
                value: Some(pts_time.to_string()),
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: format!("splice_time.immediate: {immediate}"),
                path: Some("splice_time.immediate".to_string()),
                value: Some(immediate.to_string()),
                group: None,
                is_group: false,
            });
        }
        scte35::SpliceCommand::SpliceInsert(insert) => {
            items.push(TreeItem {
                label: format!("splice_insert.splice_event_id: {}", insert.splice_event_id),
                path: Some("splice_insert.splice_event_id".to_string()),
                value: Some(insert.splice_event_id.to_string()),
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: format!(
                    "splice_insert.cancel: {}",
                    insert.splice_event_cancel_indicator != 0
                ),
                path: Some("splice_insert.cancel".to_string()),
                value: Some((insert.splice_event_cancel_indicator != 0).to_string()),
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: format!(
                    "splice_insert.out_of_network: {}",
                    insert.out_of_network_indicator != 0
                ),
                path: Some("splice_insert.out_of_network".to_string()),
                value: Some((insert.out_of_network_indicator != 0).to_string()),
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: format!(
                    "splice_insert.program_splice: {}",
                    insert.program_splice_flag != 0
                ),
                path: Some("splice_insert.program_splice".to_string()),
                value: Some((insert.program_splice_flag != 0).to_string()),
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: format!(
                    "splice_insert.splice_immediate: {}",
                    insert.splice_immediate_flag != 0
                ),
                path: Some("splice_insert.splice_immediate".to_string()),
                value: Some((insert.splice_immediate_flag != 0).to_string()),
                group: None,
                is_group: false,
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
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: format!("splice_insert.splice_time.immediate: {insert_immediate}"),
                path: Some("splice_insert.splice_time.immediate".to_string()),
                value: Some(insert_immediate.to_string()),
                group: None,
                is_group: false,
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
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: "splice_insert.duration.clear".to_string(),
                path: Some("splice_insert.duration.clear".to_string()),
                value: Some("true".to_string()),
                group: None,
                is_group: false,
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
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: format!(
                    "splice_insert.unique_program_id: {}",
                    insert.unique_program_id
                ),
                path: Some("splice_insert.unique_program_id".to_string()),
                value: Some(insert.unique_program_id.to_string()),
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: format!("splice_insert.avail_num: {}", insert.avail_num),
                path: Some("splice_insert.avail_num".to_string()),
                value: Some(insert.avail_num.to_string()),
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: format!("splice_insert.avails_expected: {}", insert.avails_expected),
                path: Some("splice_insert.avails_expected".to_string()),
                value: Some(insert.avails_expected.to_string()),
                group: None,
                is_group: false,
            });
            for (index, component) in insert.components.iter().enumerate() {
                items.push(TreeItem {
                    label: format!("splice_insert.component[{index}]"),
                    path: Some(format!("splice_insert.component[{index}]")),
                    value: None,
                    group: Some("splice_insert.components".to_string()),
                    is_group: true,
                });
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
                    group: Some(format!("splice_insert.component[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!("splice_insert.component[{index}].pts_time: {pts_time}"),
                    path: Some(format!("splice_insert.component[{index}].pts_time")),
                    value: Some(pts_time.to_string()),
                    group: Some(format!("splice_insert.component[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!("splice_insert.component[{index}].immediate: {immediate}"),
                    path: Some(format!("splice_insert.component[{index}].immediate")),
                    value: Some(immediate.to_string()),
                    group: Some(format!("splice_insert.component[{index}]")),

                    is_group: false,
                });
            }
            items.push(TreeItem {
                label: "splice_insert.component.add".to_string(),
                path: Some("splice_insert.component.add".to_string()),
                value: Some("tag=1,pts=90000".to_string()),
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: "splice_insert.component.clear".to_string(),
                path: Some("splice_insert.component.clear".to_string()),
                value: Some("true".to_string()),
                group: None,
                is_group: false,
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
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: format!(
                    "splice_schedule.cancel: {}",
                    schedule.splice_event_cancel_indicator != 0
                ),
                path: Some("splice_schedule.cancel".to_string()),
                value: Some((schedule.splice_event_cancel_indicator != 0).to_string()),
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: format!(
                    "splice_schedule.out_of_network: {}",
                    schedule.out_of_network_indicator != 0
                ),
                path: Some("splice_schedule.out_of_network".to_string()),
                value: Some((schedule.out_of_network_indicator != 0).to_string()),
                group: None,
                is_group: false,
            });
            let utc_value = schedule.utc_splice_time.unwrap_or(0);
            items.push(TreeItem {
                label: format!("splice_schedule.utc_splice_time: {utc_value}"),
                path: Some("splice_schedule.utc_splice_time".to_string()),
                value: Some(utc_value.to_string()),
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: "splice_schedule.utc_splice_time.clear".to_string(),
                path: Some("splice_schedule.utc_splice_time.clear".to_string()),
                value: Some("true".to_string()),
                group: None,
                is_group: false,
            });
            let duration_value = schedule.splice_duration.unwrap_or(0);
            items.push(TreeItem {
                label: format!("splice_schedule.duration: {duration_value}"),
                path: Some("splice_schedule.duration".to_string()),
                value: Some(duration_value.to_string()),
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: "splice_schedule.duration.clear".to_string(),
                path: Some("splice_schedule.duration.clear".to_string()),
                value: Some("true".to_string()),
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: format!(
                    "splice_schedule.unique_program_id: {}",
                    schedule.unique_program_id
                ),
                path: Some("splice_schedule.unique_program_id".to_string()),
                value: Some(schedule.unique_program_id.to_string()),
                group: None,
                is_group: false,
            });
            for (index, component) in schedule.component_list.iter().enumerate() {
                items.push(TreeItem {
                    label: format!("splice_schedule.component[{index}]"),
                    path: Some(format!("splice_schedule.component[{index}]")),
                    value: None,
                    group: Some("splice_schedule.components".to_string()),
                    is_group: true,
                });
                items.push(TreeItem {
                    label: format!(
                        "splice_schedule.component[{index}].tag: {}",
                        component.component_tag
                    ),
                    path: Some(format!("splice_schedule.component[{index}].tag")),
                    value: Some(component.component_tag.to_string()),
                    group: Some(format!("splice_schedule.component[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!(
                        "splice_schedule.component[{index}].splice_mode: {}",
                        component.splice_mode_indicator
                    ),
                    path: Some(format!("splice_schedule.component[{index}].splice_mode")),
                    value: Some(component.splice_mode_indicator.to_string()),
                    group: Some(format!("splice_schedule.component[{index}]")),

                    is_group: false,
                });
                let duration_value = component.splice_duration.unwrap_or(0);
                items.push(TreeItem {
                    label: format!("splice_schedule.component[{index}].duration: {duration_value}"),
                    path: Some(format!("splice_schedule.component[{index}].duration")),
                    value: Some(duration_value.to_string()),
                    group: Some(format!("splice_schedule.component[{index}]")),

                    is_group: false,
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
                    group: None,
                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!(
                        "splice_schedule.component[{index}].duration_flag: {}",
                        component.duration_flag != 0
                    ),
                    path: Some(format!("splice_schedule.component[{index}].duration_flag")),
                    value: Some((component.duration_flag != 0).to_string()),
                    group: Some(format!("splice_schedule.component[{index}]")),

                    is_group: false,
                });
            }
            items.push(TreeItem {
                label: "splice_schedule.component.add".to_string(),
                path: Some("splice_schedule.component.add".to_string()),
                value: Some("tag=1,splice_mode=0,duration=120".to_string()),
                group: None,
                is_group: false,
            });
            items.push(TreeItem {
                label: "splice_schedule.component.clear".to_string(),
                path: Some("splice_schedule.component.clear".to_string()),
                value: Some("true".to_string()),
                group: None,
                is_group: false,
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
                    label: format!("segmentation[{index}]"),
                    path: Some(format!("segmentation[{index}]")),
                    value: None,
                    group: Some("descriptors".to_string()),
                    is_group: true,
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].event_id: {}",
                        seg.segmentation_event_id
                    ),
                    path: Some(format!("segmentation[{index}].event_id")),
                    value: Some(seg.segmentation_event_id.to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].cancel: {}",
                        seg.segmentation_event_cancel_indicator
                    ),
                    path: Some(format!("segmentation[{index}].cancel")),
                    value: Some(seg.segmentation_event_cancel_indicator.to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].program: {}",
                        seg.program_segmentation_flag
                    ),
                    path: Some(format!("segmentation[{index}].program")),
                    value: Some(seg.program_segmentation_flag.to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                let duration_value = seg.segmentation_duration.unwrap_or(0);
                items.push(TreeItem {
                    label: format!("segmentation[{index}].duration: {duration_value}"),
                    path: Some(format!("segmentation[{index}].duration")),
                    value: Some(duration_value.to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!("segmentation[{index}].duration.clear"),
                    path: Some(format!("segmentation[{index}].duration.clear")),
                    value: Some("true".to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].delivery_not_restricted: {}",
                        seg.delivery_not_restricted_flag
                    ),
                    path: Some(format!("segmentation[{index}].delivery_not_restricted")),
                    value: Some(seg.delivery_not_restricted_flag.to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].web_delivery_allowed: {}",
                        seg.web_delivery_allowed_flag.unwrap_or(false)
                    ),
                    path: Some(format!("segmentation[{index}].web_delivery_allowed")),
                    value: Some(seg.web_delivery_allowed_flag.unwrap_or(false).to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].no_regional_blackout: {}",
                        seg.no_regional_blackout_flag.unwrap_or(false)
                    ),
                    path: Some(format!("segmentation[{index}].no_regional_blackout")),
                    value: Some(seg.no_regional_blackout_flag.unwrap_or(false).to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].archive_allowed: {}",
                        seg.archive_allowed_flag.unwrap_or(false)
                    ),
                    path: Some(format!("segmentation[{index}].archive_allowed")),
                    value: Some(seg.archive_allowed_flag.unwrap_or(false).to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].device_restrictions: {}",
                        seg.device_restrictions.unwrap_or(0)
                    ),
                    path: Some(format!("segmentation[{index}].device_restrictions")),
                    value: Some(seg.device_restrictions.unwrap_or(0).to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].upid_type: {}",
                        u8::from(seg.segmentation_upid_type)
                    ),
                    path: Some(format!("segmentation[{index}].upid_type")),
                    value: Some(u8::from(seg.segmentation_upid_type).to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].upid: {} bytes",
                        seg.segmentation_upid.len()
                    ),
                    path: Some(format!("segmentation[{index}].upid")),
                    value: Some(format!("0x{}", hex::encode(&seg.segmentation_upid))),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].type_id: {}",
                        seg.segmentation_type_id
                    ),
                    path: Some(format!("segmentation[{index}].type_id")),
                    value: Some(seg.segmentation_type_id.to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!("segmentation[{index}].segment_num: {}", seg.segment_num),
                    path: Some(format!("segmentation[{index}].segment_num")),
                    value: Some(seg.segment_num.to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].segments_expected: {}",
                        seg.segments_expected
                    ),
                    path: Some(format!("segmentation[{index}].segments_expected")),
                    value: Some(seg.segments_expected.to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].sub_segment_num: {}",
                        seg.sub_segment_num.unwrap_or(0)
                    ),
                    path: Some(format!("segmentation[{index}].sub_segment_num")),
                    value: Some(seg.sub_segment_num.unwrap_or(0).to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!(
                        "segmentation[{index}].sub_segments_expected: {}",
                        seg.sub_segments_expected.unwrap_or(0)
                    ),
                    path: Some(format!("segmentation[{index}].sub_segments_expected")),
                    value: Some(seg.sub_segments_expected.unwrap_or(0).to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!("segmentation[{index}].sub_segment.clear"),
                    path: Some(format!("segmentation[{index}].sub_segment.clear")),
                    value: Some("true".to_string()),
                    group: Some(format!("segmentation[{index}]")),

                    is_group: false,
                });
            }
            scte35::SpliceDescriptor::Avail(avail) => {
                let index = avail_index;
                avail_index += 1;
                items.push(TreeItem {
                    label: format!("avail[{index}]"),
                    path: Some(format!("avail[{index}]")),
                    value: None,
                    group: Some("descriptors".to_string()),
                    is_group: true,
                });
                items.push(TreeItem {
                    label: format!(
                        "avail[{index}].provider_id: {} bytes",
                        avail.provider_avail_id.len()
                    ),
                    path: Some(format!("avail[{index}].provider_id")),
                    value: Some(format!("0x{}", hex::encode(&avail.provider_avail_id))),
                    group: Some(format!("avail[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!("avail[{index}].identifier: {}", avail.identifier),
                    path: Some(format!("avail[{index}].identifier")),
                    value: Some(avail.identifier.to_string()),
                    group: Some(format!("avail[{index}]")),

                    is_group: false,
                });
            }
            scte35::SpliceDescriptor::Dtmf(dtmf) => {
                let index = dtmf_index;
                dtmf_index += 1;
                items.push(TreeItem {
                    label: format!("dtmf[{index}]"),
                    path: Some(format!("dtmf[{index}]")),
                    value: None,
                    group: Some("descriptors".to_string()),
                    is_group: true,
                });
                items.push(TreeItem {
                    label: format!("dtmf[{index}].preroll: {}", dtmf.preroll),
                    path: Some(format!("dtmf[{index}].preroll")),
                    value: Some(dtmf.preroll.to_string()),
                    group: Some(format!("dtmf[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!("dtmf[{index}].chars: {} bytes", dtmf.dtmf_chars.len()),
                    path: Some(format!("dtmf[{index}].chars")),
                    value: Some(format!("0x{}", hex::encode(&dtmf.dtmf_chars))),
                    group: Some(format!("dtmf[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!("dtmf[{index}].identifier: {}", dtmf.identifier),
                    path: Some(format!("dtmf[{index}].identifier")),
                    value: Some(dtmf.identifier.to_string()),
                    group: Some(format!("dtmf[{index}]")),

                    is_group: false,
                });
            }
            scte35::SpliceDescriptor::Time(time) => {
                let index = time_index;
                time_index += 1;
                items.push(TreeItem {
                    label: format!("time[{index}]"),
                    path: Some(format!("time[{index}]")),
                    value: None,
                    group: Some("descriptors".to_string()),
                    is_group: true,
                });
                items.push(TreeItem {
                    label: format!(
                        "time[{index}].tai_seconds: {} bytes",
                        time.tai_seconds.len()
                    ),
                    path: Some(format!("time[{index}].tai_seconds")),
                    value: Some(format!("0x{}", hex::encode(&time.tai_seconds))),
                    group: Some(format!("time[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!("time[{index}].tai_ns: {} bytes", time.tai_ns.len()),
                    path: Some(format!("time[{index}].tai_ns")),
                    value: Some(format!("0x{}", hex::encode(&time.tai_ns))),
                    group: Some(format!("time[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!("time[{index}].utc_offset: {} bytes", time.utc_offset.len()),
                    path: Some(format!("time[{index}].utc_offset")),
                    value: Some(format!("0x{}", hex::encode(&time.utc_offset))),
                    group: Some(format!("time[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!("time[{index}].identifier: {}", time.identifier),
                    path: Some(format!("time[{index}].identifier")),
                    value: Some(time.identifier.to_string()),
                    group: Some(format!("time[{index}]")),

                    is_group: false,
                });
            }
            scte35::SpliceDescriptor::Audio(audio) => {
                let index = audio_index;
                audio_index += 1;
                items.push(TreeItem {
                    label: format!("audio[{index}]"),
                    path: Some(format!("audio[{index}]")),
                    value: None,
                    group: Some("descriptors".to_string()),
                    is_group: true,
                });
                items.push(TreeItem {
                    label: format!(
                        "audio[{index}].components: {} bytes",
                        audio.audio_components.len()
                    ),
                    path: Some(format!("audio[{index}].components")),
                    value: Some(format!("0x{}", hex::encode(&audio.audio_components))),
                    group: Some(format!("audio[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!("audio[{index}].identifier: {}", audio.identifier),
                    path: Some(format!("audio[{index}].identifier")),
                    value: Some(audio.identifier.to_string()),
                    group: Some(format!("audio[{index}]")),

                    is_group: false,
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
                    label: format!("unknown[{index}]"),
                    path: Some(format!("unknown[{index}]")),
                    value: None,
                    group: Some("descriptors".to_string()),
                    is_group: true,
                });
                items.push(TreeItem {
                    label: format!("unknown[{index}].tag: {}", tag),
                    path: Some(format!("unknown[{index}].tag")),
                    value: Some(tag.to_string()),
                    group: Some(format!("unknown[{index}]")),

                    is_group: false,
                });
                items.push(TreeItem {
                    label: format!("unknown[{index}].data: {} bytes", data.len()),
                    path: Some(format!("unknown[{index}].data")),
                    value: Some(format!("0x{}", hex::encode(data))),
                    group: Some(format!("unknown[{index}]")),

                    is_group: false,
                });
            }
        }
    }

    items.push(TreeItem {
        label: "segmentation.add".to_string(),
        path: Some("segmentation.event_id".to_string()),
        value: Some("1".to_string()),
        group: None,
        is_group: false,
    });
    items.push(TreeItem {
        label: "avail.add".to_string(),
        path: Some("avail.provider_id".to_string()),
        value: Some("0x41424344".to_string()),
        group: None,
        is_group: false,
    });
    items.push(TreeItem {
        label: "dtmf.add".to_string(),
        path: Some("dtmf.preroll".to_string()),
        value: Some("10".to_string()),
        group: None,
        is_group: false,
    });
    items.push(TreeItem {
        label: "time.add".to_string(),
        path: Some("time.tai_seconds".to_string()),
        value: Some("0x000000000001".to_string()),
        group: None,
        is_group: false,
    });
    items.push(TreeItem {
        label: "audio.add".to_string(),
        path: Some("audio.components".to_string()),
        value: Some("0x1122".to_string()),
        group: None,
        is_group: false,
    });
    items.push(TreeItem {
        label: "unknown.add".to_string(),
        path: Some("unknown.tag".to_string()),
        value: Some("7".to_string()),
        group: None,
        is_group: false,
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
