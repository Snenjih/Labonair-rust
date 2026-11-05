//! Filesystem adapter for Editor-owned project-search Multi-Buffers.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use labonair_editor::lifecycle::{
    Bom, Encoding, FileCapabilities, FileFormat, FileIdentity, FileSnapshot, LineEnding,
};
use labonair_editor::{
    MultiBuffer, MultiBufferFileContent, MultiBufferSaveOutcome, MultiBufferSaveRequest,
    MultiBufferSavedFile, ProjectSearchHit,
};
use labonair_filesystem::file::{
    load_editor_file_lifecycle_sync, save_editor_file_lifecycle_sync, EditorLifecycleLoad,
    StorageBom, StorageEncoding, StorageFileSnapshot, StorageLineEnding, StorageSaveIntent,
};

use crate::project_search::root_safe_path;

pub(crate) fn load(
    root: &Path,
    hits: &[ProjectSearchHit],
    context_lines: usize,
) -> Result<MultiBuffer, String> {
    let paths = hits
        .iter()
        .map(|hit| PathBuf::from(&hit.path))
        .collect::<BTreeSet<_>>();
    let mut files = Vec::with_capacity(paths.len());
    for candidate in paths {
        let path = root_safe_path(root, &candidate)?;
        let loaded = load_editor_file_lifecycle_sync(&path.to_string_lossy(), None)
            .map_err(|error| error.to_string())?;
        let snapshot = match loaded {
            EditorLifecycleLoad::Text { snapshot } => editor_snapshot(snapshot),
            EditorLifecycleLoad::Binary { .. } => {
                return Err(format!(
                    "{} is binary and cannot be shown as text",
                    path.display()
                ));
            }
            EditorLifecycleLoad::TooLarge { size, limit, .. } => {
                return Err(format!(
                    "{} is too large for the editor ({size} bytes; limit {limit})",
                    path.display()
                ));
            }
            EditorLifecycleLoad::UnsupportedEncoding { encoding, .. } => {
                return Err(format!(
                    "{} uses unsupported {encoding} encoding",
                    path.display()
                ));
            }
            EditorLifecycleLoad::InvalidUtf8 { .. } => {
                return Err(format!("{} is not valid UTF-8 text", path.display()));
            }
        };
        files.push(MultiBufferFileContent { snapshot });
    }
    MultiBuffer::from_project_search(hits, files, context_lines).map_err(|error| error.to_string())
}

pub(crate) fn save(
    root: &Path,
    requests: Vec<MultiBufferSaveRequest>,
) -> Result<MultiBufferSaveOutcome, String> {
    if requests.is_empty() {
        return Ok(MultiBufferSaveOutcome::default());
    }

    // Resolve every file and compare every accepted identity before writing any file.
    let mut current_snapshots = Vec::with_capacity(requests.len());
    for request in &requests {
        let path = root_safe_path(root, &request.path)?;
        let loaded = load_editor_file_lifecycle_sync(&path.to_string_lossy(), None)
            .map_err(|error| error.to_string())?;
        let EditorLifecycleLoad::Text { snapshot } = loaded else {
            return Err(format!(
                "{} is no longer a saveable text file",
                path.display()
            ));
        };
        let identity = FileIdentity::new(
            snapshot.identity.path.clone(),
            snapshot.identity.size,
            snapshot.identity.modified_ms,
            snapshot.identity.content_hash,
        );
        if identity != request.expected_identity {
            return Err(format!(
                "{} changed on disk; review the conflict before saving",
                path.display()
            ));
        }
        if !snapshot.capabilities.can_save {
            return Err(format!("{} is read-only", path.display()));
        }
        current_snapshots.push(snapshot);
    }

    let mut outcome = MultiBufferSaveOutcome::default();
    for (request, current) in requests.into_iter().zip(current_snapshots) {
        let (encoding, line_ending, bom) = storage_format(request.format);
        match save_editor_file_lifecycle_sync(
            &request.path.to_string_lossy(),
            &request.text,
            encoding,
            line_ending,
            bom,
            &current.identity,
            StorageSaveIntent::Normal,
        ) {
            Ok(snapshot) => outcome.saved.push(MultiBufferSavedFile {
                path: request.path,
                revision: request.revision,
                snapshot: editor_snapshot(snapshot),
            }),
            Err(error) => {
                outcome.error = Some(error.to_string());
                break;
            }
        }
    }
    Ok(outcome)
}

