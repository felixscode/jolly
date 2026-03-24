use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use jolly_lib::inference::harper::HarperProvider;
use jolly_lib::inference::local::{self, LocalProvider};
use jolly_lib::inference::openrouter::OpenRouterProvider;
use jolly_lib::inference::registry::MODELS;
use jolly_lib::inference::LLMProvider;

struct TestCase {
    id: usize,
    language: &'static str,
    category: &'static str,
    input: &'static str,
    expected: &'static str,
}

/// Read RSS (Resident Set Size) in MB from /proc/self/status.
fn rss_mb() -> f64 {
    fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<f64>().ok())
        })
        .map(|kb| kb / 1024.0)
        .unwrap_or(0.0)
}

/// An error span: a contiguous region where input differs from expected.
struct ErrorSpan {
    /// Context window: a few chars before/after the fix from expected,
    /// used to verify the fix appears in the right location in output.
    context: String,
}

/// Find error spans by computing the Levenshtein edit matrix between input and expected,
/// then grouping contiguous edits into spans with surrounding context.
fn find_error_spans(input: &str, expected: &str) -> Vec<ErrorSpan> {
    let a: Vec<char> = input.chars().collect();
    let b: Vec<char> = expected.chars().collect();
    let n = a.len();
    let m = b.len();

    // Build DP matrix
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in 0..=n { dp[i][0] = i as u32; }
    for j in 0..=m { dp[0][j] = j as u32; }
    for i in 1..=n {
        for j in 1..=m {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
            } else {
                dp[i][j] = 1 + dp[i - 1][j - 1].min(dp[i - 1][j]).min(dp[i][j - 1]);
            }
        }
    }

    // Backtrace to find edit operations
    // 'M' = match, 'S' = substitute, 'D' = delete from input, 'I' = insert into input
    let mut ops: Vec<char> = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && a[i - 1] == b[j - 1] {
            ops.push('M');
            i -= 1;
            j -= 1;
        } else if i > 0 && j > 0 && dp[i][j] == dp[i - 1][j - 1] + 1 {
            ops.push('S');
            i -= 1;
            j -= 1;
        } else if i > 0 && dp[i][j] == dp[i - 1][j] + 1 {
            ops.push('D');
            i -= 1;
        } else {
            ops.push('I');
            j -= 1;
        }
    }
    ops.reverse();

    // Group contiguous non-match ops into spans, tracking positions in expected
    let mut spans = Vec::new();
    let mut in_error = false;
    let mut span_start_b = 0usize; // start position in expected
    let mut bj = 0usize; // current position in expected

    for op in &ops {
        match op {
            'M' => {
                if in_error {
                    // Grab context: 3 chars before + fix + 3 chars after from expected
                    let ctx_start = span_start_b.saturating_sub(3);
                    let ctx_end = (bj + 3).min(m);
                    let context: String = b[ctx_start..ctx_end].iter().collect();
                    spans.push(ErrorSpan { context });
                    in_error = false;
                }
                bj += 1;
            }
            'S' | 'I' => {
                if !in_error {
                    span_start_b = bj;
                    in_error = true;
                }
                bj += 1;
            }
            'D' => {
                if !in_error {
                    span_start_b = bj;
                    in_error = true;
                }
            }
            _ => {}
        }
    }
    if in_error {
        let ctx_start = span_start_b.saturating_sub(3);
        let ctx_end = (bj + 3).min(m);
        let context: String = b[ctx_start..ctx_end].iter().collect();
        spans.push(ErrorSpan { context });
    }

    spans
}

/// Count how many error spans (from input→expected diff) are fixed in the output.
/// Uses context windows (fix + surrounding chars) to avoid false positives.
/// Returns (fixed, total).
fn count_errors_fixed(input: &str, expected: &str, output: &str) -> (usize, usize) {
    let spans = find_error_spans(input, expected);
    let total = spans.len();
    if total == 0 {
        return (0, 0);
    }
    let fixed = spans.iter().filter(|s| output.contains(&s.context)).count();
    (fixed, total)
}

