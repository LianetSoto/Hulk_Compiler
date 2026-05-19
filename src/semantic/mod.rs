pub mod types;
pub mod type_checker;
pub mod inference;

pub use type_checker::TypeChecker;
pub use types::HulkType;
pub use inference::Unifier;