module kanari_framework::event {
    /// Emit a custom Move event, sending the data offchain.
    ///
    /// Used for creating custom indexes and tracking onchain
    /// activity in a way that suits a specific application the most.
    ///
    /// The type `T` is the main way to index the event, and can contain
    /// phantom parameters, eg `emit(MyEvent<phantom T>)`.
    public native fun emit<T: copy + drop>(event: T);
}