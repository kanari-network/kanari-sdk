module kanari_framework::object {
    use std::bcs;
    use std::vector;

    // Error codes
    const ENO_PERMISSIONS: u64 = 0;
    const EOBJECT_NOT_FOUND: u64 = 1;

    /// Object identifier structure
    struct ID has copy, drop, store {
        bytes: address
    }

    /// Unique identifier for objects
    struct UID has store {
        id: ID
    }

    /// Create a new ID from an address
    public fun new_from_address(addr: address): ID {
        ID { bytes: addr }
    }

    /// Convert ID to bytes
    public fun id_to_bytes(id: &ID): vector<u8> {
        bcs::to_bytes(&id.bytes)
    }

    /// Convert ID to address
    public fun id_to_address(id: &ID): address {
        id.bytes
    }

    /// Get inner ID from UID
    public fun uid_as_inner(uid: &UID): &ID {
        &uid.id
    }

    /// Convert UID to bytes
    public fun uid_to_bytes(uid: &UID): vector<u8> {
        bcs::to_bytes(&uid.id.bytes)
    }

        /// Get the underlying `ID` as bytes for an object
    public fun id_bytes<T: key>(obj: &T): vector<u8> {
        bcs::to_bytes(&borrow_uid(obj).id) 
    }

    /// Get the inner bytes as address for an object's ID
    public fun id_address<T: key>(obj: &T): address {
        borrow_uid(obj).id.bytes
    }

    /// Get ID as inner bytes
    public fun uid_to_inner(uid: &UID): ID {
        uid.id
    }

    /// Get UID as address 
    public fun uid_to_address(uid: &UID): address {
        uid.id.bytes
    }

    /// Delete a UID
    public fun delete(uid: UID) {
        let UID { id: ID { bytes: _ } } = uid;
    }

    /// Get the underlying `ID` of `obj`
    public fun id<T: key>(obj: &T): ID {
        borrow_uid(obj).id
    }

    /// Borrow the underlying `ID` of `obj`
    public fun borrow_id<T: key>(obj: &T): &ID {
        &borrow_uid(obj).id
    }

    native fun borrow_uid<T: key>(obj: &T): &UID;

}