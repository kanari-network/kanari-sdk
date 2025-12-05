pub mod object_storage;
pub mod pending_objects;

pub use object_storage::{Object, ObjectID, ObjectStorage, Owner};
pub use pending_objects::{
    ObjectFreeze, ObjectShare, ObjectTransfer, PendingObjectOps, PendingObjectOpsRef,
};
