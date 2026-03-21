# Custom Model Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow users to import custom `.gguf` model files from disk via a native file picker in Settings, with models referenced by path (no copy).

**Architecture:** Custom models are stored as an array of `{id, name, path}` entries in the existing Tauri settings store. A new Tauri command opens the native file dialog and returns the entry. The frontend persists the array and shows imported models alongside downloaded registry models in the same radio list. Backend commands (`activate_model`, `load_local_model`) branch on the `custom-` ID prefix to resolve paths from the store instead of the static registry. Note: `correct_text` does not need changes — it delegates to `LocalProvider` which uses the globally loaded model via `is_model_loaded()`, so it works with any model loaded by `activate_model` or `load_local_model`.

**Tech Stack:** Rust (Tauri 2, tauri-plugin-dialog, uuid), TypeScript (SvelteKit 5, Svelte 5 runes), Tailwind CSS 4

**Spec:** `docs/superpowers/specs/2026-03-21-custom-model-import-design.md`

---

## File Structure

| File | Responsibility | Action |
|------|---------------|--------|
| `src-tauri/Cargo.toml` | Rust dependencies | Modify: add `tauri-plugin-dialog` and `uuid` |
| `src-tauri/capabilities/default.json` | Tauri permissions | Modify: add `dialog:allow-open` |
| `src-tauri/src/lib.rs` | App setup, model loading at startup | Modify: init dialog plugin, handle custom models in `load_local_model` |
| `src-tauri/src/commands.rs` | Tauri command handlers | Modify: add `import_custom_model`, `validate_custom_models`, add `CustomModelEntry` struct, add `get_custom_model_path` helper, update `activate_model` |
| `src/lib/types/models.ts` | TypeScript type definitions | Modify: add `CustomModel` interface |
| `src/lib/stores/settings.svelte.ts` | Reactive settings store | Modify: add `customModels` state, `importCustomModel`, `removeCustomModel`, `validateCustomModels` methods, getter |
| `src/lib/components/Settings.svelte` | Settings panel UI | Modify: add import section, show custom models in radio list |

---

### Task 1: Add Rust dependencies and Tauri permissions

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src-tauri/src/lib.rs:54-58` (plugin chain)

- [ ] **Step 1: Add `tauri-plugin-dialog` and `uuid` to Cargo.toml**

In `src-tauri/Cargo.toml`, add to `[dependencies]`:

```toml
tauri-plugin-dialog = "2"
uuid = { version = "1", features = ["v4"] }
```

- [ ] **Step 2: Add dialog permission to capabilities**

In `src-tauri/capabilities/default.json`, add `"dialog:allow-open"` to the `permissions` array:

```json
{
	"$schema": "../gen/schemas/desktop-schema.json",
	"identifier": "default",
	"description": "Default permissions for Jolly",
	"windows": ["main"],
	"permissions": [
		"core:default",
		"opener:default",
		"clipboard-manager:allow-read-text",
		"clipboard-manager:allow-write-text",
		"store:default",
		"keyring:default",
		"dialog:allow-open"
	]
}
```

- [ ] **Step 3: Initialize dialog plugin in lib.rs**

In `src-tauri/src/lib.rs`, add `.plugin(tauri_plugin_dialog::init())` to the builder chain, after the keyring plugin:

```rust
.plugin(tauri_plugin_keyring::init())
.plugin(tauri_plugin_dialog::init())
```

- [ ] **Step 4: Verify it compiles**

Run: `cd src-tauri && cargo check 2>&1 | tail -5`
Expected: compilation succeeds (warnings OK)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/capabilities/default.json src-tauri/src/lib.rs
git commit -m "feat: add tauri-plugin-dialog and uuid dependencies for custom model import"
```

---

### Task 2: Add `CustomModelEntry` struct and new Tauri commands

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Add imports and `CustomModelEntry` struct**

At the top of `src-tauri/src/commands.rs`, add `use uuid::Uuid;` to the imports. Then add the struct after the existing `ModelWithState` struct (after line 124):

```rust
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomModelEntry {
    pub id: String,
    pub name: String,
    pub path: String,
}
```

- [ ] **Step 2: Add `import_custom_model` command**

Add after the `delete_model` command (after line 241):

```rust
#[tauri::command]
pub async fn import_custom_model(app: AppHandle) -> Result<Option<CustomModelEntry>, String> {
    use tauri_plugin_dialog::DialogExt;

    let file_path = app
        .dialog()
        .file()
        .add_filter("GGUF Models", &["gguf"])
        .pick_file()
        .await;

    let file_path = match file_path {
        Some(f) => f.path,
        None => return Ok(None), // User cancelled
    };

    // Server-side extension validation (dialog filters can be bypassed on some Linux DEs)
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !ext.eq_ignore_ascii_case("gguf") {
        return Err("Selected file is not a .gguf model".to_string());
    }

    let name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown Model")
        .to_string();

    let id = format!("custom-{}", Uuid::new_v4());

    Ok(Some(CustomModelEntry {
        id,
        name,
        path: file_path.to_string_lossy().to_string(),
    }))
}
```

