use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::render::context::document::scan::canvas_import::tail_refill::TailAppendSeed;
use crate::render::context::document::scan::canvas_import::TombstoneRestoreSeed;
use crate::render::layout::{BlockGridAddress, SceneLayoutCursor};
use crate::render::scene::SceneTailPlan;

use super::stream_canvas_scan_merge;

#[test]
fn new_media_does_not_reuse_free_tombstone_slot() {
    let root = unique_test_dir("worker_append_after_tombstone");
    fs::create_dir_all(&root).expect("create root");
    let new_media = root.join("new_media.png");
    fs::write(&new_media, b"not-a-real-png").expect("write media");

    let (tx, rx) = mpsc::channel();
    stream_canvas_scan_merge(
        vec![new_media.clone()],
        Vec::new(),
        vec![TombstoneRestoreSeed {
            idx: 7,
            id: 42,
            path: PathBuf::from("deleted.png"),
        }],
        100,
        Some(root.as_path()),
        1,
        1,
        1,
        SceneLayoutCursor::new_centered(8.0),
        10,
        2,
        None,
        None,
        None,
        true,
        &tx,
    );

    let result = rx.recv().expect("scan result");
    assert!(result.final_scan);
    assert!(result.restores.is_empty());
    assert!(result.batch.tail.is_none());
    assert_eq!(result.batch.blocks.len(), 1);
    assert_eq!(result.batch.blocks[0].entries.len(), 1);
    assert_eq!(result.batch.blocks[0].entries[0].file.id, 100);
    assert_eq!(result.batch.blocks[0].entries[0].file.path, new_media);
    assert_eq!(result.batch.blocks[0].block_id, 2);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn new_media_appends_to_tail_after_last_block_tombstone_without_backfill() {
    let root = unique_test_dir("worker_tail_append_after_tombstone");
    fs::create_dir_all(&root).expect("create root");
    let first = root.join("first.png");
    let second = root.join("second.png");
    fs::write(&first, b"not-a-real-png").expect("write first media");
    fs::write(&second, b"not-a-real-png").expect("write second media");

    let (tx, rx) = mpsc::channel();
    stream_canvas_scan_merge(
        vec![first.clone(), second.clone()],
        Vec::new(),
        vec![TombstoneRestoreSeed {
            idx: 1000,
            id: 77,
            path: PathBuf::from("deleted.png"),
        }],
        200,
        Some(root.as_path()),
        1,
        1,
        1,
        SceneLayoutCursor::new_centered(8.0),
        1023,
        2,
        Some([0.0, -1.5, 1.5, 0.0]),
        None,
        Some(TailAppendSeed {
            block_id: 1,
            grid: BlockGridAddress { col: 1, row: 0 },
            existing_len: 1023,
            bounds: [0.0, -50.0, 50.0, 0.0],
        }),
        true,
        &tx,
    );

    let first_result = rx.recv().expect("scan result");
    assert_eq!(first_result.restores.len(), 0);
    let tail_append = match first_result.batch.tail.as_ref().expect("tail append batch") {
        SceneTailPlan::Append(plan) => plan,
        SceneTailPlan::Refill(_) => panic!("expected tail append"),
    };
    assert_eq!(tail_append.start_local_idx, 1023);
    assert_eq!(tail_append.entries.len(), 1);
    assert_eq!(tail_append.entries[0].file.path, first);
    assert_eq!(first_result.batch.blocks.len(), 1);
    assert_eq!(first_result.batch.blocks[0].entries.len(), 1);
    assert_eq!(first_result.batch.blocks[0].entries[0].file.path, second);
    assert!(first_result.final_scan);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn tombstone_refresh_restores_exact_paths_without_importing_other_files() {
    let root = unique_test_dir("worker_exact_refresh_only");
    fs::create_dir_all(&root).expect("create root");
    let restored = root.join("restored.png");
    let unrelated = root.join("unrelated.png");
    fs::write(&restored, b"not-a-real-png").expect("write restored media");
    fs::write(&unrelated, b"not-a-real-png").expect("write unrelated media");

    let (tx, rx) = mpsc::channel();
    stream_canvas_scan_merge(
        vec![restored.clone(), unrelated.clone()],
        Vec::new(),
        vec![TombstoneRestoreSeed {
            idx: 3,
            id: 77,
            path: restored.clone(),
        }],
        200,
        Some(root.as_path()),
        1,
        1,
        1,
        SceneLayoutCursor::new_centered(8.0),
        10,
        2,
        None,
        None,
        None,
        false,
        &tx,
    );

    let result = rx.recv().expect("scan result");
    assert!(result.final_scan);
    assert_eq!(result.restores.len(), 1);
    assert_eq!(result.restores[0].idx, 3);
    assert_eq!(result.restores[0].file.id, 77);
    assert_eq!(result.restores[0].file.path, restored);
    assert!(result.batch.tail.is_none());
    assert!(result.batch.blocks.is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn exact_restore_only_emits_empty_batch_snapshot() {
    let root = unique_test_dir("worker_restore_only_empty_batch");
    fs::create_dir_all(&root).expect("create root");
    let restored = root.join("restored.png");
    fs::write(&restored, b"not-a-real-png").expect("write restored media");

    let (tx, rx) = mpsc::channel();
    stream_canvas_scan_merge(
        vec![restored.clone()],
        Vec::new(),
        vec![TombstoneRestoreSeed {
            idx: 5,
            id: 15,
            path: restored.clone(),
        }],
        200,
        Some(root.as_path()),
        1,
        1,
        1,
        SceneLayoutCursor::new_centered(8.0),
        10,
        2,
        None,
        None,
        None,
        false,
        &tx,
    );

    let result = rx.recv().expect("scan result");
    assert!(result.final_scan);
    assert_eq!(result.restores.len(), 1);
    assert!(result.batch.blocks.is_empty());
    assert!(result.batch.tail.is_none());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn duplicate_discovered_paths_do_not_duplicate_restore_or_import() {
    let root = unique_test_dir("worker_duplicate_paths");
    fs::create_dir_all(&root).expect("create root");
    let restored = root.join("restored.png");
    let new_media = root.join("new_media.png");
    fs::write(&restored, b"not-a-real-png").expect("write restored media");
    fs::write(&new_media, b"not-a-real-png").expect("write new media");

    let (tx, rx) = mpsc::channel();
    stream_canvas_scan_merge(
        vec![
            restored.clone(),
            restored.clone(),
            new_media.clone(),
            new_media.clone(),
        ],
        Vec::new(),
        vec![TombstoneRestoreSeed {
            idx: 3,
            id: 77,
            path: restored.clone(),
        }],
        200,
        Some(root.as_path()),
        1,
        1,
        1,
        SceneLayoutCursor::new_centered(8.0),
        10,
        2,
        None,
        None,
        None,
        true,
        &tx,
    );

    let result = rx.recv().expect("scan result");
    assert_eq!(result.restores.len(), 1);
    assert_eq!(result.batch.blocks.len(), 1);
    assert_eq!(result.batch.blocks[0].entries.len(), 1);
    assert_eq!(result.batch.blocks[0].entries[0].file.path, new_media);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn closed_channel_stops_scan_without_panicking() {
    let root = unique_test_dir("worker_closed_channel");
    fs::create_dir_all(&root).expect("create root");
    let new_media = root.join("new_media.png");
    fs::write(&new_media, b"not-a-real-png").expect("write media");

    let (tx, rx) = mpsc::channel();
    drop(rx);

    stream_canvas_scan_merge(
        vec![new_media],
        Vec::new(),
        Vec::new(),
        100,
        Some(root.as_path()),
        1,
        1,
        1,
        SceneLayoutCursor::new_centered(8.0),
        0,
        1,
        None,
        None,
        None,
        true,
        &tx,
    );

    let _ = fs::remove_dir_all(root);
}

fn unique_test_dir(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("nenarwia_{label}_{suffix}"))
}
