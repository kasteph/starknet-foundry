use blockifier::{
    execution::{
        entry_point::{CallEntryPoint, CallType},
        execution_utils::ReadOnlySegment,
        syscalls::{
            hint_processor::{create_retdata_segment, SyscallExecutionError},
            SyscallResult,
        },
    },
    transaction::transaction_utils::update_remaining_gas,
};
use cairo_vm::vm::vm_core::VirtualMachine;
use starknet_api::{
    core::{ClassHash, EntryPointSelector},
    deprecated_contract_class::EntryPointType,
    transaction::Calldata,
};

use super::{
    cheatable_syscall_handler::CheatableSyscallHandler, entry_point::execute_call_entry_point,
};

// blockifier/src/execution/syscalls/hint_processor.rs:541 (execute_inner_call)
pub fn execute_inner_call(
    call: &mut CallEntryPoint,
    vm: &mut VirtualMachine,
    syscall_handler: &mut CheatableSyscallHandler<'_>, // Changed parameter type
    remaining_gas: &mut u64,
) -> SyscallResult<ReadOnlySegment> {
    // region: Modified blockifier code
    let call_info = execute_call_entry_point(
        call,
        syscall_handler.child.state,
        syscall_handler.cheatnet_state,
        syscall_handler.child.resources,
        syscall_handler.child.context,
    )?;
    // endregion

    let raw_retdata = &call_info.execution.retdata.0;

    if call_info.execution.failed {
        // TODO(spapini): Append an error word according to starknet spec if needed.
        // Something like "EXECUTION_ERROR".
        return Err(SyscallExecutionError::SyscallError {
            error_data: raw_retdata.clone(),
        });
    }

    let retdata_segment = create_retdata_segment(vm, &mut syscall_handler.child, raw_retdata)?;
    update_remaining_gas(remaining_gas, &call_info);

    syscall_handler.child.inner_calls.push(call_info);

    Ok(retdata_segment)
}

// blockifier/src/execution/syscalls/hint_processor.rs:577 (execute_library_call)
pub fn execute_library_call(
    syscall_handler: &mut CheatableSyscallHandler<'_>, // Modified parameter type
    vm: &mut VirtualMachine,
    class_hash: ClassHash,
    call_to_external: bool,
    entry_point_selector: EntryPointSelector,
    calldata: Calldata,
    remaining_gas: &mut u64,
) -> SyscallResult<ReadOnlySegment> {
    let entry_point_type = if call_to_external {
        EntryPointType::External
    } else {
        EntryPointType::L1Handler
    };
    let mut entry_point = CallEntryPoint {
        class_hash: Some(class_hash),
        code_address: None,
        entry_point_type,
        entry_point_selector,
        calldata,
        // The call context remains the same in a library call.
        storage_address: syscall_handler.child.storage_address(),
        caller_address: syscall_handler.child.caller_address(),
        call_type: CallType::Delegate,
        initial_gas: *remaining_gas,
    };

    execute_inner_call(&mut entry_point, vm, syscall_handler, remaining_gas)
}
