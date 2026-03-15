# Plugin CLI

Plugin-focused scaffolding, build, smoke-test, and diagnostics commands for Volt backend plugins.

## `volt plugin init`

Create a new backend plugin project scaffold.

```bash
volt plugin init my-plugin
```

Creates a `volt-plugin.json`, `src/plugin.ts`, `package.json`, and `tsconfig.json` scaffold that can be bundled immediately with `volt plugin build`.

## `volt plugin build`

Bundle the plugin backend entry declared by `volt-plugin.json`.

```bash
volt plugin build
```

Behavior:
1. Loads and validates `volt-plugin.json`
2. Resolves the plugin source entry (`src/plugin.ts`, `src/plugin.js`, `plugin.ts`, or `plugin.js`)
3. Bundles the backend to the configured manifest output, typically `dist/plugin.js`
4. Treats `volt:*` imports as external runtime modules

## `volt plugin test`

Run a smoke test against the real `volt-plugin-host` binary.

```bash
volt plugin test
```

Behavior:
1. Builds the plugin bundle
2. Starts the real plugin host process with the plugin loaded
3. Sends `activate`
4. Invokes each command listed in `contributes.commands`
5. Sends `deactivate` and tears down the host cleanly

## `volt plugin doctor`

Validate plugin schema and compatibility with the nearest Volt app, when present.

```bash
volt plugin doctor
```

Checks:
1. Manifest presence, JSON parsing, and schema validity
2. Plugin source entry existence and extension support
3. Bundle output path validity and current build presence
4. `apiVersion` compatibility with the current Volt CLI/runtime
5. `engine.volt` semver compatibility
6. Parent app permission and grant compatibility, when a Volt app config is found