- [ ] **Step 3: Add `validate_custom_models` command**

Add after `import_custom_model`:

```rust
#[tauri::command]
pub async fn validate_custom_models(paths: Vec<String>) -> Result<Vec<String>, String> {
    Ok(paths
        .into_iter()
        .filter(|p| std::path::Path::new(p).exists())
        .collect())
}
```

- [ ] **Step 4: Register new commands in lib.rs**

In `src-tauri/src/lib.rs`, add the new commands to the `invoke_handler` macro:

```rust
.invoke_handler(tauri::generate_handler![
    commands::correct_text,
    commands::list_available_models,
    commands::list_downloaded_models,
    commands::start_download,
    commands::cancel_download,
    commands::delete_model,
    commands::activate_model,
    commands::import_custom_model,
    commands::validate_custom_models,
])
```

- [ ] **Step 5: Verify it compiles**

Run: `cd src-tauri && cargo check 2>&1 | tail -5`
Expected: compilation succeeds

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add import_custom_model and validate_custom_models Tauri commands"
```

---

### Task 3: Add `get_custom_model_path` helper and update backend commands

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add helper to read custom model path from store**

In `src-tauri/src/commands.rs`, add this helper function after the `CustomModelEntry` struct (which was added in Task 2):

```rust
/// Look up a custom model's file path from the settings store.
pub(crate) fn get_custom_model_path(app: &AppHandle, model_id: &str) -> Option<String> {
    let store = app.store("settings.json").ok()?;
    let custom_models: Vec<CustomModelEntry> = store
        .get("customModels")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    custom_models
        .into_iter()
        .find(|m| m.id == model_id)
        .map(|m| m.path)
}
```

- [ ] **Step 2: Update `activate_model` to support custom models**

Replace the `activate_model` function in `src-tauri/src/commands.rs` (lines 202-219) with:

```rust
#[tauri::command]
pub async fn activate_model(app: AppHandle, model_id: String) -> Result<(), String> {
    let model_path = if model_id.starts_with("custom-") {
        let path_str = get_custom_model_path(&app, &model_id)
            .ok_or("Custom model not found in settings")?;
        let path = std::path::PathBuf::from(&path_str);
        if !path.exists() {
            return Err(format!("Custom model file not found: {}", path_str));
        }
        path
    } else {
        let model = registry::find_model(&model_id).ok_or("Unknown model ID")?;
        let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let models_path = models_dir(&app_data)?;
        let path = models_path.join(model.file_name);
        if !path.exists() {
            return Err(format!("Model file not found: {}", model.file_name));
        }
        path
    };

    tokio::task::spawn_blocking(move || {
        crate::inference::local::swap_model(&model_path)
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}
```

- [ ] **Step 3: Update `load_local_model` in lib.rs to support custom models**

Replace the `load_local_model` function in `src-tauri/src/lib.rs` (lines 13-50) with:

```rust
fn load_local_model(app: &tauri::AppHandle) {
    let app_data = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[jolly] Failed to get app data directory: {}", e);
            return;
        }
    };

    let models_path = match model_manager::models_dir(&app_data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[jolly] Failed to get models directory: {}", e);
            return;
        }
    };

    // Read activeModelId from store (if set)
    let active_model_id: Option<String> = app
        .store("settings.json")
        .ok()
        .and_then(|store| store.get("activeModelId"))
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    let model_path = if let Some(ref id) = active_model_id {
        if id.starts_with("custom-") {
            // Custom model: read path using shared helper
            let path_str = commands::get_custom_model_path(app, id);
            match path_str {
                Some(p) => {
                    let path = std::path::PathBuf::from(&p);
                    if !path.exists() {
                        println!("[jolly] Custom model file not found: {}", p);
                        return;
                    }
                    path
                }
                None => {
                    println!("[jolly] Custom model ID not found in settings: {}", id);
                    return;
                }
            }
        } else {
            // Registry model: use resolve_model_path
            match model_manager::resolve_model_path(&models_path, active_model_id.as_deref()) {
                Ok(p) => p,
                Err(e) => {
                    println!("[jolly] No local model available: {}", e);
                    return;
                }
            }
        }
    } else {
        // No active model set: try fallback resolution
        match model_manager::resolve_model_path(&models_path, None) {
            Ok(p) => p,
            Err(e) => {
                println!("[jolly] No local model available: {}", e);
                return;
            }
        }
    };

    match inference::local::init_model(&model_path) {
        Ok(()) => println!("[jolly] Local model loaded: {:?}", model_path),
        Err(e) => eprintln!("[jolly] Failed to load local model: {}", e),
    }
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cd src-tauri && cargo check 2>&1 | tail -5`
Expected: compilation succeeds

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: support custom model IDs in activate_model and load_local_model"
```

