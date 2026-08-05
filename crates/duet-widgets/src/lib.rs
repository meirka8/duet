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

pub mod copy_move_dialog;
pub mod progress_manager;
pub mod conflict_dialog;
pub mod error_report_dialog;
pub mod recovery_overlay;
pub mod delete_dialog;
pub mod dir_rename_link_dialog;
pub mod permissions_dialog;
pub mod viewer_widget;
pub mod search_dialog;
pub mod quick_view;
pub mod pack_dialog;
pub mod unpack_dialog;
pub mod drive_bar;
pub mod connection_manager;
pub mod plugin_manager;

pub use pack_dialog::{ArchiveFormat, PackDialog, PackDialogState};
pub use unpack_dialog::{UnpackDialog, UnpackDialogState};
pub use drive_bar::{DriveBar, DriveBarData, DriveEntry, DriveKind};
pub use connection_manager::{ConnectionManagerDialog, ConnectionManagerDialogState};
pub use plugin_manager::{PluginManagerDialog, PluginManagerDialogState};

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

pub use copy_move_dialog::{CopyMoveDialog, CopyMoveDialogState, CopyMoveOptions};
pub use progress_manager::{
    JobItemDisplay, JobManagerModalState, OperationManagerModal, StatusProgressTray,
    StatusProgressTrayData,
};
pub use conflict_dialog::{ConflictDialogState, ConflictResolutionDialog, FileMetaSide};
pub use error_report_dialog::{ErrorLogEntry, ErrorReportDialog, ErrorReportState};
pub use recovery_overlay::{JournalRecoveryEntry, StartupRecoveryOverlay, StartupRecoveryState};
pub use delete_dialog::{DeleteConfirmationDialog, DeleteDialogState};
pub use dir_rename_link_dialog::{
    CreateDirDialog, CreateDirDialogState, CreateLinkDialog, CreateLinkDialogState, LinkKind,
    RenameDialog, RenameDialogState,
};
pub use permissions_dialog::{PermissionsDialog, PermissionsDialogState};
pub use viewer_widget::{
    InternalViewerWidget, ViewerEncoding, ViewerMode, ViewerSearchState, ViewerState,
};
pub use search_dialog::{SearchDialogState, SearchResultEntry, SearchViewWidget};
pub use quick_view::QuickViewWidget;
