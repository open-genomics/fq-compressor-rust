// =============================================================================
// fqc-rust - Stage Timing Stats
// =============================================================================
// Verifies that compression stage timings (parse/reorder/process/write) are
// populated on the stats handoff after an archive compression.
// =============================================================================

use std::io::Write as _;

use fqc::commands::compress::CompressOptions;
use fqc::engine::compression_engine::CompressionEngine;

#[test]
fn archive_stats_carry_stage_timings() {
    let mut seq = Vec::new();
    for i in 0..1000u32 {
        let id = format!("r{i}");
        let seq_body = "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT";
        writeln!(seq, "@{id}").unwrap();
        writeln!(seq, "{seq_body}").unwrap();
        writeln!(seq, "+").unwrap();
        writeln!(seq, "IIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIII").unwrap();
    }
    let dir = std::env::temp_dir();
    let input = dir.join("fqc_stats_timing_input.fastq");
    let out = dir.join("fqc_stats_timing_out.fqc");
    std::fs::write(&input, seq).unwrap();

    let opts = CompressOptions {
        input_path: input.to_string_lossy().to_string(),
        output_path: out.to_string_lossy().to_string(),
        show_progress: false,
        ..Default::default()
    };
    let outcome = CompressionEngine::new().run(opts.to_request()).unwrap();
    assert!(
        outcome.stats.parse_ms + outcome.stats.process_ms + outcome.stats.write_ms > 0,
        "archive compression should fill stage timings (parse={}, reorder={}, process={}, write={})",
        outcome.stats.parse_ms,
        outcome.stats.reorder_ms,
        outcome.stats.process_ms,
        outcome.stats.write_ms
    );

    std::fs::remove_file(&input).ok();
    std::fs::remove_file(&out).ok();
}
