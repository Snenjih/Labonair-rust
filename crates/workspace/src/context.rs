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
}