fn test_cases() -> Vec<TestCase> {
    let mut id = 0;
    let mut next_id = || {
        id += 1;
        id
    };

    vec![
        // ── Short EN (2) ──────────────────────────────────────
        TestCase {
            id: next_id(), language: "en", category: "short",
            input: "I recieved your messege yesterday.",
            expected: "I received your message yesterday.",
        },
        TestCase {
            id: next_id(), language: "en", category: "short",
            input: "The resturant had excelent sevice.",
            expected: "The restaurant had excellent service.",
        },

        // ── Short DE (2) ───────────────────────────────────────
        TestCase {
            id: next_id(), language: "de", category: "short",
            input: "Ich habe gestern deine Nachich erhalten.",
            expected: "Ich habe gestern deine Nachricht erhalten.",
        },
        TestCase {
            id: next_id(), language: "de", category: "short",
            input: "Die Regirung hat neue Massnahmen beschlosen.",
            expected: "Die Regierung hat neue Maßnahmen beschlossen.",
        },

        // ── Medium EN (1) ──────────────────────────────────────
        TestCase {
            id: next_id(), language: "en", category: "medium",
            input: "The anual report has been finalized and is reddy for distribusion. It includes a comprehensve overview of our finacial performence, key achivements, and strategec goals for the upcomming year. All departement heads should reveiw the relevent sections and provide there feedback by the end of next week. We are confidant that the resuts will demonstarte our strong market postion. Please do not hesistate to reach out if you have any questons.",
            expected: "The annual report has been finalized and is ready for distribution. It includes a comprehensive overview of our financial performance, key achievements, and strategic goals for the upcoming year. All department heads should review the relevant sections and provide their feedback by the end of next week. We are confident that the results will demonstrate our strong market position. Please do not hesitate to reach out if you have any questions.",
        },

        // ── Medium DE (1) ──────────────────────────────────────
        TestCase {
            id: next_id(), language: "de", category: "medium",
            input: "Der Jahresbericht wurde fertiggestelt und ist bereit zur Verteilung. Er enthält einen umfasenden Überblick über unsere finanziele Leistung, wichtige Errungenschaften und strategische Ziele für das komende Jahr. Alle Abteilungsleiter solten die relevanten Abschnite überprüfen und bis Ende nächster Woche ihr Fedback geben. Wir sind zuversichtlich, dass die Ergebnise unsere starke Marktpostion zeigen werden. Bitte zögern Sie nicht, sich bei Fragen an uns zu weden.",
            expected: "Der Jahresbericht wurde fertiggestellt und ist bereit zur Verteilung. Er enthält einen umfassenden Überblick über unsere finanzielle Leistung, wichtige Errungenschaften und strategische Ziele für das kommende Jahr. Alle Abteilungsleiter sollten die relevanten Abschnitte überprüfen und bis Ende nächster Woche ihr Feedback geben. Wir sind zuversichtlich, dass die Ergebnisse unsere starke Marktposition zeigen werden. Bitte zögern Sie nicht, sich bei Fragen an uns zu wenden.",
        },

        // ── Email EN (1) ───────────────────────────────────────
        TestCase {
            id: next_id(), language: "en", category: "email",
            input: "Hi Sarah,\n\nI hope this email findz you well. I wanted to follow up on our conversaton from last week regardng the new project timelien.\n\nAfter revieing the requiremants, I beleive we can meet the orignal deadlien if we allocate addtional resurces to the developement team. Could we scheduel a quick call tommorow to discus this furthur?\n\nBest regards,\nTom",
            expected: "Hi Sarah,\n\nI hope this email finds you well. I wanted to follow up on our conversation from last week regarding the new project timeline.\n\nAfter reviewing the requirements, I believe we can meet the original deadline if we allocate additional resources to the development team. Could we schedule a quick call tomorrow to discuss this further?\n\nBest regards,\nTom",
        },

        // ── Email DE (1) ───────────────────────────────────────
        TestCase {
            id: next_id(), language: "de", category: "email",
            input: "Hallo Frau Müller,\n\nich hofe, diese E-Mail ereicht Sie gut. Ich wollte mich bezüglich unseres Gesprächs von letzter Woche zum neuen Projektzeitplan melden.\n\nNach Durchsicht der Anforderugen bin ich der Meinung, dass wir den ursprünglichen Termin einhalten könen, wenn wir dem Entwicklungsteam zusätzliche Ressorcen zuweisen. Könnten wir morgen einen kurzen Anruf vereinbahren, um dies weiter zu bespreschen?\n\nMit freundlichen Grüßen,\nThomas Schmidt",
            expected: "Hallo Frau Müller,\n\nich hoffe, diese E-Mail erreicht Sie gut. Ich wollte mich bezüglich unseres Gesprächs von letzter Woche zum neuen Projektzeitplan melden.\n\nNach Durchsicht der Anforderungen bin ich der Meinung, dass wir den ursprünglichen Termin einhalten können, wenn wir dem Entwicklungsteam zusätzliche Ressourcen zuweisen. Könnten wir morgen einen kurzen Anruf vereinbaren, um dies weiter zu besprechen?\n\nMit freundlichen Grüßen,\nThomas Schmidt",
        },
    ]
}