fn editor_snapshot(snapshot: StorageFileSnapshot) -> FileSnapshot {
    FileSnapshot::new(
        FileIdentity::new(
            snapshot.identity.path,
            snapshot.identity.size,
            snapshot.identity.modified_ms,
            snapshot.identity.content_hash,
        ),
        snapshot.text,
        match snapshot.line_ending {
            StorageLineEnding::Lf => LineEnding::Lf,
            StorageLineEnding::CrLf => LineEnding::CrLf,
            StorageLineEnding::Cr => LineEnding::Cr,
            StorageLineEnding::Mixed => LineEnding::Mixed,
        },
        match snapshot.encoding {
            StorageEncoding::Utf8 => Encoding::Utf8,
            StorageEncoding::Utf16Le => Encoding::Utf16Le,
            StorageEncoding::Utf16Be => Encoding::Utf16Be,
            StorageEncoding::Windows1252 => Encoding::Windows1252,
            StorageEncoding::ShiftJis => Encoding::ShiftJis,
        },
        match snapshot.bom {
            StorageBom::None => Bom::None,
            StorageBom::Utf8 => Bom::Utf8,
            StorageBom::Utf16Le => Bom::Utf16Le,
            StorageBom::Utf16Be => Bom::Utf16Be,
        },
        FileCapabilities {
            can_read: snapshot.capabilities.can_read,
            can_edit: snapshot.capabilities.can_edit,
            can_save: snapshot.capabilities.can_save,
        },
    )
}

fn storage_format(format: FileFormat) -> (StorageEncoding, StorageLineEnding, StorageBom) {
    let encoding = match format.encoding {
        Encoding::Utf8 => StorageEncoding::Utf8,
        Encoding::Utf16Le => StorageEncoding::Utf16Le,
        Encoding::Utf16Be => StorageEncoding::Utf16Be,
        Encoding::Windows1252 => StorageEncoding::Windows1252,
        Encoding::ShiftJis => StorageEncoding::ShiftJis,
    };
    let line_ending = match format.line_ending {
        LineEnding::Lf => StorageLineEnding::Lf,
        LineEnding::CrLf => StorageLineEnding::CrLf,
        LineEnding::Cr => StorageLineEnding::Cr,
        LineEnding::Mixed => StorageLineEnding::Mixed,
    };
    let bom = match format.bom {
        Bom::None => StorageBom::None,
        Bom::Utf8 => StorageBom::Utf8,
        Bom::Utf16Le => StorageBom::Utf16Le,
        Bom::Utf16Be => StorageBom::Utf16Be,
    };
    (encoding, line_ending, bom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ROOT: AtomicUsize = AtomicUsize::new(0);

    fn temp_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "labonair-project-multibuffer-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn hit(path: &Path, relative: &str, text: &str) -> ProjectSearchHit {
        ProjectSearchHit::with_relative_path(
            path.to_string_lossy(),
            relative,
            1,
            text,
            0,
            "needle".chars().count(),
        )
    }

    #[test]
    fn project_search_load_and_save_preserve_crlf_format() {
        let root = temp_root();
        std::fs::create_dir_all(&root).expect("create root");
        let path = root.join("source.txt");
        std::fs::write(&path, b"needle\r\n").expect("write source");
        let mut buffer =
            load(&root, &[hit(&path, "source.txt", "needle")], 0).expect("load multi-buffer");
        let excerpt = buffer.snapshot().excerpts()[0].id;
        buffer
            .replace_excerpt_text(excerpt, "updated\n")
            .expect("edit excerpt");
        let requests = buffer.save_requests().expect("save requests");
        assert_eq!(requests[0].format.line_ending, LineEnding::CrLf);
        let outcome = save(&root, requests).expect("save multi-buffer");
        assert_eq!(outcome.saved.len(), 1);
        assert_eq!(
            std::fs::read(&path).expect("read saved file"),
            b"updated\r\n"
        );
        buffer
            .mark_saved_snapshot(
                &path,
                outcome.saved[0].revision,
                outcome.saved[0].snapshot.clone(),
            )
            .expect("accept saved snapshot");
        assert!(!buffer.is_dirty(&path).expect("saved state"));
        std::fs::remove_dir_all(root).expect("remove root");
    }

    #[test]
    fn save_preflights_every_file_before_writing_any_file() {
        let root = temp_root();
        std::fs::create_dir_all(&root).expect("create root");
        let first = root.join("a.txt");
        let second = root.join("b.txt");
        std::fs::write(&first, b"needle a\n").expect("write first");
        std::fs::write(&second, b"needle b\n").expect("write second");
        let hits = [
            hit(&first, "a.txt", "needle a"),
            hit(&second, "b.txt", "needle b"),
        ];
        let mut buffer = load(&root, &hits, 0).expect("load multi-buffer");
        for excerpt in buffer.snapshot().excerpts().to_vec() {
            buffer
                .replace_excerpt_text(excerpt.id, "changed\n")
                .expect("edit file");
        }
        let requests = buffer.save_requests().expect("save requests");
        std::fs::write(&second, b"external change\n").expect("external write");
        let error = save(&root, requests).expect_err("external change must stop save");
        assert!(error.contains("changed on disk"));
        assert_eq!(std::fs::read(&first).expect("read first"), b"needle a\n");
        assert_eq!(
            std::fs::read(&second).expect("read second"),
            b"external change\n"
        );
        std::fs::remove_dir_all(root).expect("remove root");
    }
}
