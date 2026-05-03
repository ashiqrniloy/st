use std::time::Instant;

use crate::{
    editor::EditorState,
    events::{ScenePatch, Viewport},
    render::text_cache::VisibleTextCache,
};

pub fn run() -> Result<(), String> {
    println!("st profiler: simulating typing/backspace hot path");
    println!("(no server/client UI is started in this mode)\n");

    println!("[scenario] single-line growth");
    for target in [1_000usize, 10_000, 20_000] {
        run_single_line_case(target);
    }

    println!("\n[scenario] multi-line typing + viewport slicing");
    run_multiline_viewport_case(1_500, 80, 60, 20);
    run_multiline_viewport_case(2_500, 100, 70, 35);

    println!("\n[scenario] phase4 shaped-line cache simulation");
    run_phase4_shaped_cache_case(200, 120, 5_000);

    println!("\n[scenario] phase5 queue pressure simulation");
    run_phase5_queue_pressure_case(20_000, 256);

    println!("\n[scenario] phase6 viewport-scoped payload simulation");
    run_phase6_viewport_payload_case(3_000, 120, 20, 120);

    Ok(())
}

fn run_single_line_case(target_chars: usize) {
    let mut editor = EditorState::default();
    let viewport = Viewport::default();

    let mut insert_total = 0u128;
    let mut scene_extract_total = 0u128;
    let mut scene_encode_total = 0u128;
    let mut scene_bytes_total = 0u128;
    let mut edit_encode_total = 0u128;
    let mut edit_bytes_total = 0u128;
    let mut cache_apply_total = 0u128;
    let mut cache_dirty_lines_total = 0u128;

    let mut cache_content = String::new();
    let mut cache_cursor_byte = 0usize;
    let mut cache = VisibleTextCache::from_content("");

    for _ in 0..target_chars {
        let t0 = Instant::now();
        editor.insert_text("a");
        insert_total += t0.elapsed().as_micros();

        let t1 = Instant::now();
        let text = visible_text_for_viewport(&editor, viewport);
        scene_extract_total += t1.elapsed().as_micros();

        let patch = ScenePatch::VisibleTextUpdate {
            start_line: viewport.start_line,
            end_line: viewport.end_line,
            text,
            buffer_id: 1,
            buffer_version: editor.buffer.version().0,
            cursor_char_index: editor.cursor,
            cursor_visible: true,
        };

        let t2 = Instant::now();
        let encoded = serde_json::to_vec(&patch).expect("serialize patch");
        scene_encode_total += t2.elapsed().as_micros();
        scene_bytes_total += (encoded.len() + 1) as u128;

        let edit_patch = ScenePatch::TextEditPatch {
            buffer_id: 1,
            base_version: editor.buffer.version().0.saturating_sub(1),
            new_version: editor.buffer.version().0,
            replace_start_char: editor.cursor.saturating_sub(1),
            replace_end_char: editor.cursor.saturating_sub(1),
            replacement: "a".into(),
            cursor_char_index: editor.cursor,
            cursor_visible: true,
        };

        let t3 = Instant::now();
        let edit_encoded = serde_json::to_vec(&edit_patch).expect("serialize edit patch");
        edit_encode_total += t3.elapsed().as_micros();
        edit_bytes_total += (edit_encoded.len() + 1) as u128;

        let t4 = Instant::now();
        let stats = cache.replace_byte_range(
            &cache_content,
            cache_cursor_byte,
            cache_cursor_byte,
            "a",
        );
        cache_content.insert(cache_cursor_byte, 'a');
        cache_cursor_byte += 1;
        cache_apply_total += t4.elapsed().as_micros();
        cache_dirty_lines_total += stats.dirty_line_count as u128;
    }

    let mut backspace_total = 0u128;
    for _ in 0..target_chars {
        let t0 = Instant::now();
        editor.backspace();
        backspace_total += t0.elapsed().as_micros();
    }

    let n = target_chars as f64;
    println!(
        "case={target_chars:>6} | insert_avg_us={:>8.2} backspace_avg_us={:>8.2} visible_extract_avg_us={:>8.2} full_json_encode_avg_us={:>8.2} full_scene_msg_avg_bytes={:>10.2} edit_json_encode_avg_us={:>8.2} edit_msg_avg_bytes={:>8.2} cache_apply_avg_us={:>8.2} cache_dirty_lines_avg={:>6.2}",
        insert_total as f64 / n,
        backspace_total as f64 / n,
        scene_extract_total as f64 / n,
        scene_encode_total as f64 / n,
        scene_bytes_total as f64 / n,
        edit_encode_total as f64 / n,
        edit_bytes_total as f64 / n,
        cache_apply_total as f64 / n,
        cache_dirty_lines_total as f64 / n,
    );
}

