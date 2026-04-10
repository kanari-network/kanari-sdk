use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::gas_algebra::InternalGas;
use move_core_types::vm_status::StatusCode;
use move_vm_types::gas::GasMeter;

/// KanariGasMeter: Acts as a step counter.
/// Used to prevent DDoS attacks or endless loops.
/// It does not involve deducting coins from the user (because the Gas cost is 0).
pub struct KanariGasMeter {
    /// The number of steps that have been used
    steps_used: u64,
    /// The maximum limit of steps (e.g., 1,000,000 steps per Transaction)
    steps_limit: u64,
}

impl KanariGasMeter {
    pub fn new(steps_limit: u64) -> Self {
        Self {
            steps_used: 0,
            steps_limit,
        }
    }

    /// Main function for deducting execution quota
    #[inline]
    pub fn charge_step(&mut self, amount: u64) -> PartialVMResult<()> {
        self.steps_used = self.steps_used.saturating_add(amount);

        if self.steps_used > self.steps_limit {
            return Err(PartialVMError::new(StatusCode::OUT_OF_GAS).with_message(
                "Kanari Execution Limit Exceeded: Infinite Loop Detected!".to_string(),
            ));
        }
        Ok(())
    }
}

const NATIVE_FUNCTION_BASE_COST: u64 = 10;

// ==============================================================================
// GasMeter Trait Implementation (Updated for latest Move VM)
// ==============================================================================

impl GasMeter for KanariGasMeter {
    fn remaining_gas(&self) -> InternalGas {
        let remaining = self.steps_limit.saturating_sub(self.steps_used);
        InternalGas::new(remaining)
    }

    fn charge_simple_instr(
        &mut self,
        _instr: move_vm_types::gas::SimpleInstruction,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_pop(
        &mut self,
        _popped_val: impl move_vm_types::views::ValueView,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_call(
        &mut self,
        _module_id: &move_core_types::language_storage::ModuleId,
        _func_name: &str,
        _args: impl ExactSizeIterator<Item = impl move_vm_types::views::ValueView>,
        _num_locals: move_core_types::gas_algebra::NumArgs,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_call_generic(
        &mut self,
        _module_id: &move_core_types::language_storage::ModuleId,
        _func_name: &str,
        _ty_args: impl ExactSizeIterator<Item = impl move_vm_types::views::TypeView>,
        _args: impl ExactSizeIterator<Item = impl move_vm_types::views::ValueView>,
        _num_locals: move_core_types::gas_algebra::NumArgs,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_ld_const(
        &mut self,
        _size: move_core_types::gas_algebra::NumBytes,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_ld_const_after_deserialization(
        &mut self,
        _val: impl move_vm_types::views::ValueView,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_copy_loc(
        &mut self,
        _val: impl move_vm_types::views::ValueView,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_move_loc(
        &mut self,
        _val: impl move_vm_types::views::ValueView,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_store_loc(
        &mut self,
        _val: impl move_vm_types::views::ValueView,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_pack(
        &mut self,
        _is_generic: bool,
        _args: impl ExactSizeIterator<Item = impl move_vm_types::views::ValueView>,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_unpack(
        &mut self,
        _is_generic: bool,
        _args: impl ExactSizeIterator<Item = impl move_vm_types::views::ValueView>,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_read_ref(
        &mut self,
        _val: impl move_vm_types::views::ValueView,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_write_ref(
        &mut self,
        _new_val: impl move_vm_types::views::ValueView,
        _old_val: impl move_vm_types::views::ValueView,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_eq(
        &mut self,
        _lhs: impl move_vm_types::views::ValueView,
        _rhs: impl move_vm_types::views::ValueView,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_neq(
        &mut self,
        _lhs: impl move_vm_types::views::ValueView,
        _rhs: impl move_vm_types::views::ValueView,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_vec_pack<'a>(
        &mut self,
        _ty: impl move_vm_types::views::TypeView + 'a,
        _args: impl ExactSizeIterator<Item = impl move_vm_types::views::ValueView>,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_vec_len(&mut self, _ty: impl move_vm_types::views::TypeView) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_vec_borrow(
        &mut self,
        _is_mut: bool,
        _ty: impl move_vm_types::views::TypeView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_vec_push_back(
        &mut self,
        _ty: impl move_vm_types::views::TypeView,
        _val: impl move_vm_types::views::ValueView,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_vec_pop_back(
        &mut self,
        _ty: impl move_vm_types::views::TypeView,
        _val: Option<impl move_vm_types::views::ValueView>,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_vec_unpack(
        &mut self,
        _ty: impl move_vm_types::views::TypeView,
        _expect_num_elements: move_core_types::gas_algebra::NumArgs,
        _elems: impl ExactSizeIterator<Item = impl move_vm_types::views::ValueView>,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_vec_swap(&mut self, _ty: impl move_vm_types::views::TypeView) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_native_function(
        &mut self,
        _amount: InternalGas,
        _ret_vals: Option<impl ExactSizeIterator<Item = impl move_vm_types::views::ValueView>>,
    ) -> PartialVMResult<()> {
        self.charge_step(NATIVE_FUNCTION_BASE_COST)
    }

    fn charge_native_function_before_execution(
        &mut self,
        _ty_args: impl ExactSizeIterator<Item = impl move_vm_types::views::TypeView>,
        _args: impl ExactSizeIterator<Item = impl move_vm_types::views::ValueView>,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn charge_drop_frame(
        &mut self,
        _locals: impl Iterator<Item = impl move_vm_types::views::ValueView>,
    ) -> PartialVMResult<()> {
        self.charge_step(1)
    }

    fn get_profiler_mut(&mut self) -> Option<&mut move_vm_profiler::GasProfiler> {
        None
    }

    fn set_profiler(&mut self, _profiler: move_vm_profiler::GasProfiler) {
        // Do nothing
    }
}
