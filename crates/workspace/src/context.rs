//! Identity and activity state for project and standalone workspaces.
//!
//! The identity is deliberately separate from the tab count: a project
//! workspace may temporarily have no tabs, and a standalone workspace may
//! contain several tabs. The shell and tool modules therefore share one
//! state model instead of maintaining separate project/standalone layouts.

use std::path::{Path, PathBuf};

/// Durable identity of a workspace.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum WorkspaceIdentity {
    /// A temporary context for a one-off tool action.
    #[default]
    Standalone,
    /// A context anchored to a local or remote project root.
    Project { root: PathBuf },
}

impl WorkspaceIdentity {
    pub fn standalone() -> Self {
        Self::Standalone
    }

    pub fn project(root: impl Into<PathBuf>) -> Self {
        Self::Project { root: root.into() }
    }

    pub fn is_project(&self) -> bool {
        matches!(self, Self::Project { .. })
    }

    pub fn project_root(&self) -> Option<&Path> {
        match self {
            Self::Standalone => None,
            Self::Project { root } => Some(root),
        }
    }
}

/// Runtime activity of a workspace, combining identity with tab presence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceState {
    Empty {
        identity: WorkspaceIdentity,
    },
    Active {
        identity: WorkspaceIdentity,
        tab_count: usize,
    },
}

impl WorkspaceState {
    pub fn from_parts(identity: WorkspaceIdentity, tab_count: usize) -> Self {
        if tab_count == 0 {
            Self::Empty { identity }
        } else {
            Self::Active {
                identity,
                tab_count,
            }
        }
    }

    pub fn identity(&self) -> &WorkspaceIdentity {
        match self {
            Self::Empty { identity } | Self::Active { identity, .. } => identity,
        }
    }

    pub fn tab_count(&self) -> usize {
        match self {
            Self::Empty { .. } => 0,
            Self::Active { tab_count, .. } => *tab_count,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty { .. })
    }
}

/// Mutable workspace identity owned by the workspace module.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkspaceContext {
    identity: WorkspaceIdentity,
}

impl WorkspaceContext {
    pub fn standalone() -> Self {
        Self::default()
    }

    pub fn identity(&self) -> &WorkspaceIdentity {
        &self.identity
    }

    pub fn set_standalone(&mut self) {
        self.identity = WorkspaceIdentity::Standalone;
    }

    pub fn set_project(&mut self, root: impl Into<PathBuf>) {
        self.identity = WorkspaceIdentity::project(root);
    }
}

/// Resolve the filesystem root used by workspace-owned file surfaces.
///
/// An explicit project identity is authoritative. Standalone workspaces may
/// fall back to the active terminal directory and finally the user's home
/// directory so one-off sessions still have a useful explorer root.
pub fn resolve_filesystem_root(
    project_root: Option<PathBuf>,
    active_cwd: Option<String>,
    home: Option<PathBuf>,
) -> Option<String> {
    project_root
        .map(|path| path.to_string_lossy().into_owned())
        .or(active_cwd)
        .or_else(|| home.map(|path| path.to_string_lossy().into_owned()))
}

/// Resolve the repository root used by workspace-owned Git surfaces.
///
/// Git deliberately has no home-directory fallback: an unscoped standalone
/// workspace must not accidentally treat the user's home directory as a
/// repository.
pub fn resolve_git_root(
    project_root: Option<PathBuf>,
    active_cwd: Option<String>,
) -> Option<String> {
    project_root
        .map(|path| path.to_string_lossy().into_owned())
        .or(active_cwd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_active_are_orthogonal_to_identity() {
        let project = WorkspaceIdentity::project("/work/app");
        assert!(WorkspaceState::from_parts(project.clone(), 0).is_empty());
        assert_eq!(
            WorkspaceState::from_parts(project.clone(), 0).identity(),
            &project
        );
        assert_eq!(WorkspaceState::from_parts(project, 2).tab_count(), 2);

        let standalone = WorkspaceIdentity::standalone();
        assert!(WorkspaceState::from_parts(standalone.clone(), 0).is_empty());
        assert!(!WorkspaceState::from_parts(standalone, 1).is_empty());
    }

    #[test]
    fn context_transitions_do_not_create_a_second_layout_model() {
        let mut context = WorkspaceContext::standalone();
        assert!(!context.identity().is_project());

        context.set_project("/work/app");
        assert_eq!(
            context.identity().project_root(),
            Some(Path::new("/work/app"))
        );

        context.set_standalone();
        assert_eq!(context.identity(), &WorkspaceIdentity::Standalone);
    }

    #[test]
    fn explicit_project_root_wins_over_terminal_cwd() {
        assert_eq!(
            resolve_filesystem_root(
                Some(PathBuf::from("/project")),
                Some("/project/src".into()),
                Some(PathBuf::from("/Users/test")),
            ),
            Some("/project".into())
        );
        assert_eq!(
            resolve_git_root(Some(PathBuf::from("/project")), Some("/project/src".into())),
            Some("/project".into())
        );
    }

    #[test]
    fn standalone_root_uses_terminal_cwd_then_home() {
        assert_eq!(
            resolve_filesystem_root(
                None,
                Some("/tmp/standalone".into()),
                Some(PathBuf::from("/Users/test")),
            ),
            Some("/tmp/standalone".into())
        );
        assert_eq!(
            resolve_filesystem_root(None, None, Some(PathBuf::from("/Users/test"))),
            Some("/Users/test".into())
        );
        assert_eq!(resolve_git_root(None, None), None);
    }
}
