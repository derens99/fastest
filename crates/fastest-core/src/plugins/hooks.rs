//! Hook name constants matching pytest's plugin hook protocol.
//!
//! Each constant corresponds to a well-known pytest hook point. Plugins
//! receive these names via [`super::Plugin::on_hook`] and can choose which
//! hooks to handle.

/// Fired when test collection begins.
pub const COLLECTION_START: &str = "pytest_collection_start";

/// Fired to allow plugins to reorder or filter collected test items.
pub const COLLECTION_MODIFY_ITEMS: &str = "pytest_collection_modifyitems";

/// Fired when test collection has finished.
pub const COLLECTION_FINISH: &str = "pytest_collection_finish";

/// Fired before each test's setup phase.
pub const RUNTEST_SETUP: &str = "pytest_runtest_setup";

/// Fired for the actual test invocation.
pub const RUNTEST_CALL: &str = "pytest_runtest_call";

/// Fired during each test's teardown phase.
pub const RUNTEST_TEARDOWN: &str = "pytest_runtest_teardown";

/// Fired to report the result of a test phase.
pub const RUNTEST_LOGREPORT: &str = "pytest_runtest_logreport";

/// All known hook names, useful for validation.
pub const ALL_HOOKS: &[&str] = &[
    COLLECTION_START,
    COLLECTION_MODIFY_ITEMS,
    COLLECTION_FINISH,
    RUNTEST_SETUP,
    RUNTEST_CALL,
    RUNTEST_TEARDOWN,
    RUNTEST_LOGREPORT,
];