fn escape_csv(s: &str) -> String {
    // CSV: wrap in quotes, double any existing quotes
    let escaped = s.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

/// Write a single benchmark result row to the CSV.
fn write_result(
    csv: &mut fs::File,
    case: &TestCase,
    model_id: &str,
    model_name: &str,
    output: &str,
    elapsed_ms: u128,
    current_rss: f64,
) {
    let trimmed = output.trim();
    let exact_match = trimmed == case.expected.trim();
    let (fixed, total) = count_errors_fixed(case.input, case.expected, trimmed);
    eprintln!(
        "  [{:2}] {:2} {:6} {:>5}ms fixed={}/{} {} | {}",
        case.id,
        case.language,
        case.category,
        elapsed_ms,
        fixed,
        total,
        if exact_match { "PASS" } else { "FAIL" },
        &output.chars().take(60).collect::<String>(),
    );
    writeln!(
        csv,
        "{},{},{},{},{},{},{},{},{:.0},{},{},{}",
        case.id,
        case.language,
        case.category,
        model_id,
        escape_csv(model_name),
        exact_match,
        format!("{}/{}", fixed, total),
        elapsed_ms,
        current_rss,
        escape_csv(case.input),
        escape_csv(case.expected),
        escape_csv(output),
    )
    .unwrap();
}

/// Write an error row to the CSV.
fn write_error(
    csv: &mut fs::File,
    case: &TestCase,
    model_id: &str,
    model_name: &str,
    error: &str,
    elapsed_ms: u128,
    current_rss: f64,
) {
    let (_, total) = count_errors_fixed(case.input, case.expected, "");
    eprintln!(
        "  [{:2}] {:2} {:6} ERROR: {}",
        case.id, case.language, case.category, error
    );
    writeln!(
        csv,
        "{},{},{},{},{},false,0/{},{},{:.0},{},{},{}",
        case.id,
        case.language,
        case.category,
        model_id,
        escape_csv(model_name),
        total,
        elapsed_ms,
        current_rss,
        escape_csv(case.input),
        escape_csv(case.expected),
        escape_csv(&format!("ERROR: {}", error)),
    )
    .unwrap();
}

/// Benchmark an async LLMProvider against all test cases.
async fn bench_provider(
    csv: &mut fs::File,
    cases: &[TestCase],
    provider: &dyn LLMProvider,
    model_id: &str,
    model_name: &str,
) {
    for case in cases {
        let start = Instant::now();
        let result = provider.correct_text(case.input).await;
        let elapsed_ms = start.elapsed().as_millis();
        let current_rss = rss_mb();

        match result {
            Ok(output) => write_result(csv, case, model_id, model_name, &output, elapsed_ms, current_rss),
            Err(e) => write_error(csv, case, model_id, model_name, &e, elapsed_ms, current_rss),
        }
    }
}

#[tokio::main]
async fn main() {
    // Load .env from project root (one level up from src-tauri/)
    let env_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.env");
    if let Ok(contents) = fs::read_to_string(&env_path) {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                std::env::set_var(key.trim(), value.trim());
            }
        }
        eprintln!("Loaded .env from {}", env_path.display());
    }

    let cases = test_cases();
    eprintln!("Loaded {} test cases", cases.len());

    // Open CSV output (repo root, one level up from src-tauri/)
    let csv_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../benchmark_results.csv");
    let mut csv = fs::File::create(&csv_path).expect("Failed to create CSV file");
    writeln!(
        csv,
        "test_id,language,category,model_id,model_name,exact_match,errors_fixed,time_ms,rss_mb,input,expected,output"
    )
    .unwrap();

    let rss_before = rss_mb();
    eprintln!("Baseline RSS: {:.0} MB", rss_before);

    // ── 1. Harper ────────────────────────────────────────────────
    eprintln!("\n=== Harper ===");
    let harper = HarperProvider::new();
    bench_provider(&mut csv, &cases, &harper, "harper", "Harper").await;

    // ── 2. OpenRouter ────────────────────────────────────────────
    match std::env::var("OPENROUTER_API_KEY") {
        Ok(api_key) if !api_key.is_empty() => {
            eprintln!("\n=== OpenRouter (gpt-4o-mini) ===");
            let openrouter = OpenRouterProvider::new(api_key);
            bench_provider(&mut csv, &cases, &openrouter, "openrouter-gpt4o-mini", "OpenRouter gpt-4o-mini").await;
        }
        _ => {
            eprintln!("\n=== OpenRouter: SKIPPED (no OPENROUTER_API_KEY) ===");
        }
    }

    // ── 3. Local models ──────────────────────────────────────────
    let models_dir = dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("/home/dev/.local/share"))
        .join("com.jolly.desktop/models");

    if !models_dir.exists() {
        eprintln!("\nModels directory not found: {}", models_dir.display());
        eprintln!("Skipping local model benchmarks.");
    } else {
        let downloaded: Vec<_> = MODELS
            .iter()
            .filter(|m| models_dir.join(m.file_name).exists())
            .collect();

        if downloaded.is_empty() {
            eprintln!("\nNo downloaded models found in {}", models_dir.display());
        } else {
            eprintln!("\nFound {} downloaded models:", downloaded.len());
            for m in &downloaded {
                eprintln!("  - {} ({})", m.name, m.file_name);
            }

            let local_provider = LocalProvider::new();

            for model in &downloaded {
                let model_path = models_dir.join(model.file_name);
                eprintln!("\n=== Loading model: {} ===", model.name);

                let load_start = Instant::now();
                if let Err(e) = local::init_model(&model_path, model.id, model.prompt_template) {
                    eprintln!("FAILED to load {}: {}", model.name, e);
                    continue;
                }
                let rss_after_load = rss_mb();
                eprintln!(
                    "Model loaded in {:.1}s | RSS: {:.0} MB (+{:.0} MB)",
                    load_start.elapsed().as_secs_f64(),
                    rss_after_load,
                    rss_after_load - rss_before,
                );

                bench_provider(&mut csv, &cases, &local_provider, model.id, model.name).await;

                local::unload_model();
                eprintln!("Model unloaded | RSS: {:.0} MB", rss_mb());
            }
        }
    }

    csv.flush().unwrap();
    eprintln!("\n=== Results written to {} ===", csv_path.display());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_spans_simple_typos() {
        let spans = find_error_spans(
            "I recieved your messege yesterday.",
            "I received your message yesterday.",
        );
        // "recieved"→"received" and "messege"→"message"
        assert_eq!(spans.len(), 2);
        // Context includes surrounding chars from expected
        assert!(spans.iter().any(|s| s.context.contains("eived")));
        assert!(spans.iter().any(|s| s.context.contains("essage")));
    }

    #[test]
    fn error_spans_umlaut() {
        let spans = find_error_spans("Massnahmen", "Maßnahmen");
        assert!(!spans.is_empty());
        assert!(spans.iter().any(|s| s.context.contains('ß')));
    }

    #[test]
    fn error_spans_identical() {
        let spans = find_error_spans("Hello world.", "Hello world.");
        assert_eq!(spans.len(), 0);
    }

    #[test]
    fn count_fixed_all() {
        let (fixed, total) = count_errors_fixed(
            "I recieved your messege yesterday.",
            "I received your message yesterday.",
            "I received your message yesterday.",
        );
        assert_eq!(fixed, total);
        assert!(total > 0);
    }

    #[test]
    fn count_fixed_none() {
        let (fixed, total) = count_errors_fixed(
            "I recieved your messege yesterday.",
            "I received your message yesterday.",
            "I recieved your messege yesterday.", // unchanged
        );
        assert_eq!(fixed, 0);
        assert!(total > 0);
    }

    #[test]
    fn count_fixed_partial() {
        let (fixed, total) = count_errors_fixed(
            "I recieved your messege yesterday.",
            "I received your message yesterday.",
            "I received your messege yesterday.", // only first fixed
        );
        assert_eq!(fixed, 1);
        assert_eq!(total, 2);
    }

    #[test]
    fn count_fixed_german_umlauts() {
        let (fixed, total) = count_errors_fixed(
            "Die Regirung hat neue Massnahmen beschlosen.",
            "Die Regierung hat neue Maßnahmen beschlossen.",
            "Die Regierung hat neue Maßnahmen beschlossen.",
        );
        assert_eq!(fixed, total);
        assert!(total >= 3); // Regirung→Regierung, ss→ß, osen→ossen
    }

    #[test]
    fn count_fixed_comma_insertion() {
        let (fixed, total) = count_errors_fixed(
            "Hello world",
            "Hello, world",
            "Hello, world",
        );
        assert_eq!(total, 1);
        assert_eq!(fixed, 1);
    }

    #[test]
    fn count_fixed_space_removal() {
        let (fixed, total) = count_errors_fixed(
            "Hello  world",
            "Hello world",
            "Hello world",
        );
        assert_eq!(total, 1);
        assert_eq!(fixed, 1);
    }
}
