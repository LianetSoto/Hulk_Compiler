pub mod types;
pub mod type_checker;
pub mod inference;

pub use type_checker::{TypeChecker, FlattenedMethod, FlattenedType};
pub use types::HulkType;
pub use inference::Unifier;
pub mod base_detector;