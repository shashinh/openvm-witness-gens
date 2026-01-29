# Base ALU Core Witness Generation

This crate provides scaffolding to build a harness necessary to construct an execution record for the base ALU core and then generate trace rows for the corresponding AIR.

## Overview

The crate implements a complete trace generation pipeline for the base ALU operations in the RISC-V 32-bit integer multiplication extension (RV32IM). It follows these main steps:

1. **Harness Construction**: Sets up the necessary components (AIR, executor, chip) for trace generation
2. **Execution Record Construction**: Executes instructions and records execution data (called an execution record internally)
3. **Trace Generation**: Consumes execution record and fills trace rows via `generate_proving_ctx` and `fill_trace_row` (both internal to openvm)
4. **Trace Extraction**: Slices and maps the generated trace to base ALU core columns

## Architecture

### Execution Record Structure

The execution record (`BaseAluCoreRecord`) contains (this crate does not directly interact with this, it is constructed and consumed internally in openvm):
- `b: [u8; NUM_LIMBS]`: First operand
- `c: [u8; NUM_LIMBS]`: Second operand  
- `local_opcode: u8`: The ALU operation opcode
These can be considered the *public inputs* to the witness generator.

## Workflow

### 1. Harness Setup

The `create_harness` function (`arch/test.rs`) constructs the harness:

```rust
let (mut harness, bitwise) = create_harness(&tester);
```
The associated methods are reproduced verbatim from extensions/rv32im/circuit/src/base_alu/tests.rs (they are test methods and therefore not visible externally).

This creates:
- A bitwise operation lookup chip for range checks (unused)
- The composite AIR, executor, and chip
- A harness with capacity for `MAX_INS_CAPACITY` instructions 

### 2. Execution Record Construction (`set_and_execute`)

The `set_and_execute` function (`arch/test.rs:58-96`, reproduced verbatim from extensions/rv32im/circuit/src/base_alu/tests.rs) prepares inputs and executes an instruction:

1. **Input Preparation**: 
   - Sets up operands `b` and `c` (either provided or randomly generated)
   - Determines if `c` should be treated as an immediate value
   - Generates immediate value if needed

2. **Instruction Creation**:
   - Calls `rv32_rand_write_register_or_imm` to create an instruction
   - Writes input values to registers/memory via `tester.write`
   - Returns an `Instruction` with the opcode and register pointers

3. **Execution**:
   - Calls `tester.execute(executor, arena, &instruction)`
   - This invokes `PreflightExecutor::execute` on `Rv32BaseAluExecutor`
   - The executor:
     - Allocates records in the arena (adapter + core records)
     - Reads operands `b` and `c` via the adapter
     - Computes result `a = run_alu(opcode, b, c)`
     - Writes result back via the adapter
     - Stores execution data in `BaseAluCoreRecord`

### 3. Trace Generation

The `load` method (`harness.rs:55`) triggers trace generation:

```rust
let tester = tester.build().load(harness);
```

This internally:

1. **Extracts Arena**: Gets the `MatrixRecordArena` from the harness containing execution records

2. **Generates Proving Context**:
   - Calls `harness.chip.generate_proving_ctx(arena)`
   - For `Rv32BaseAluChip`, this dispatches to `VmChipWrapper::generate_proving_ctx`
   - Converts execution records in the arena into a matrix representation
   - Creates an `AirProvingContext` containing the trace

3. **Fills Trace Rows** (internal to `generate_proving_ctx`):
   - For each row in the trace, calls `fill_trace_row` on the chip's filler
   - The filler consumes execution records and populates trace columns:
     - **Adapter columns**: Filled by `Rv32BaseAluAdapterFiller::fill_trace_row` (which we don't care about while dealing with the core trace gen)
       - Memory read/write auxiliary columns
       - Register pointers (rs1, rs2, rd)
       - Timestamps and PC values
       - Immediate value handling
     - **Core columns**: Filled by `BaseAluFiller::fill_trace_row`
       - Operands `b` and `c` (from execution record)
       - Result `a` (computed via `run_alu`)
       - Opcode flags (add_flag, sub_flag, xor_flag, or_flag, and_flag)

4. **Returns Context**: The `AirProvingContext` is stored in `tester.air_ctxs` (unused)

### 4. Trace Extraction and Mapping

After trace generation, the trace is extracted and mapped to columns:

1. **Extract Trace**:
   ```rust
   let base_alu_core_air_ctx = air_ctxs.iter().find(|(air, _)| air.name().contains("BaseAluCore"));
   let trace: RowMajorMatrix<F> = base_alu_core_air_ctx.unwrap().1.common_main.as_ref().unwrap().as_ref().clone().to_row_major_matrix();
   ```

2. **Slice Trace Row**:
   - Gets the first row: `trace.row_slice(0)`
   - Splits at `adapter_width` to separate adapter and core portions:
     ```rust
     let (_adapter_row, core_row) = trace_row.split_at(adapter_width);
     ```

3. **Map to Columns**:
   - Casts the core row slice to `BaseAluCoreCols`:
     ```rust
     let core_cols: &BaseAluCoreCols<_, RV32_REGISTER_NUM_LIMBS, RV32_CELL_BITS> = core_row.borrow();
     ```

The `BaseAluCoreCols` structure contains:
- `a: [F; NUM_LIMBS]`: ALU result
- `b: [F; NUM_LIMBS]`: First operand
- `c: [F; NUM_LIMBS]`: Second operand
- `opcode_add_flag: F`: Selector for ADD operation
- `opcode_sub_flag: F`: Selector for SUB operation
- `opcode_xor_flag: F`: Selector for XOR operation
- `opcode_or_flag: F`: Selector for OR operation
- `opcode_and_flag: F`: Selector for AND operation

## Usage

The main entry point is `trace_gen` (`harness.rs:23-79`):

```rust
trace_gen(opcode, b, c, is_imm);
```

This function:
1. Sets up the harness
2. Computes width metadata for adapter and core AIRs
3. Calls `set_and_execute` to construct execution records
4. Calls `load(harness)` to generate traces
5. Extracts and prints the core columns

**Example** (`main.rs`):
```rust
let opcode = BaseAluOpcode::ADD;
let b = Some([0x78, 0x56, 0x34, 0x12]);
let c = Some([0xF0, 0xDE, 0xBC, 0x00]);
let is_imm = Some(false);

trace_gen(opcode, b, c, is_imm);
```
Simply execute `cargo run` to demonstrate trace gen with a hardcoded set of public inputs.
