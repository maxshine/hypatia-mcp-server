use crate::services::hypatia_memory_tools::dirs_home;
use hypatia;
use std::path::{Path, PathBuf};

pub(crate) fn run_init_shelf(name: Option<&str>) -> hypatia::Result<()> {
    let shelf_name = name.unwrap_or("default");
    let mut lab = hypatia::lab::Lab::new()?;
    let shelf_path = dirs_home().join(".hypatia").join(&shelf_name);
    let shelf = target_shelf(&mut lab, Some(&shelf_path), Some(&shelf_name))?;
    for line in report(&lab.shelf_status(&shelf)?) {
        println!("{line}");
    }
    Ok(())
}

/// Whether two paths name one directory, through symlinks too (macOS `/tmp` is `/private/tmp`).
pub(crate) fn same_dir(a: &Path, b: &Path) -> bool {
    let key = |p: &Path| std::fs::canonicalize(p).or_else(|_| std::path::absolute(p));
    matches!((key(a), key(b)), (Ok(a), Ok(b)) if a == b)
}

/// The shelf to set up: the default one, a registered one by name, or the one in `path`,
/// connected unless it already is. Running it again changes nothing, and no registration is
/// ever replaced.
fn target_shelf(
    lab: &mut hypatia::lab::Lab,
    path: Option<&Path>,
    name: Option<&str>,
) -> hypatia::Result<String> {
    let registered: Vec<(String, PathBuf, bool)> = lab
        .list_shelves()
        .into_iter()
        .map(|(known, dir, connected)| (known.to_string(), dir.clone(), connected))
        .collect();
    let shelf = match path {
        None => {
            let name = name.unwrap_or("default");
            if !registered.iter().any(|(known, _, _)| known == name) {
                return Err(hypatia::HypatiaError::Shelf(format!(
                    "shelf '{name}' is not connected; run `hypatia init <dir> -n {name}` to set it up"
                )));
            }
            name.to_string()
        }
        Some(path) => {
            // Named and registered as typed, with `..` resolved; compared through links.
            let path = resolve(path)?;
            match registered.iter().find(|(_, dir, _)| same_dir(dir, &path)) {
                Some((existing, _, _)) if name.is_none_or(|name| name == existing) => {
                    existing.clone()
                }
                // Under another name: connecting refuses it, as `hypatia connect` does.
                Some(_) => return lab.connect_shelf(&path, name),
                None => {
                    let wanted = name.map_or_else(
                        || hypatia::model::ShelfId::new(path.clone()).name,
                        String::from,
                    );
                    // Connecting would re-point that name, even at a shelf that failed to open.
                    if let Some((_, other, _)) =
                        registered.iter().find(|(known, _, _)| *known == wanted)
                    {
                        return Err(hypatia::HypatiaError::Shelf(format!(
                            "the name '{wanted}' belongs to the shelf at {}; choose another with `-n <name>`",
                            other.display()
                        )));
                    }
                    return lab.connect_shelf(&path, name);
                }
            }
        }
    };
    // Registered but not open: it failed to open at startup, so open it again to say why.
    if registered
        .iter()
        .any(|(known, _, connected)| *known == shelf && !connected)
    {
        lab.reopen_shelf(&shelf)?;
    }
    Ok(shelf)
}

/// `path` made absolute with `..` resolved, but a link at its end kept: the shelf is named and
/// registered after the name the user gave, not where that link points today.
fn resolve(path: &Path) -> std::io::Result<PathBuf> {
    let path = std::path::absolute(path)?;
    Ok(match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => std::fs::canonicalize(parent)
            .unwrap_or_else(|_| parent.to_path_buf())
            .join(name),
        _ => std::fs::canonicalize(&path).unwrap_or(path),
    })
}

/// What `hypatia init` prints: what works, what does not yet, and the command that fixes it.
fn report(status: &hypatia::lab::ShelfStatus) -> Vec<String> {
    let backend = if status.postgres {
        "PostgreSQL"
    } else {
        "SQLite"
    };
    let mut lines = vec![
        format!(
            "✓ Shelf '{}' is ready: {} ({backend})",
            status.name,
            status.path.display()
        ),
        "✓ Full-text search, graph traversal and JSE queries work".to_string(),
    ];
    let debt = &status.debt;
    let pending = debt.pending_knowledge + debt.pending_statement;
    match (&status.semantic_search_off, &status.attention) {
        (Some(off), _) => {
            lines.push(format!("○ Semantic search is off: {off}"));
            lines.push(
                "  Entries already on the shelf get vectors automatically once it is on."
                    .to_string(),
            );
        }
        (None, Some(attention)) => {
            lines.push(format!("! Semantic search needs attention: {attention}"));
        }
        (None, None) => {
            // Set up, not proven: init neither loads the model nor calls the API.
            lines.push(format!(
                "✓ Semantic search is set up with {}",
                status.embedder
            ));
            if pending > 0 {
                lines.push(match &debt.paused {
                    Some(paused) => format!(
                        "  {pending} entries are waiting for vectors; automatic embedding is paused: {}",
                        paused.reason
                    ),
                    None => format!(
                        "  {pending} entries are waiting for vectors; they are embedded as you keep using hypatia, or all at once with `hypatia backfill -s {}`",
                        status.name
                    ),
                });
            }
        }
    }
    lines
}