---

### Task 4: Add `CustomModel` TypeScript type

**Files:**
- Modify: `src/lib/types/models.ts`

- [ ] **Step 1: Add the interface**

Append to `src/lib/types/models.ts` after the existing types:

```typescript
export interface CustomModel {
	id: string;
	name: string;
	path: string;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/types/models.ts
git commit -m "feat: add CustomModel TypeScript interface"
```

---

### Task 5: Add custom model state and methods to settings store

**Files:**
- Modify: `src/lib/stores/settings.svelte.ts`

- [ ] **Step 1: Add import for `CustomModel` type**

At the top of `src/lib/stores/settings.svelte.ts`, update the import (line 1):

```typescript
import type { ModelWithState, CustomModel } from '$lib/types/models';
```

- [ ] **Step 2: Add `customModels` state**

Inside `createSettingsStore()`, after the `let downloadError` line (after line 37), add:

```typescript
	// Custom imported models — persisted in store
	let customModels = $state<CustomModel[]>([]);
```

- [ ] **Step 3: Add `validateCustomModels` method**

After the `refreshModels` function (after line 85), add:

```typescript
	async function validateCustomModels() {
		if (customModels.length === 0) return;
		try {
			const invoke = await getInvoke();
			const validPaths = await invoke<string[]>('validate_custom_models', {
				paths: customModels.map((m) => m.path)
			});
			const validSet = new Set(validPaths);
			const staleModels = customModels.filter((m) => !validSet.has(m.path));
			if (staleModels.length > 0) {
				customModels = customModels.filter((m) => validSet.has(m.path));
				// If active model was stale, clear selection
				if (activeModelId && staleModels.some((m) => m.id === activeModelId)) {
					activeModelId = null;
					const store = await getStore();
					await store.set('activeModelId', null);
				}
				const store = await getStore();
				await store.set('customModels', customModels);
				await store.save();
			}
		} catch (e) {
			console.warn('Failed to validate custom models:', e);
		}
	}
```

- [ ] **Step 4: Load custom models in `loadAll` and call validation**

In the `loadAll` function, after the line that loads `correctionHistory` (after line 51), add:

```typescript
			customModels = ((await store.get('customModels')) as CustomModel[] | null) ?? [];
```

Then after the `await refreshModels();` call (after line 69), add:

```typescript
		// Validate custom model paths (prune stale references)
		await validateCustomModels();
```

- [ ] **Step 5: Add `importCustomModel` method**

After `validateCustomModels`, add:

```typescript
	async function importCustomModel() {
		try {
			const invoke = await getInvoke();
			const entry = await invoke<CustomModel | null>('import_custom_model');
			if (!entry) return; // User cancelled

			// Deduplicate by path
			if (customModels.some((m) => m.path === entry.path)) {
				return; // Already imported
			}

			customModels = [...customModels, entry];
			const store = await getStore();
			await store.set('customModels', customModels);
			await store.save();
		} catch (e) {
			console.error('Failed to import custom model:', e);
		}
	}
```

- [ ] **Step 6: Add `removeCustomModel` method**

After `importCustomModel`, add:

```typescript
	async function removeCustomModel(id: string) {
		customModels = customModels.filter((m) => m.id !== id);
		try {
			const store = await getStore();
			await store.set('customModels', customModels);
			// If the removed model was active, clear selection
			if (activeModelId === id) {
				activeModelId = null;
				await store.set('activeModelId', null);
			}
			await store.save();
		} catch (e) {
			console.error('Failed to remove custom model:', e);
		}
	}
```

- [ ] **Step 7: Expose new state and methods in the return object**

Add to the return object in `createSettingsStore()`:

After `get correctionHistory()` (after line 306), add:

```typescript
		get customModels() {
			return customModels;
		},
```

After `refreshModels,` (after line 319), add:

```typescript
		importCustomModel,
		removeCustomModel,
```

- [ ] **Step 8: Verify frontend compiles**

Run: `npm run check 2>&1 | tail -10`
Expected: no errors (warnings OK)

- [ ] **Step 9: Commit**

```bash
git add src/lib/stores/settings.svelte.ts
git commit -m "feat: add custom model state and methods to settings store"
```

---

### Task 6: Add import section and custom models to Settings UI

**Files:**
- Modify: `src/lib/components/Settings.svelte`

- [ ] **Step 1: Add CustomModel import**

In `src/lib/components/Settings.svelte`, update the type import (line 3) to also import `CustomModel`:

```typescript
	import type { ModelWithState, CustomModel } from '$lib/types/models';
```

- [ ] **Step 2: Add handleModelSwitch support for custom models**