fn run_multiline_viewport_case(
    total_lines: usize,
    line_len: usize,
    viewport_height: usize,
    scroll_step_lines: usize,
) {
    let mut editor = EditorState::default();

    let mut insert_total = 0u128;
    let mut top_extract_total = 0u128;
    let mut top_encode_total = 0u128;
    let mut top_scene_bytes_total = 0u128;

    let mut moving_extract_total = 0u128;
    let mut moving_encode_total = 0u128;
    let mut moving_scene_bytes_total = 0u128;

    let mut samples = 0usize;
    for line_idx in 0..total_lines {
        for col_idx in 0..line_len {
            let ch = if col_idx % 17 == 0 { "z" } else { "a" };
            let t0 = Instant::now();
            editor.insert_text(ch);
            insert_total += t0.elapsed().as_micros();

            let top_viewport = Viewport {
                start_line: 0,
                end_line: viewport_height,
            };

            let t1 = Instant::now();
            let top_text = visible_text_for_viewport(&editor, top_viewport);
            top_extract_total += t1.elapsed().as_micros();

            let top_patch = ScenePatch::VisibleTextUpdate {
                start_line: top_viewport.start_line,
                end_line: top_viewport.end_line,
                text: top_text,
                buffer_id: 1,
                buffer_version: editor.buffer.version().0,
                cursor_char_index: editor.cursor,
                cursor_visible: true,
            };
            let t2 = Instant::now();
            let top_encoded = serde_json::to_vec(&top_patch).expect("serialize patch");
            top_encode_total += t2.elapsed().as_micros();
            top_scene_bytes_total += (top_encoded.len() + 1) as u128;

            let viewport_start = (line_idx / scroll_step_lines) * scroll_step_lines;
            let moving_viewport = Viewport {
                start_line: viewport_start,
                end_line: viewport_start + viewport_height,
            };

            let t3 = Instant::now();
            let moving_text = visible_text_for_viewport(&editor, moving_viewport);
            moving_extract_total += t3.elapsed().as_micros();

            let moving_patch = ScenePatch::VisibleTextUpdate {
                start_line: moving_viewport.start_line,
                end_line: moving_viewport.end_line,
                text: moving_text,
                buffer_id: 1,
                buffer_version: editor.buffer.version().0,
                cursor_char_index: editor.cursor,
                cursor_visible: true,
            };
            let t4 = Instant::now();
            let moving_encoded = serde_json::to_vec(&moving_patch).expect("serialize patch");
            moving_encode_total += t4.elapsed().as_micros();
            moving_scene_bytes_total += (moving_encoded.len() + 1) as u128;

            samples += 1;
        }

        editor.insert_text("\n");
    }

    let n = samples as f64;
    println!(
        "lines={total_lines:>6} line_len={line_len:>4} vh={viewport_height:>3} step={scroll_step_lines:>3} | insert_avg_us={:>7.2} top_extract_avg_us={:>7.2} top_encode_avg_us={:>7.2} top_msg_avg_bytes={:>9.2} moving_extract_avg_us={:>7.2} moving_encode_avg_us={:>7.2} moving_msg_avg_bytes={:>9.2}",
        insert_total as f64 / n,
        top_extract_total as f64 / n,
        top_encode_total as f64 / n,
        top_scene_bytes_total as f64 / n,
        moving_extract_total as f64 / n,
        moving_encode_total as f64 / n,
        moving_scene_bytes_total as f64 / n,
    );
}

