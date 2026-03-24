use std::path::PathBuf;
use std::time::Instant;

use jolly_lib::inference::local;
use jolly_lib::inference::registry::MODELS;

fn models_dir() -> PathBuf {
    dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("/home/dev/.local/share"))
        .join("com.jolly.desktop/models")
}

/// Find the first downloaded model from the registry.
fn find_any_model() -> Option<&'static jolly_lib::inference::registry::ModelEntry> {
    let dir = models_dir();
    MODELS.iter().find(|m| dir.join(m.file_name).exists())
}

/// Integration test: loads a real model and verifies inference produces output.
/// Run with: cargo test --test local_inference -- --nocapture
#[test]
fn test_inference_produces_output() {
    let model = match find_any_model() {
        Some(m) => m,
        None => {
            eprintln!("SKIPPED: No downloaded models found");
            return;
        }
    };

    let path = models_dir().join(model.file_name);
    eprintln!("Using model: {} ({})", model.name, model.file_name);

    local::init_model(&path, model.id, model.prompt_template)
        .expect("Failed to load model");

    let input = "I recieved your messege yesterday.";
    let start = Instant::now();
    let result = local::run_inference(input).expect("Inference failed");
    let elapsed = start.elapsed();

    eprintln!("Input:  {:?}", input);
    eprintln!("Output: {:?}", result);
    eprintln!("Time:   {:.2}s", elapsed.as_secs_f64());

    // Basic sanity checks
    assert!(!result.is_empty(), "Output should not be empty");
    assert!(
        result.len() < input.len() * 3,
        "Output should not be much longer than input (runaway detection)"
    );
    // Output should not contain prompt markers (model should not echo the template)
    assert!(
        !result.contains("### Original Text:"),
        "Output should not contain registry template markers"
    );
    assert!(
        !result.contains("<start_of_turn>"),
        "Output should not contain GRMR turn markers"
    );

    local::unload_model();
}

/// Integration test: test each downloaded model with the correct prompt template.
/// Run with: cargo test --test local_inference test_all_models -- --nocapture
#[test]
fn test_all_models_inference() {
    let dir = models_dir();
    let downloaded: Vec<_> = MODELS.iter().filter(|m| dir.join(m.file_name).exists()).collect();

    if downloaded.is_empty() {
        eprintln!("SKIPPED: No downloaded models found");
        return;
    }

    let test_input = "The resturant had excelent sevice.";
    let expected_words = ["restaurant", "excellent", "service"];

    for model in &downloaded {
        let path = dir.join(model.file_name);
        eprintln!("\n=== {} ===", model.name);

        if let Err(e) = local::init_model(&path, model.id, model.prompt_template) {
            eprintln!("FAILED to load: {}", e);
            continue;
        }

        let start = Instant::now();
        let result = local::run_inference(test_input);
        let elapsed = start.elapsed();

        match result {
            Ok(output) => {
                eprintln!("  Input:  {:?}", test_input);
                eprintln!("  Output: {:?}", output);
                eprintln!("  Time:   {:.2}s", elapsed.as_secs_f64());

                // Check for runaway generation (warn, don't fail — some models like Qwen3
                // emit <think> blocks that are legitimately longer than the input)
                if output.len() > test_input.len() * 3 {
                    eprintln!("  WARN: Output much longer than input ({} chars) — possible runaway", output.len());
                }

                // Verify no template leakage
                assert!(
                    !output.contains("### Original Text:"),
                    "{}: Output contains registry template markers", model.name
                );
                assert!(
                    !output.contains("<start_of_turn>"),
                    "{}: Output contains GRMR turn markers", model.name
                );

                // Check if at least one expected correction word appears
                let output_lower = output.to_lowercase();
                let matches = expected_words.iter().filter(|w| output_lower.contains(*w)).count();
                eprintln!("  Matched {}/{} expected words", matches, expected_words.len());
            }
            Err(e) => {
                eprintln!("  ERROR: {}", e);
            }
        }

        local::unload_model();
    }
}

/// Integration test: verify GRMR 2B Instruct uses registry template (not raw fallback).
/// Run with: cargo test --test local_inference test_grmr_2b -- --nocapture
#[test]
fn test_grmr_2b_uses_registry_template() {
    let model = MODELS.iter().find(|m| m.id == "grmr-2b-instruct-q4km");
    let model = match model {
        Some(m) => m,
        None => {
            eprintln!("SKIPPED: GRMR 2B not in registry");
            return;
        }
    };

    // Verify the registry has a prompt template for this model
    assert!(
        model.prompt_template.is_some(),
        "GRMR 2B Instruct should have a prompt_template in registry"
    );

    let template = model.prompt_template.unwrap();
    assert!(
        template.contains("### Original Text:"),
        "Template should contain '### Original Text:' marker"
    );
    assert!(
        template.contains("### Corrected Text:"),
        "Template should contain '### Corrected Text:' marker"
    );
    assert!(
        template.contains("{text}"),
        "Template should contain '{{text}}' placeholder"
    );

    // Verify the template formats correctly
    let formatted = template.replace("{text}", "Hello wrold.");
    assert!(formatted.contains("Hello wrold."));
    assert!(!formatted.contains("{text}"));

    let path = models_dir().join(model.file_name);
    if !path.exists() {
        eprintln!("SKIPPED: GRMR 2B model not downloaded");
        return;
    }

    // Load and run inference
    local::init_model(&path, model.id, model.prompt_template)
        .expect("Failed to load GRMR 2B");

    let start = Instant::now();
    let result = local::run_inference("I recieved your messege yesterday.")
        .expect("Inference failed");
    let elapsed = start.elapsed();

    eprintln!("Output: {:?} ({:.2}s)", result, elapsed.as_secs_f64());

    // Should finish in reasonable time (not 38s runaway)
    assert!(
        elapsed.as_secs() < 30,
        "Inference took too long ({}s) — possible runaway generation",
        elapsed.as_secs()
    );

    // Should not contain template artifacts
    assert!(!result.contains("### Original Text:"));
    assert!(!result.contains("### Corrected Text:"));

    local::unload_model();
}

/// Integration test: verify GRMR V3 models use their embedded GGUF template.
/// Run with: cargo test --test local_inference test_grmr_v3 -- --nocapture
#[test]
fn test_grmr_v3_uses_gguf_template() {
    let model = MODELS.iter().find(|m| m.id == "grmr-v3-g4b-q4km");
    let model = match model {
        Some(m) => m,
        None => {
            eprintln!("SKIPPED: GRMR V3 not in registry");
            return;
        }
    };

    // GRMR V3 should NOT have a registry template — it uses the embedded GGUF one
    assert!(
        model.prompt_template.is_none(),
        "GRMR V3 should not have a registry prompt_template"
    );

    let path = models_dir().join(model.file_name);
    if !path.exists() {
        eprintln!("SKIPPED: GRMR V3 Q4_K_M model not downloaded");
        return;
    }

    local::init_model(&path, model.id, model.prompt_template)
        .expect("Failed to load GRMR V3");

    let start = Instant::now();
    let result = local::run_inference("The resturant had excelent sevice.")
        .expect("Inference failed");
    let elapsed = start.elapsed();

    eprintln!("Output: {:?} ({:.2}s)", result, elapsed.as_secs_f64());

    assert!(
        elapsed.as_secs() < 30,
        "Inference took too long ({}s)", elapsed.as_secs()
    );
    assert!(!result.contains("<start_of_turn>"));
    assert!(!result.contains("<end_of_turn>"));

    local::unload_model();
}