Update the `handleModelSwitch` function (lines 31-40) to also check custom models for the name:

```typescript
	async function handleModelSwitch(modelId: string) {
		const model = settings.availableModels.find((m) => m.id === modelId);
		const custom = settings.customModels.find((m) => m.id === modelId);
		await settings.setActiveModel(modelId);
		const name = model?.name ?? custom?.name;
		if (name) {
			switchedModelName = name;
			setTimeout(() => {
				switchedModelName = null;
			}, 2000);
		}
	}
```

- [ ] **Step 3: Add "Import Model" section in the template**

After the closing `</section>` of Section 2 (the "Local AI Models" download section, after line 204), add:

```svelte
		<!-- Section: Import Custom Model -->
		<section>
			<h3 class="text-sm font-bold text-[#423f37] dark:text-[#e8e8e3]">Import Model</h3>
			<p class="mt-1 text-xs text-gray-400 dark:text-[#e8e8e3]/50">
				Use your own GGUF model file from disk.
			</p>
			<button
				onclick={() => settings.importCustomModel()}
				class="mt-2 rounded-md border-2 border-[#960200] bg-transparent px-4 py-2 text-sm font-medium text-[#423f37] transition-colors hover:bg-[#ffd046] hover:text-[#960200] dark:border-[#ffd046] dark:text-[#e8e8e3] dark:hover:bg-[#960200] dark:hover:text-[#ffd046]"
			>
				Import .gguf
			</button>
		</section>
```

- [ ] **Step 4: Add custom models to the "Downloaded Models" radio list**

In the "Downloaded Models" section, within the `{:else if !settings.useOpenRouter && !settings.useHarper}` block (after the `{#each downloadedModels as model}` loop's closing `{/each}`, which is at line 256), add the custom models loop:

```svelte
					{#each settings.customModels as model}
						<div
							class="flex items-center gap-2 rounded-md px-3 py-2 transition-colors hover:bg-gray-50 dark:hover:bg-white/5"
						>
							<label class="flex flex-1 cursor-pointer items-center gap-3">
								<input
									type="radio"
									name="active-model"
									value={model.id}
									checked={settings.activeModelId === model.id}
									onchange={() => handleModelSwitch(model.id)}
									class="h-4 w-4 accent-[#960200] dark:accent-[#ffd046]"
								/>
								<span class="text-sm text-[#423f37] dark:text-[#e8e8e3]">
									<span class="font-medium">{model.name}</span>
									<span
										class="block truncate text-xs text-gray-400 dark:text-[#e8e8e3]/40"
										title={model.path}
									>
										{model.path}
									</span>
								</span>
							</label>
							<button
								onclick={() => settings.removeCustomModel(model.id)}
								class="rounded p-1 text-gray-300 transition-colors hover:text-red-500 dark:text-[#e8e8e3]/30 dark:hover:text-red-400"
								aria-label="Remove {model.name}"
							>
								<svg
									xmlns="http://www.w3.org/2000/svg"
									width="14"
									height="14"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
								>
									<polyline points="3 6 5 6 21 6" />
									<path
										d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
									/>
								</svg>
							</button>
						</div>
					{/each}
```

- [ ] **Step 5: Update empty state check to include custom models**

The "No models downloaded yet" check (line 209) currently only checks `downloadedModels.length === 0`. Update it to also check custom models:

```svelte
			{#if downloadedModels.length === 0 && settings.customModels.length === 0}
```

- [ ] **Step 6: Verify frontend compiles**

Run: `npm run check 2>&1 | tail -10`
Expected: no errors

- [ ] **Step 7: Commit**

```bash
git add src/lib/components/Settings.svelte
git commit -m "feat: add import model section and custom models to Settings UI"
```

---

### Task 7: Manual smoke test

- [ ] **Step 1: Start the dev environment**

Run: `npm run tauri dev`

- [ ] **Step 2: Test import flow**

1. Open Settings panel
2. Click "Import .gguf" button — native file dialog should appear
3. Select a `.gguf` file — it should appear in the "Downloaded Models" list with its name and path
4. Select the imported model via radio button — "Switched to ..." message should appear
5. Close and reopen Settings — the imported model should persist

- [ ] **Step 3: Test duplicate rejection**

1. Click "Import .gguf" and select the same file again
2. It should silently not add a duplicate

- [ ] **Step 4: Test removal**

1. Click the trash icon on an imported model
2. It should disappear from the list
3. If it was the active model, the radio selection should be cleared

- [ ] **Step 5: Test stale reference pruning**

1. Import a model, close the app
2. Move/rename the original `.gguf` file
3. Reopen the app — the stale model should be silently removed from the list

- [ ] **Step 6: Final commit if any fixes needed**

```bash
git add -A && git commit -m "fix: address issues found during smoke test"
```
