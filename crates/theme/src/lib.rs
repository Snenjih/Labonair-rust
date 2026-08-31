//! Labonair theme system and design tokens.
//!
//! The single source of truth for the app's visual design is
//! `reference-src/src/styles/globals.css`. This crate transcribes every token
//! from that file into typed Rust data ([`Theme`]) with all Oklch colors
//! converted to [`gpui::Hsla`].
//!
//! Populated by Phase 01 (T02-*). T02-001 covers token extraction; later tasks
//! add the runtime theme provider/store and user import/export.

mod color;
mod tokens;

pub use color::{oklch, oklch_a, parse_color, to_rgb8, transparent};
pub use tokens::{
    Animation, AnsiColors, BorderVariants, CoreColors, CubicBezier, InteractionColors, RadiusScale,
    ShadowLayer, Shadows, SidebarColors, StatusColors, SurfaceColors, TerminalPalette, Theme,
    Typography,
};