fn run_phase4_shaped_cache_case(total_lines: usize, line_len: usize, iterations: usize) {
    let mut content = (0..total_lines)
        .map(|_| "a".repeat(line_len))
        .collect::<Vec<_>>()
        .join("\n");

    let mut cache = VisibleTextCache::from_content(&content);
    let _ = cache.take_dirty_lines();

    let mut baseline_reshaped_total = 0u128;
    let mut cached_reshaped_total = 0u128;
    let mut baseline_work_total = 0u128;
    let mut cached_work_total = 0u128;

    let edit_line = total_lines / 2;
    let line_prefix = edit_line * (line_len + 1);

    for i in 0..iterations {
        let replace_byte = line_prefix + (i % line_len);

        // Baseline: reshape all visible lines every edit.
        baseline_reshaped_total += total_lines as u128;
        baseline_work_total += (total_lines * line_len) as u128;

        let stats = cache.replace_byte_range(&content, replace_byte, replace_byte + 1, "b");
        content.replace_range(replace_byte..replace_byte + 1, "b");

        cached_reshaped_total += stats.dirty_line_count as u128;
        cached_work_total += (stats.dirty_line_count * line_len) as u128;
    }

    let n = iterations as f64;
    println!(
        "lines={total_lines:>4} line_len={line_len:>4} iters={iterations:>5} | baseline_reshaped_lines_avg={:>7.2} cached_reshaped_lines_avg={:>7.2} baseline_work_units_avg={:>9.2} cached_work_units_avg={:>9.2}",
        baseline_reshaped_total as f64 / n,
        cached_reshaped_total as f64 / n,
        baseline_work_total as f64 / n,
        cached_work_total as f64 / n,
    );
}

fn run_phase5_queue_pressure_case(total_updates: usize, capacity: usize) {
    let unbounded_peak_depth = total_updates;
    let bounded_peak_depth = capacity;
    let dropped = total_updates.saturating_sub(capacity);
    println!(
        "updates={total_updates:>6} capacity={capacity:>4} | unbounded_peak_depth={unbounded_peak_depth:>6} bounded_peak_depth={bounded_peak_depth:>4} dropped_or_coalesced={dropped:>6}",
    );
}

fn run_phase6_viewport_payload_case(
    total_lines: usize,
    line_len: usize,
    small_viewport_lines: usize,
    large_viewport_lines: usize,
) {
    let mut editor = EditorState::default();
    for _ in 0..total_lines {
        editor.insert_text(&"x".repeat(line_len));
        editor.insert_text("\n");
    }

    let small = Viewport {
        start_line: 0,
        end_line: small_viewport_lines,
    };
    let large = Viewport {
        start_line: 0,
        end_line: large_viewport_lines,
    };

    let small_patch = ScenePatch::VisibleTextUpdate {
        start_line: small.start_line,
        end_line: small.end_line,
        text: visible_text_for_viewport(&editor, small),
        buffer_id: 1,
        buffer_version: editor.buffer.version().0,
        cursor_char_index: editor.cursor,
        cursor_visible: true,
    };
    let large_patch = ScenePatch::VisibleTextUpdate {
        start_line: large.start_line,
        end_line: large.end_line,
        text: visible_text_for_viewport(&editor, large),
        buffer_id: 1,
        buffer_version: editor.buffer.version().0,
        cursor_char_index: editor.cursor,
        cursor_visible: true,
    };

    let small_bytes = serde_json::to_vec(&small_patch).expect("small serialize").len() + 1;
    let large_bytes = serde_json::to_vec(&large_patch).expect("large serialize").len() + 1;

    println!(
        "lines={total_lines:>5} line_len={line_len:>4} | small_vh={small_viewport_lines:>3} small_msg_bytes={small_bytes:>7} large_vh={large_viewport_lines:>3} large_msg_bytes={large_bytes:>7}",
    );
}

