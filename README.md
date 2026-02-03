# scte35-editor

[![CI](https://github.com/CMThF/scte35-editor/actions/workflows/ci.yml/badge.svg)](https://github.com/CMThF/scte35-editor/actions/workflows/ci.yml)
[![Release](https://github.com/CMThF/scte35-editor/actions/workflows/release.yml/badge.svg)](https://github.com/CMThF/scte35-editor/actions/workflows/release.yml)

`scte35-editor` is a pragmatic CLI (and interactive TUI) for creating, editing, validating, and inspecting SCTE-35 splice_info_section messages. It accepts JSON, base64, or hex inputs and outputs either a single format or a combined JSON blob with JSON + base64 + hex.

This repository targets:
- Fast iteration on SCTE-35 payloads for testing and integration work.
- Clear, consistent CLI workflows.
- An interactive tree editor for exploration and edits.

## Disclaimer

- Written with excessive use of GenAI Agents.

## Features

- Parse and validate SCTE-35 messages from JSON, base64, or hex.
- Apply changes using `--set path=value` with validation.
- Create new messages from templates.
- Delete descriptors and components by index.
- List supported patch paths and constraints.
- Interactive editor with tree navigation and edit/create/delete/write flows.

## Requirements

- Rust toolchain (`cargo`).
- System `pkg-config` and `fontconfig` development files for the TUI stack.
- `PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig` on Ubuntu when building or testing.

## CLI overview

Commands:
1. `show` - parse input and render output.
2. `validate` - parse input and return success feedback.
3. `edit` - parse input, apply `--set` changes, render output.
4. `new` - create a new message from a template, apply `--set` changes, render output.
5. `delete` - remove a descriptor or component by index, render output.
6. `list-paths` - list supported `--set` paths and their constraints.
7. `interactive` - run the TUI editor.

## Input handling

Inputs are positional by default. Use `--file` for file input.

- Positional input:
  - `show "<payload>"`
  - `edit "<payload>" --set ...`
  - `delete "<payload>" --target ...`
- File input:
  - `show --file ./trigger.txt`

`--input` is an explicit flag for argument input.

## Output formats

`--output-format` controls the output format:
- `json` (default)
- `hex`
- `base64`

When `--output-format json` is used, output is a **pretty-printed JSON object** with:
- `json` (object) - parsed JSON representation of the message.
- `hex` (string) - hex representation.
- `base64` (string) - base64 representation.

When `--output-format hex` or `--output-format base64` is used, output is only that format.

## Examples

### Show a trigger

```bash
scte35-editor show "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A=="
```

### Validate a trigger

```bash
scte35-editor validate "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A=="
```

Output:
```json
{
  "valid": true
}
```

### Edit a trigger

```bash
scte35-editor edit "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==" \
  --set table_id=252 \
  --set tier=4095 \
  --output-format json
```

### Create a new trigger

Templates:
- `time-signal`
- `splice-null`
- `splice-insert`
- `splice-schedule`

```bash
scte35-editor new time-signal \
  --set table_id=252 \
  --set tier=4095 \
  --output-format json
```

### Delete a descriptor or component

```bash
scte35-editor delete "/DAvAAAAAAAA///wBQb+dGKQoAAZAhdDVUVJSAAAjn+fCAgAAAAALKChijUCAKnMZ1g=" \
  --target segmentation[0] \
  --output-format json
```

### List supported paths

```bash
scte35-editor list-paths
```

## Patch path model

Edits and creates are driven by `--set path=value`. Paths are validated and include type constraints.

Examples:
- `table_id=252`
- `splice_command=splice_insert`
- `splice_insert.duration=90000`
- `segmentation[0].event_id=1`
- `avail[0].provider_id=0x41424344`
- `unknown[0].data=0x010203`

Use `list-paths` to get the full supported path list with types and constraints.

## Interactive mode

Run:
```bash
scte35-editor interactive
```

If no input is provided, the TUI starts in **template selection** mode. Select a template and press `C` to create it.

Controls:
- Arrow keys: navigate
- `E`: edit selected path
- `C`: create (for `.add` nodes or template selection)
- `D`: delete selected component/descriptor
- `W`: write output
- `Q` or `Esc`: quit

Write behavior:
- Output is written to the configured output target.
- If an output file is specified, the output is also printed to stdout.

## Output files

Use `--output-file` to write output to a file.

```bash
scte35-editor show "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==" \
  --output-file ./out.json
```

## Testing remote validation

There is a test that posts rendered triggers to the [Middleman SCTE parser](https://tools.middleman.tv/scte35-parser) endpoints for validation.
If those endpoints are unavailable, tests can fail due to network errors.
