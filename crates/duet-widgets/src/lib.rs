//! `duet-widgets` façade wrapping UI controls to isolate direct GPUI / `gpui-component` usage.
//! Implements ADR-0002 (UI Isolation Boundary) and ADR-0003 (`gpui-compat` Shim Strategy).

pub mod gpui_compat;
pub mod table;
pub mod list;
pub mod input;
pub mod select;
pub mod menu;
pub mod dialog;
pub mod toast;
pub mod panels;
pub mod function_bar;
pub mod status_bar;
pub mod tab_bar;

// Re-exports for convenient top-level access
pub use gpui_compat::ContextShim;
pub use dialog::DialogWidget;
pub use function_bar::{FunctionBar, FunctionKey};
pub use input::{InputState, InputWidget};
pub use list::ListWidget;
pub use menu::{MenuItem, MenuWidget};
pub use panels::{ResizableSplitter, SplitDirection, SplitterState};
pub use select::{SelectOption, SelectWidget};
pub use status_bar::{StatusBar, StatusBarData};
pub use tab_bar::{TabBar, TabBarColors, TabItem};
pub use table::{TableColumnConfig, TableWidget, TextAlignment};
pub use toast::{ToastKind, ToastMessage, ToastWidget};