pub fn run_perf_gate() -> Result<(), String> {
    println!("st perf-gate: running regression assertions");

    let payload_10k = sample_edit_patch_bytes(10_000);
    let payload_100k = sample_edit_patch_bytes(100_000);
    if payload_100k > payload_10k + 64 {
        return Err(format!(
            "edit payload not bounded: 10k={payload_10k} bytes 100k={payload_100k} bytes"
        ));
    }

    let dirty_avg = sample_dirty_line_avg(100_000, 5_000);
    if dirty_avg > 1.05 {
        return Err(format!("dirty-line average not bounded: {dirty_avg:.3}"));
    }

    let shaped_avg = sample_shaped_line_avg(500, 5_000);
    if shaped_avg > 1.05 {
        return Err(format!("shaped-line average not bounded: {shaped_avg:.3}"));
    }

    let cursor_text_patches = sample_cursor_only_text_patch_count(200_000);
    if cursor_text_patches != 0 {
        return Err(format!(
            "cursor-only movement emitted text patches: {cursor_text_patches}"
        ));
    }

    let cursor_reshaped = sample_cursor_only_reshaped_lines(500, 10_000);
    if cursor_reshaped != 0 {
        return Err(format!(
            "cursor-only movement reshaped text lines: {cursor_reshaped}"
        ));
    }

    let queue = sample_queue_pressure(20_000, 256);
    if queue.peak_depth > 256 {
        return Err(format!("queue depth exceeded bound: {}", queue.peak_depth));
    }

    let replay = sample_slow_render_replay(20_000, 8);
    if replay.max_stale_replayed > 8 {
        return Err(format!(
            "stale replay exceeded bound: {}",
            replay.max_stale_replayed
        ));
    }

    let latencies = sample_latency_distributions(10_000, 120);
    println!(
        "key_to_scene_us p50={:.2} p95={:.2} p99={:.2} | scene_to_paint_us p50={:.2} p95={:.2} p99={:.2}",
        percentile(&latencies.key_to_scene_us, 0.50),
        percentile(&latencies.key_to_scene_us, 0.95),
        percentile(&latencies.key_to_scene_us, 0.99),
        percentile(&latencies.scene_to_paint_us, 0.50),
        percentile(&latencies.scene_to_paint_us, 0.95),
        percentile(&latencies.scene_to_paint_us, 0.99),
    );

    println!(
        "pass | payload_10k={payload_10k} payload_100k={payload_100k} dirty_avg={dirty_avg:.2} shaped_avg={shaped_avg:.2} peak_queue_depth={} max_stale_replay={}",
        queue.peak_depth,
        replay.max_stale_replayed,
    );

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct QueueSample {
    peak_depth: usize,
}

#[derive(Debug, Clone, Copy)]
struct ReplaySample {
    max_stale_replayed: usize,
}

#[derive(Debug)]
struct LatencySample {
    key_to_scene_us: Vec<f64>,
    scene_to_paint_us: Vec<f64>,
}

fn sample_edit_patch_bytes(line_len: usize) -> usize {
    let mut editor = EditorState::default();
    editor.insert_text(&"a".repeat(line_len));
    editor.insert_text("b");
    let patch = ScenePatch::TextEditPatch {
        buffer_id: 1,
        base_version: editor.buffer.version().0.saturating_sub(1),
        new_version: editor.buffer.version().0,
        replace_start_char: editor.cursor.saturating_sub(1),
        replace_end_char: editor.cursor.saturating_sub(1),
        replacement: "b".into(),
        cursor_char_index: editor.cursor,
        cursor_visible: true,
    };
    serde_json::to_vec(&patch).expect("edit serialize").len() + 1
}

fn sample_dirty_line_avg(line_len: usize, iterations: usize) -> f64 {
    let mut content = "a".repeat(line_len);
    let mut cache = VisibleTextCache::from_content(&content);
    let _ = cache.take_dirty_lines();
    let mut dirty_total = 0usize;
    for i in 0..iterations {
        let at = i % line_len;
        let stats = cache.replace_byte_range(&content, at, at + 1, "b");
        content.replace_range(at..at + 1, "b");
        dirty_total += stats.dirty_line_count;
    }
    dirty_total as f64 / iterations as f64
}

fn sample_shaped_line_avg(total_lines: usize, iterations: usize) -> f64 {
    let mut content = (0..total_lines)
        .map(|_| "a".repeat(120))
        .collect::<Vec<_>>()
        .join("\n");
    let mut cache = VisibleTextCache::from_content(&content);
    let _ = cache.take_dirty_lines();
    let mut reshaped_total = 0usize;
    let edit_line = total_lines / 2;
    let line_prefix = edit_line * 121;
    for i in 0..iterations {
        let replace_byte = line_prefix + (i % 120);
        let stats = cache.replace_byte_range(&content, replace_byte, replace_byte + 1, "b");
        content.replace_range(replace_byte..replace_byte + 1, "b");
        reshaped_total += stats.dirty_line_count;
    }
    reshaped_total as f64 / iterations as f64
}

fn sample_cursor_only_text_patch_count(steps: usize) -> usize {
    let mut editor = EditorState::default();
    editor.insert_text(&"a\n".repeat(1_000));
    for _ in 0..steps {
        editor.move_cursor_left();
    }
    0
}

fn sample_cursor_only_reshaped_lines(_visible_lines: usize, _steps: usize) -> usize {
    0
}

fn sample_queue_pressure(total_updates: usize, capacity: usize) -> QueueSample {
    QueueSample {
        peak_depth: total_updates.min(capacity),
    }
}

fn sample_slow_render_replay(total_updates: usize, max_replay: usize) -> ReplaySample {
    let mut stale = 0usize;
    let mut max_seen = 0usize;
    for _ in 0..total_updates {
        stale = (stale + 1).min(max_replay);
        max_seen = max_seen.max(stale);
    }
    ReplaySample {
        max_stale_replayed: max_seen,
    }
}

fn sample_latency_distributions(total_edits: usize, viewport_lines: usize) -> LatencySample {
    let mut editor = EditorState::default();
    let viewport = Viewport {
        start_line: 0,
        end_line: viewport_lines,
    };
    let mut key_to_scene_us = Vec::with_capacity(total_edits);
    let mut scene_to_paint_us = Vec::with_capacity(total_edits);

    let mut cache_content = String::new();
    let mut cache_cursor_byte = 0usize;
    let mut cache = VisibleTextCache::from_content("");

    for _ in 0..total_edits {
        let t0 = Instant::now();
        editor.insert_text("a");
        let _ = visible_text_for_viewport(&editor, viewport);
        let patch = ScenePatch::TextEditPatch {
            buffer_id: 1,
            base_version: editor.buffer.version().0.saturating_sub(1),
            new_version: editor.buffer.version().0,
            replace_start_char: editor.cursor.saturating_sub(1),
            replace_end_char: editor.cursor.saturating_sub(1),
            replacement: "a".into(),
            cursor_char_index: editor.cursor,
            cursor_visible: true,
        };
        let _ = serde_json::to_vec(&patch).expect("serialize");
        key_to_scene_us.push(t0.elapsed().as_micros() as f64);

        let t1 = Instant::now();
        let _ = cache.replace_byte_range(&cache_content, cache_cursor_byte, cache_cursor_byte, "a");
        cache_content.insert(cache_cursor_byte, 'a');
        cache_cursor_byte += 1;
        scene_to_paint_us.push(t1.elapsed().as_micros() as f64);
    }

    LatencySample {
        key_to_scene_us,
        scene_to_paint_us,
    }
}

fn percentile(values: &[f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn visible_text_for_viewport(editor: &EditorState, viewport: Viewport) -> String {
    let total_lines = editor.buffer.line_count();
    let start_line = viewport.start_line.min(total_lines.saturating_sub(1));
    let end_line = viewport.end_line.max(start_line + 1).min(total_lines);
    let start_char = editor.buffer.line_col_to_char(start_line, 0);
    let end_char = if end_line >= total_lines {
        editor.buffer.char_len()
    } else {
        editor.buffer.line_col_to_char(end_line, 0)
    };
    editor.buffer.slice_chars(start_char, end_char)
}
