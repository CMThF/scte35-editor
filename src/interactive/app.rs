use crate::core::DescriptorKind;
use crate::core::OutputFormat;
use crate::core::Scte35Document;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui_toolkit::{TreeNavigator, TreeNode, TreeViewState};
use scte35::SpliceCommand;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct InteractiveArgs {
    pub input: Option<String>,
    pub file: Option<PathBuf>,
    pub input_format: crate::io::InputFormat,
    pub output_format: OutputFormat,
    pub output_file: Option<PathBuf>,
    pub no_crc: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Browse,
    Edit,
    ConfirmWrite,
}

pub struct App {
    pub(crate) mode: Mode,
    pub(crate) document: Scte35Document,
    output_format: OutputFormat,
    output_file: Option<PathBuf>,
    pub(crate) input_buffer: String,
    pub(crate) status: String,
    pub(crate) tree_nodes: Vec<TreeNode<TreeItem>>,
    pub(crate) tree_state: TreeViewState,
    navigator: TreeNavigator,
    start_requires_template: bool,
    output_to_write: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TreeItem {
    pub label: String,
    pub path: Option<String>,
    pub value: Option<String>,
    pub delete_target: Option<String>,
}

impl App {
    fn new(
        document: Scte35Document,
        output_format: OutputFormat,
        output_file: Option<PathBuf>,
        start_requires_template: bool,
    ) -> Self {
        let tree_nodes = if start_requires_template {
            build_template_tree()
        } else {
            build_tree(&document)
        };
        let mut tree_state = TreeViewState::new();
        if !tree_nodes.is_empty() {
            tree_state.selected_path = Some(vec![0]);
        }
        Self {
            mode: Mode::Browse,
            document,
            output_format,
            output_file,
            input_buffer: String::new(),
            status: "Ready".to_string(),
            tree_nodes,
            tree_state,
            navigator: TreeNavigator::new(),
            start_requires_template,
            output_to_write: None,
        }
    }

    pub(crate) fn selected_item(&self) -> Option<&TreeItem> {
        let path = self.tree_state.selected_path.as_ref()?;
        let mut node: &TreeNode<TreeItem> = self.tree_nodes.get(*path.first()?)?;
        for index in path.iter().skip(1) {
            node = node.children.get(*index)?;
        }
        Some(&node.data)
    }
}

pub fn run_app(
    document: Scte35Document,
    output_format: OutputFormat,
    output_file: Option<PathBuf>,
    start_requires_template: bool,
) -> Result<(), String> {
    enable_raw_mode().map_err(|err| format!("failed to enable raw mode: {err}"))?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)
        .map_err(|err| format!("failed to enter alt screen: {err}"))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|err| format!("terminal init: {err}"))?;
    let mut app = App::new(
        document,
        output_format,
        output_file,
        start_requires_template,
    );
    let res = run_loop(&mut terminal, &mut app);
    disable_raw_mode().map_err(|err| format!("failed to disable raw mode: {err}"))?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )
    .map_err(|err| format!("failed to leave alt screen: {err}"))?;
    terminal.show_cursor().ok();
    if let Some(output) = app.output_to_write.take() {
        let target = crate::io::OutputTarget::from_path(app.output_file.as_deref());
        crate::io::write_output(target, &output)?;
        if app.output_file.is_some() {
            crate::io::write_output(crate::io::OutputTarget::Stdout, &output)?;
        }
    }
    res
}

fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<(), String> {
    let tick_rate = Duration::from_millis(50);
    let mut last_tick = Instant::now();
    loop {
        terminal
            .draw(|frame| crate::interactive::ui::draw(frame, app))
            .map_err(|err| format!("draw error: {err}"))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));
        if event::poll(timeout).map_err(|err| format!("poll error: {err}"))?
            && let Event::Key(key) = event::read().map_err(|err| format!("read error: {err}"))?
            && handle_key(app, key)?
        {
            break;
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool, String> {
    match app.mode {
        Mode::Browse => match key.code {
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                let _ = app
                    .navigator
                    .handle_key(key, &app.tree_nodes, &mut app.tree_state);
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                app.mode = Mode::Edit;
                app.input_buffer.clear();
                if let Some(item) = app.selected_item() {
                    if let Some(path) = item.path.as_deref() {
                        let value = item.value.as_deref().unwrap_or("");
                        app.input_buffer = format!("{path}={value}");
                        app.status = "Edit mode: update path=value and press Enter".to_string();
                    } else {
                        app.status = "Edit mode: enter path=value".to_string();
                    }
                } else {
                    app.status = "Edit mode: enter path=value".to_string();
                }
            }
            KeyCode::Char('w') | KeyCode::Char('W') => {
                app.mode = Mode::ConfirmWrite;
                app.status = "Write output? (y/n)".to_string();
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                let selection = app
                    .selected_item()
                    .and_then(|item| item.path.as_ref().map(|path| (item, path.clone())));
                if let Some((item, path)) = selection {
                    let value = item.value.clone().unwrap_or_default();
                    let mut create_path = path.clone();
                    if create_path.starts_with("template.") {
                        let (document, label) = match create_path.as_str() {
                            "template.time_signal" => {
                                (Scte35Document::new_default_time_signal()?, "time_signal")
                            }
                            "template.splice_null" => {
                                (Scte35Document::new_default_splice_null()?, "splice_null")
                            }
                            "template.splice_insert" => (
                                Scte35Document::new_default_splice_insert()?,
                                "splice_insert",
                            ),
                            "template.splice_schedule" => (
                                Scte35Document::new_default_splice_schedule()?,
                                "splice_schedule",
                            ),
                            _ => {
                                app.status = "Unknown template selection".to_string();
                                return Ok(false);
                            }
                        };
                        app.document = document;
                        app.tree_nodes = build_tree(&app.document);
                        app.start_requires_template = false;
                        app.status = format!("Template selected: {label}");
                        return Ok(false);
                    }
                    if item.label.ends_with(".add")
                        && let Some(indexed) = expand_add_path(&app.document, &path)
                    {
                        create_path = indexed;
                    }
                    let changes = vec![(create_path.clone(), value)];
                    match app.document.apply_sets(&changes) {
                        Ok(()) => {
                            app.status = format!("Created via {create_path}");
                            app.tree_nodes = build_tree(&app.document);
                        }
                        Err(err) => {
                            app.status = err;
                        }
                    }
                } else {
                    app.status = "Create requires a node with a patch path".to_string();
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                let target = app
                    .selected_item()
                    .and_then(|item| item.delete_target.clone());
                if let Some(target) = target {
                    match app.document.delete_target(&target) {
                        Ok(()) => {
                            app.status = format!("Deleted {target}");
                            app.tree_nodes = build_tree(&app.document);
                        }
                        Err(err) => {
                            app.status = err;
                        }
                    }
                } else {
                    app.status = "Delete requires a removable node".to_string();
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => return Ok(true),
            _ => {}
        },
        Mode::Edit => match key.code {
            KeyCode::Esc => {
                app.mode = Mode::Browse;
                app.input_buffer.clear();
                app.status = "Edit canceled".to_string();
            }
            KeyCode::Enter => {
                let input = app.input_buffer.trim();
                if let Some((path, value)) = input.split_once('=') {
                    let changes = vec![(path.trim().to_string(), value.trim().to_string())];
                    match app.document.apply_sets(&changes) {
                        Ok(()) => {
                            app.status = "Applied".to_string();
                            app.tree_nodes = build_tree(&app.document);
                        }
                        Err(err) => {
                            app.status = err;
                        }
                    }
                } else {
                    app.status = "Expected path=value".to_string();
                }
                app.input_buffer.clear();
                app.mode = Mode::Browse;
            }
            KeyCode::Backspace => {
                app.input_buffer.pop();
            }
            KeyCode::Char(ch) => {
                app.input_buffer.push(ch);
            }
            _ => {}
        },
        Mode::ConfirmWrite => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let output = render_output(&app.document, app.output_format)?;
                app.output_to_write = Some(output);
                app.status = "Written".to_string();
                return Ok(true);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.mode = Mode::Browse;
                app.status = "Write canceled".to_string();
            }
            _ => {}
        },
    }
    Ok(false)
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
        .filter(|descriptor| {
            matches!(
                (descriptor, kind),
                (
                    scte35::SpliceDescriptor::Segmentation(_),
                    DescriptorKind::Segmentation
                ) | (scte35::SpliceDescriptor::Avail(_), DescriptorKind::Avail)
                    | (scte35::SpliceDescriptor::Dtmf(_), DescriptorKind::Dtmf)
                    | (scte35::SpliceDescriptor::Time(_), DescriptorKind::Time)
                    | (scte35::SpliceDescriptor::Audio(_), DescriptorKind::Audio)
                    | (
                        scte35::SpliceDescriptor::Unknown { .. },
                        DescriptorKind::Unknown
                    )
            )
        })
        .count()
}

fn build_tree(doc: &Scte35Document) -> Vec<TreeNode<TreeItem>> {
    let section = doc.section();
    let mut nodes = Vec::new();

    nodes.push(TreeNode::new(TreeItem {
        label: format!("table_id: {}", section.table_id),
        path: Some("table_id".to_string()),
        value: Some(section.table_id.to_string()),
        delete_target: None,
    }));
    nodes.push(TreeNode::new(TreeItem {
        label: format!("pts_adjustment: {}", section.pts_adjustment),
        path: Some("pts_adjustment".to_string()),
        value: Some(section.pts_adjustment.to_string()),
        delete_target: None,
    }));
    nodes.push(TreeNode::new(TreeItem {
        label: format!("tier: {}", section.tier),
        path: Some("tier".to_string()),
        value: Some(section.tier.to_string()),
        delete_target: None,
    }));
    nodes.push(TreeNode::new(TreeItem {
        label: format!("cw_index: {}", section.cw_index),
        path: Some("cw_index".to_string()),
        value: Some(section.cw_index.to_string()),
        delete_target: None,
    }));

    let (splice_command_label, splice_command_children) = match &section.splice_command {
        SpliceCommand::SpliceNull => ("splice_command: splice_null".to_string(), Vec::new()),
        SpliceCommand::BandwidthReservation(reservation) => {
            let children = vec![TreeNode::new(TreeItem {
                label: format!("dwbw_reservation: {}", reservation.dwbw_reservation),
                path: None,
                value: None,
                delete_target: None,
            })];
            (
                "splice_command: bandwidth_reservation".to_string(),
                children,
            )
        }
        SpliceCommand::PrivateCommand(command) => {
            let children = vec![
                TreeNode::new(TreeItem {
                    label: format!("private_command_id: {}", command.private_command_id),
                    path: None,
                    value: None,
                    delete_target: None,
                }),
                TreeNode::new(TreeItem {
                    label: format!("private_command_length: {}", command.private_command_length),
                    path: None,
                    value: None,
                    delete_target: None,
                }),
                TreeNode::new(TreeItem {
                    label: format!("private_bytes: {} bytes", command.private_bytes.len()),
                    path: None,
                    value: None,
                    delete_target: None,
                }),
            ];
            ("splice_command: private_command".to_string(), children)
        }
        SpliceCommand::Unknown => ("splice_command: unknown".to_string(), Vec::new()),
        SpliceCommand::TimeSignal(time_signal) => {
            let mut children = Vec::new();
            let pts_value = time_signal.splice_time.pts_time.unwrap_or(0);
            let immediate_value = time_signal.splice_time.pts_time.is_none();
            children.push(TreeNode::new(TreeItem {
                label: format!("splice_time.pts_time: {}", pts_value),
                path: Some("splice_time.pts_time".to_string()),
                value: Some(pts_value.to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: format!("splice_time.immediate: {}", immediate_value),
                path: Some("splice_time.immediate".to_string()),
                value: Some(immediate_value.to_string()),
                delete_target: None,
            }));
            ("splice_command: time_signal".to_string(), children)
        }
        SpliceCommand::SpliceInsert(insert) => {
            let mut children = Vec::new();
            children.push(TreeNode::new(TreeItem {
                label: format!("splice_event_id: {}", insert.splice_event_id),
                path: Some("splice_insert.splice_event_id".to_string()),
                value: Some(insert.splice_event_id.to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: format!("cancel: {}", insert.splice_event_cancel_indicator != 0),
                path: Some("splice_insert.cancel".to_string()),
                value: Some((insert.splice_event_cancel_indicator != 0).to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: format!("out_of_network: {}", insert.out_of_network_indicator != 0),
                path: Some("splice_insert.out_of_network".to_string()),
                value: Some((insert.out_of_network_indicator != 0).to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: format!("program_splice: {}", insert.program_splice_flag != 0),
                path: Some("splice_insert.program_splice".to_string()),
                value: Some((insert.program_splice_flag != 0).to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: format!("splice_immediate: {}", insert.splice_immediate_flag != 0),
                path: Some("splice_insert.splice_immediate".to_string()),
                value: Some((insert.splice_immediate_flag != 0).to_string()),
                delete_target: None,
            }));
            let pts_time = insert
                .splice_time
                .as_ref()
                .and_then(|time| time.pts_time)
                .unwrap_or(0);
            let splice_time_immediate = insert.splice_time.is_none();
            children.push(TreeNode::new(TreeItem {
                label: format!("splice_time.pts_time: {}", pts_time),
                path: Some("splice_insert.splice_time.pts_time".to_string()),
                value: Some(pts_time.to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: format!("splice_time.immediate: {}", splice_time_immediate),
                path: Some("splice_insert.splice_time.immediate".to_string()),
                value: Some(splice_time_immediate.to_string()),
                delete_target: None,
            }));
            let duration_value = insert
                .break_duration
                .as_ref()
                .map(|d| d.duration)
                .unwrap_or(0);
            let auto_return_value = insert
                .break_duration
                .as_ref()
                .map(|d| d.auto_return != 0)
                .unwrap_or(false);
            children.push(TreeNode::new(TreeItem {
                label: format!("duration: {}", duration_value),
                path: Some("splice_insert.duration".to_string()),
                value: Some(duration_value.to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: "duration.clear".to_string(),
                path: Some("splice_insert.duration.clear".to_string()),
                value: Some("true".to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: format!("auto_return: {}", auto_return_value),
                path: Some("splice_insert.auto_return".to_string()),
                value: Some(auto_return_value.to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: format!("unique_program_id: {}", insert.unique_program_id),
                path: Some("splice_insert.unique_program_id".to_string()),
                value: Some(insert.unique_program_id.to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: format!("avail_num: {}", insert.avail_num),
                path: Some("splice_insert.avail_num".to_string()),
                value: Some(insert.avail_num.to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: format!("avails_expected: {}", insert.avails_expected),
                path: Some("splice_insert.avails_expected".to_string()),
                value: Some(insert.avails_expected.to_string()),
                delete_target: None,
            }));

            let mut component_children = Vec::new();
            for (index, component) in insert.components.iter().enumerate() {
                let mut fields = Vec::new();
                fields.push(TreeNode::new(TreeItem {
                    label: format!("tag: {}", component.component_tag),
                    path: Some(format!("splice_insert.component[{index}].tag")),
                    value: Some(component.component_tag.to_string()),
                    delete_target: None,
                }));
                let pts_time = component
                    .splice_time
                    .as_ref()
                    .and_then(|time| time.pts_time)
                    .unwrap_or(0);
                let component_immediate = component.splice_time.is_none();
                fields.push(TreeNode::new(TreeItem {
                    label: format!("pts_time: {}", pts_time),
                    path: Some(format!("splice_insert.component[{index}].pts_time")),
                    value: Some(pts_time.to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!("immediate: {}", component_immediate),
                    path: Some(format!("splice_insert.component[{index}].immediate")),
                    value: Some(component_immediate.to_string()),
                    delete_target: None,
                }));
                component_children.push(TreeNode::with_children(
                    TreeItem {
                        label: format!("component[{index}]"),
                        path: None,
                        value: None,
                        delete_target: Some(format!("splice_insert.component[{index}]")),
                    },
                    fields,
                ));
            }
            children.push(TreeNode::new(TreeItem {
                label: "component.add".to_string(),
                path: Some("splice_insert.component.add".to_string()),
                value: Some("tag=1,pts=90000".to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: "component.clear".to_string(),
                path: Some("splice_insert.component.clear".to_string()),
                value: Some("true".to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::with_children(
                TreeItem {
                    label: format!("components: {}", insert.components.len()),
                    path: None,
                    value: None,
                    delete_target: None,
                },
                component_children,
            ));
            ("splice_command: splice_insert".to_string(), children)
        }
        SpliceCommand::SpliceSchedule(schedule) => {
            let mut children = Vec::new();
            children.push(TreeNode::new(TreeItem {
                label: format!("splice_event_id: {}", schedule.splice_event_id),
                path: Some("splice_schedule.splice_event_id".to_string()),
                value: Some(schedule.splice_event_id.to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: format!("cancel: {}", schedule.splice_event_cancel_indicator != 0),
                path: Some("splice_schedule.cancel".to_string()),
                value: Some((schedule.splice_event_cancel_indicator != 0).to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: format!("out_of_network: {}", schedule.out_of_network_indicator != 0),
                path: Some("splice_schedule.out_of_network".to_string()),
                value: Some((schedule.out_of_network_indicator != 0).to_string()),
                delete_target: None,
            }));
            let utc_value = schedule.utc_splice_time.unwrap_or(0);
            children.push(TreeNode::new(TreeItem {
                label: format!("utc_splice_time: {}", utc_value),
                path: Some("splice_schedule.utc_splice_time".to_string()),
                value: Some(utc_value.to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: "utc_splice_time.clear".to_string(),
                path: Some("splice_schedule.utc_splice_time.clear".to_string()),
                value: Some("true".to_string()),
                delete_target: None,
            }));
            let duration_value = schedule.splice_duration.unwrap_or(0);
            children.push(TreeNode::new(TreeItem {
                label: format!("duration: {}", duration_value),
                path: Some("splice_schedule.duration".to_string()),
                value: Some(duration_value.to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: "duration.clear".to_string(),
                path: Some("splice_schedule.duration.clear".to_string()),
                value: Some("true".to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: format!("unique_program_id: {}", schedule.unique_program_id),
                path: Some("splice_schedule.unique_program_id".to_string()),
                value: Some(schedule.unique_program_id.to_string()),
                delete_target: None,
            }));

            let mut component_children = Vec::new();
            for (index, component) in schedule.component_list.iter().enumerate() {
                let mut fields = Vec::new();
                fields.push(TreeNode::new(TreeItem {
                    label: format!("tag: {}", component.component_tag),
                    path: Some(format!("splice_schedule.component[{index}].tag")),
                    value: Some(component.component_tag.to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!("splice_mode: {}", component.splice_mode_indicator),
                    path: Some(format!("splice_schedule.component[{index}].splice_mode")),
                    value: Some(component.splice_mode_indicator.to_string()),
                    delete_target: None,
                }));
                let duration_value = component.splice_duration.unwrap_or(0);
                fields.push(TreeNode::new(TreeItem {
                    label: format!("duration: {}", duration_value),
                    path: Some(format!("splice_schedule.component[{index}].duration")),
                    value: Some(duration_value.to_string()),
                    delete_target: None,
                }));
                let utc_value = component.utc_splice_time.unwrap_or(0);
                fields.push(TreeNode::new(TreeItem {
                    label: format!("utc_splice_time: {}", utc_value),
                    path: Some(format!(
                        "splice_schedule.component[{index}].utc_splice_time"
                    )),
                    value: Some(utc_value.to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!("duration_flag: {}", component.duration_flag != 0),
                    path: Some(format!("splice_schedule.component[{index}].duration_flag")),
                    value: Some((component.duration_flag != 0).to_string()),
                    delete_target: None,
                }));
                component_children.push(TreeNode::with_children(
                    TreeItem {
                        label: format!("component[{index}]"),
                        path: None,
                        value: None,
                        delete_target: Some(format!("splice_schedule.component[{index}]")),
                    },
                    fields,
                ));
            }
            children.push(TreeNode::new(TreeItem {
                label: "component.add".to_string(),
                path: Some("splice_schedule.component.add".to_string()),
                value: Some("tag=1,splice_mode=0,duration=120".to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::new(TreeItem {
                label: "component.clear".to_string(),
                path: Some("splice_schedule.component.clear".to_string()),
                value: Some("true".to_string()),
                delete_target: None,
            }));
            children.push(TreeNode::with_children(
                TreeItem {
                    label: format!("components: {}", schedule.component_list.len()),
                    path: None,
                    value: None,
                    delete_target: None,
                },
                component_children,
            ));
            ("splice_command: splice_schedule".to_string(), children)
        }
    };

    nodes.push(TreeNode::with_children(
        TreeItem {
            label: splice_command_label,
            path: Some("splice_command".to_string()),
            value: Some(splice_command_value(&section.splice_command)),
            delete_target: None,
        },
        splice_command_children,
    ));

    nodes.push(TreeNode::new(TreeItem {
        label: "splice_command.set".to_string(),
        path: Some("splice_command".to_string()),
        value: Some(splice_command_value(&section.splice_command)),
        delete_target: None,
    }));

    let mut descriptor_children = Vec::new();
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
                let mut fields = Vec::new();
                fields.push(TreeNode::new(TreeItem {
                    label: format!("event_id: {}", seg.segmentation_event_id),
                    path: Some(format!("segmentation[{index}].event_id")),
                    value: Some(seg.segmentation_event_id.to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!("cancel: {}", seg.segmentation_event_cancel_indicator),
                    path: Some(format!("segmentation[{index}].cancel")),
                    value: Some(seg.segmentation_event_cancel_indicator.to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!("program: {}", seg.program_segmentation_flag),
                    path: Some(format!("segmentation[{index}].program")),
                    value: Some(seg.program_segmentation_flag.to_string()),
                    delete_target: None,
                }));
                let duration_value = seg.segmentation_duration.unwrap_or(0);
                fields.push(TreeNode::new(TreeItem {
                    label: format!("duration: {}", duration_value),
                    path: Some(format!("segmentation[{index}].duration")),
                    value: Some(duration_value.to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: "duration.clear".to_string(),
                    path: Some(format!("segmentation[{index}].duration.clear")),
                    value: Some("true".to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!(
                        "delivery_not_restricted: {}",
                        seg.delivery_not_restricted_flag
                    ),
                    path: Some(format!("segmentation[{index}].delivery_not_restricted")),
                    value: Some(seg.delivery_not_restricted_flag.to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!(
                        "web_delivery_allowed: {}",
                        seg.web_delivery_allowed_flag.unwrap_or(false)
                    ),
                    path: Some(format!("segmentation[{index}].web_delivery_allowed")),
                    value: Some(seg.web_delivery_allowed_flag.unwrap_or(false).to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!(
                        "no_regional_blackout: {}",
                        seg.no_regional_blackout_flag.unwrap_or(false)
                    ),
                    path: Some(format!("segmentation[{index}].no_regional_blackout")),
                    value: Some(seg.no_regional_blackout_flag.unwrap_or(false).to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!(
                        "archive_allowed: {}",
                        seg.archive_allowed_flag.unwrap_or(false)
                    ),
                    path: Some(format!("segmentation[{index}].archive_allowed")),
                    value: Some(seg.archive_allowed_flag.unwrap_or(false).to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!(
                        "device_restrictions: {}",
                        seg.device_restrictions.unwrap_or(0)
                    ),
                    path: Some(format!("segmentation[{index}].device_restrictions")),
                    value: Some(seg.device_restrictions.unwrap_or(0).to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!("upid_type: {}", u8::from(seg.segmentation_upid_type)),
                    path: Some(format!("segmentation[{index}].upid_type")),
                    value: Some(u8::from(seg.segmentation_upid_type).to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!("upid: {} bytes", seg.segmentation_upid.len()),
                    path: Some(format!("segmentation[{index}].upid")),
                    value: Some(format!("0x{}", hex::encode(&seg.segmentation_upid))),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!("type_id: {}", seg.segmentation_type_id),
                    path: Some(format!("segmentation[{index}].type_id")),
                    value: Some(seg.segmentation_type_id.to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!("segment_num: {}", seg.segment_num),
                    path: Some(format!("segmentation[{index}].segment_num")),
                    value: Some(seg.segment_num.to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!("segments_expected: {}", seg.segments_expected),
                    path: Some(format!("segmentation[{index}].segments_expected")),
                    value: Some(seg.segments_expected.to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!("sub_segment_num: {}", seg.sub_segment_num.unwrap_or(0)),
                    path: Some(format!("segmentation[{index}].sub_segment_num")),
                    value: Some(seg.sub_segment_num.unwrap_or(0).to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: format!(
                        "sub_segments_expected: {}",
                        seg.sub_segments_expected.unwrap_or(0)
                    ),
                    path: Some(format!("segmentation[{index}].sub_segments_expected")),
                    value: Some(seg.sub_segments_expected.unwrap_or(0).to_string()),
                    delete_target: None,
                }));
                fields.push(TreeNode::new(TreeItem {
                    label: "sub_segment.clear".to_string(),
                    path: Some(format!("segmentation[{index}].sub_segment.clear")),
                    value: Some("true".to_string()),
                    delete_target: None,
                }));
                descriptor_children.push(TreeNode::with_children(
                    TreeItem {
                        label: format!("segmentation[{index}]"),
                        path: None,
                        value: None,
                        delete_target: Some(format!("segmentation[{index}]")),
                    },
                    fields,
                ));
            }
            scte35::SpliceDescriptor::Avail(avail) => {
                let index = avail_index;
                avail_index += 1;
                let fields = vec![
                    TreeNode::new(TreeItem {
                        label: format!("provider_id: {} bytes", avail.provider_avail_id.len()),
                        path: Some(format!("avail[{index}].provider_id")),
                        value: Some(format!("0x{}", hex::encode(&avail.provider_avail_id))),
                        delete_target: None,
                    }),
                    TreeNode::new(TreeItem {
                        label: format!("identifier: {}", avail.identifier),
                        path: Some(format!("avail[{index}].identifier")),
                        value: Some(avail.identifier.to_string()),
                        delete_target: None,
                    }),
                ];
                descriptor_children.push(TreeNode::with_children(
                    TreeItem {
                        label: format!("avail[{index}]"),
                        path: None,
                        value: None,
                        delete_target: Some(format!("avail[{index}]")),
                    },
                    fields,
                ));
            }
            scte35::SpliceDescriptor::Dtmf(dtmf) => {
                let index = dtmf_index;
                dtmf_index += 1;
                let fields = vec![
                    TreeNode::new(TreeItem {
                        label: format!("preroll: {}", dtmf.preroll),
                        path: Some(format!("dtmf[{index}].preroll")),
                        value: Some(dtmf.preroll.to_string()),
                        delete_target: None,
                    }),
                    TreeNode::new(TreeItem {
                        label: format!("chars: {} bytes", dtmf.dtmf_chars.len()),
                        path: Some(format!("dtmf[{index}].chars")),
                        value: Some(format!("0x{}", hex::encode(&dtmf.dtmf_chars))),
                        delete_target: None,
                    }),
                    TreeNode::new(TreeItem {
                        label: format!("identifier: {}", dtmf.identifier),
                        path: Some(format!("dtmf[{index}].identifier")),
                        value: Some(dtmf.identifier.to_string()),
                        delete_target: None,
                    }),
                ];
                descriptor_children.push(TreeNode::with_children(
                    TreeItem {
                        label: format!("dtmf[{index}]"),
                        path: None,
                        value: None,
                        delete_target: Some(format!("dtmf[{index}]")),
                    },
                    fields,
                ));
            }
            scte35::SpliceDescriptor::Time(time) => {
                let index = time_index;
                time_index += 1;
                let fields = vec![
                    TreeNode::new(TreeItem {
                        label: format!("tai_seconds: {} bytes", time.tai_seconds.len()),
                        path: Some(format!("time[{index}].tai_seconds")),
                        value: Some(format!("0x{}", hex::encode(&time.tai_seconds))),
                        delete_target: None,
                    }),
                    TreeNode::new(TreeItem {
                        label: format!("tai_ns: {} bytes", time.tai_ns.len()),
                        path: Some(format!("time[{index}].tai_ns")),
                        value: Some(format!("0x{}", hex::encode(&time.tai_ns))),
                        delete_target: None,
                    }),
                    TreeNode::new(TreeItem {
                        label: format!("utc_offset: {} bytes", time.utc_offset.len()),
                        path: Some(format!("time[{index}].utc_offset")),
                        value: Some(format!("0x{}", hex::encode(&time.utc_offset))),
                        delete_target: None,
                    }),
                    TreeNode::new(TreeItem {
                        label: format!("identifier: {}", time.identifier),
                        path: Some(format!("time[{index}].identifier")),
                        value: Some(time.identifier.to_string()),
                        delete_target: None,
                    }),
                ];
                descriptor_children.push(TreeNode::with_children(
                    TreeItem {
                        label: format!("time[{index}]"),
                        path: None,
                        value: None,
                        delete_target: Some(format!("time[{index}]")),
                    },
                    fields,
                ));
            }
            scte35::SpliceDescriptor::Audio(audio) => {
                let index = audio_index;
                audio_index += 1;
                let fields = vec![
                    TreeNode::new(TreeItem {
                        label: format!("components: {} bytes", audio.audio_components.len()),
                        path: Some(format!("audio[{index}].components")),
                        value: Some(format!("0x{}", hex::encode(&audio.audio_components))),
                        delete_target: None,
                    }),
                    TreeNode::new(TreeItem {
                        label: format!("identifier: {}", audio.identifier),
                        path: Some(format!("audio[{index}].identifier")),
                        value: Some(audio.identifier.to_string()),
                        delete_target: None,
                    }),
                ];
                descriptor_children.push(TreeNode::with_children(
                    TreeItem {
                        label: format!("audio[{index}]"),
                        path: None,
                        value: None,
                        delete_target: Some(format!("audio[{index}]")),
                    },
                    fields,
                ));
            }
            scte35::SpliceDescriptor::Unknown { tag, length, data } => {
                let index = unknown_index;
                unknown_index += 1;
                let fields = vec![
                    TreeNode::new(TreeItem {
                        label: format!("tag: {}", tag),
                        path: Some(format!("unknown[{index}].tag")),
                        value: Some(tag.to_string()),
                        delete_target: None,
                    }),
                    TreeNode::new(TreeItem {
                        label: format!("length: {}", length),
                        path: None,
                        value: None,
                        delete_target: None,
                    }),
                    TreeNode::new(TreeItem {
                        label: format!("data: {} bytes", data.len()),
                        path: Some(format!("unknown[{index}].data")),
                        value: Some(format!("0x{}", hex::encode(data))),
                        delete_target: None,
                    }),
                ];
                descriptor_children.push(TreeNode::with_children(
                    TreeItem {
                        label: format!("unknown[{index}]"),
                        path: None,
                        value: None,
                        delete_target: Some(format!("unknown[{index}]")),
                    },
                    fields,
                ));
            }
        }
    }

    descriptor_children.push(TreeNode::new(TreeItem {
        label: "segmentation.add".to_string(),
        path: Some("segmentation.event_id".to_string()),
        value: Some("1".to_string()),
        delete_target: None,
    }));
    descriptor_children.push(TreeNode::new(TreeItem {
        label: "avail.add".to_string(),
        path: Some("avail.provider_id".to_string()),
        value: Some("0x41424344".to_string()),
        delete_target: None,
    }));
    descriptor_children.push(TreeNode::new(TreeItem {
        label: "dtmf.add".to_string(),
        path: Some("dtmf.preroll".to_string()),
        value: Some("10".to_string()),
        delete_target: None,
    }));
    descriptor_children.push(TreeNode::new(TreeItem {
        label: "time.add".to_string(),
        path: Some("time.tai_seconds".to_string()),
        value: Some("0x000000000001".to_string()),
        delete_target: None,
    }));
    descriptor_children.push(TreeNode::new(TreeItem {
        label: "audio.add".to_string(),
        path: Some("audio.components".to_string()),
        value: Some("0x1122".to_string()),
        delete_target: None,
    }));
    descriptor_children.push(TreeNode::new(TreeItem {
        label: "unknown.add".to_string(),
        path: Some("unknown.tag".to_string()),
        value: Some("7".to_string()),
        delete_target: None,
    }));

    nodes.push(TreeNode::with_children(
        TreeItem {
            label: format!("descriptors: {}", section.splice_descriptors.len()),
            path: None,
            value: None,
            delete_target: None,
        },
        descriptor_children,
    ));

    nodes
}

fn render_output(document: &Scte35Document, output_format: OutputFormat) -> Result<String, String> {
    match output_format {
        OutputFormat::Json => {
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
        OutputFormat::Hex => document.render(OutputFormat::Hex),
        OutputFormat::Base64 => document.render(OutputFormat::Base64),
    }
}

fn build_template_tree() -> Vec<TreeNode<TreeItem>> {
    vec![TreeNode::with_children(
        TreeItem {
            label: "Choose template (press C)".to_string(),
            path: None,
            value: None,
            delete_target: None,
        },
        vec![
            TreeNode::new(TreeItem {
                label: "time_signal".to_string(),
                path: Some("template.time_signal".to_string()),
                value: None,
                delete_target: None,
            }),
            TreeNode::new(TreeItem {
                label: "splice_null".to_string(),
                path: Some("template.splice_null".to_string()),
                value: None,
                delete_target: None,
            }),
            TreeNode::new(TreeItem {
                label: "splice_insert".to_string(),
                path: Some("template.splice_insert".to_string()),
                value: None,
                delete_target: None,
            }),
            TreeNode::new(TreeItem {
                label: "splice_schedule".to_string(),
                path: Some("template.splice_schedule".to_string()),
                value: None,
                delete_target: None,
            }),
        ],
    )]
}

fn splice_command_value(command: &SpliceCommand) -> String {
    match command {
        SpliceCommand::SpliceNull => "splice_null".to_string(),
        SpliceCommand::SpliceInsert(_) => "splice_insert".to_string(),
        SpliceCommand::TimeSignal(_) => "time_signal".to_string(),
        SpliceCommand::SpliceSchedule(_) => "splice_schedule".to_string(),
        SpliceCommand::BandwidthReservation(_) => "bandwidth_reservation".to_string(),
        SpliceCommand::PrivateCommand(_) => "private_command".to_string(),
        SpliceCommand::Unknown => "unknown".to_string(),
    }
}
