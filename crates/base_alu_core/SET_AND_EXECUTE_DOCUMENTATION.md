# `set_and_execute` Method Documentation

## Overview

The `set_and_execute` function (`arch/test.rs:58-96`) is a test utility function that prepares operands for a RISC-V base ALU instruction, executes it, and verifies the result. It handles both register-to-register operations and immediate operations, where the second operand (`c`) can be either a register value or an immediate value embedded in the instruction.

## Function Signature

```rust
pub fn set_and_execute<RA: Arena, E: PreflightExecutor<F, RA>>(
    tester: &mut impl TestBuilder<F>,
    executor: &mut E,
    arena: &mut RA,
    rng: &mut StdRng,
    opcode: BaseAluOpcode,
    b: Option<[u8; RV32_REGISTER_NUM_LIMBS]>,
    is_imm: Option<bool>,
    c: Option<[u8; RV32_REGISTER_NIMBS]>,
)
```

## Parameters

- **`tester`**: Test builder for setting up memory/registers and executing instructions
- **`executor`**: Preflight executor that executes instructions and builds execution records
- **`arena`**: Arena for storing execution records
- **`rng`**: Random number generator for generating random values when inputs are not provided
- **`opcode`**: The ALU operation to execute (e.g., ADD, SUB, AND, OR, etc.)
- **`b`**: First operand (optional). If `None`, generates random bytes.
- **`is_imm`**: Determines whether `c` should be treated as an immediate value
  - `Some(true)`: Force immediate mode
  - `Some(false)`: Force register mode
  - `None`: Randomly choose between immediate and register mode (50% probability each)
- **`c`**: Second operand (optional). Interpretation depends on `is_imm`:
  - In immediate mode: Used as the immediate value if provided, otherwise generated randomly
  - In register mode: Written to a register if provided, otherwise generated randomly

## Behavior by `is_imm` Value

### Case 1: `is_imm = Some(true)` (Immediate Mode)

When `is_imm` is `Some(true)`, the function forces the instruction to use an immediate value for the second operand.

#### Step-by-Step Execution:

1. **Operand `b` Preparation** (line 68):
   - If `b` is `Some(...)`, uses the provided value
   - If `b` is `None`, generates random bytes: `array::from_fn(|_| rng.gen_range(0..=u8::MAX))`

2. **Operand `c` and Immediate Value Determination** (lines 69-75):
   - Since `is_imm.unwrap_or(...)` evaluates to `true`, enters the immediate branch
   - **If `c` is provided** (`Some(...)`):
     - Extracts the immediate value: `(u32::from_le_bytes(c) & 0xFFFFFF) as usize`
       - Converts `c` bytes to a little-endian `u32`
       - Masks to 24 bits (0xFFFFFF) to get a 12-bit sign-extended immediate
     - Uses `c` as-is for the byte array representation
   - **If `c` is `None`**:
     - Calls `generate_rv32_is_type_immediate(rng)` to generate a random immediate
       - Generates a 12-bit immediate value (sign-extended to 24 bits)
       - Returns both the immediate value (`usize`) and its byte representation
   - Sets `c_imm = Some(imm)` where `imm` is the extracted/generated immediate value
   - Sets `c` to the byte array (either provided or generated)

3. **Instruction Creation** (lines 83-90):
   - Calls `rv32_rand_write_register_or_imm(tester, b, c, c_imm, opcode.global_opcode().as_usize(), rng)`
   - Inside `rv32_rand_write_register_or_imm`:
     - `rs2_is_imm = imm.is_some()` evaluates to `true` (line 36)
     - Generates random pointers for `rs1`, `rs2`, and `rd` registers
     - **Writes `b` to register `rs1`** (line 42)
     - **Does NOT write `c` to register `rs2`** (lines 43-45: condition `!rs2_is_imm` is false)
     - Creates instruction with immediate flag set to `0` (line 50: `if rs2_is_imm { 0 } else { 1 }`)
     - The instruction encodes: `[rd, rs1, rs2, 1, 0]` where the last `0` indicates immediate mode

4. **Instruction Execution** (line 91):
   - Calls `tester.execute(executor, arena, &instruction)`
   - The executor reads `b` from register `rs1`
   - The executor uses the immediate value encoded in the instruction (from `rs2` field) as `c`
   - Performs ALU operation: `a = run_alu(opcode, b, c)`
   - Writes result `a` to register `rd`

5. **Result Verification** (lines 93-95):
   - Computes expected result: `run_alu::<RV32_REGISTER_NUM_LIMBS, RV32_CELL_BITS>(opcode, &b, &c)`
   - Reads actual result from register `rd`: `tester.read::<RV32_REGISTER_NUM_LIMBS>(1, rd)`
   - Asserts they match: `assert_eq!(a, tester.read(...))`

#### Key Characteristics:
- ✅ `c` is **NOT** written to any register
- ✅ The immediate value is encoded directly in the instruction
- ✅ The instruction flag indicates immediate mode (`0`)
- ✅ The executor extracts the immediate from the instruction encoding

---

### Case 2: `is_imm = Some(false)` (Register Mode)

When `is_imm` is `Some(false)`, the function forces the instruction to use a register value for the second operand.

#### Step-by-Step Execution:

1. **Operand `b` Preparation** (line 68):
   - Same as Case 1: uses provided value or generates random bytes

2. **Operand `c` Determination** (lines 76-80):
   - Since `is_imm.unwrap_or(...)` evaluates to `false`, enters the register branch
   - Sets `c_imm = None` (line 78)
   - **If `c` is provided** (`Some(...)`):
     - Uses the provided value as-is
   - **If `c` is `None`**:
     - Generates random bytes: `array::from_fn(|_| rng.gen_range(0..=u8::MAX))`

3. **Instruction Creation** (lines 83-90):
   - Calls `rv32_rand_write_register_or_imm(tester, b, c, c_imm, opcode.global_opcode().as_usize(), rng)`
   - Inside `rv32_rand_write_register_or_imm`:
     - `rs2_is_imm = imm.is_some()` evaluates to `false` (since `c_imm = None`)
     - Generates random pointers for `rs1`, `rs2`, and `rd` registers
     - **Writes `b` to register `rs1`** (line 42)
     - **Writes `c` to register `rs2`** (lines 43-45: condition `!rs2_is_imm` is true)
     - Creates instruction with immediate flag set to `1` (line 50: `if rs2_is_imm { 0 } else { 1 }`)
     - The instruction encodes: `[rd, rs1, rs2, 1, 1]` where the last `1` indicates register mode

4. **Instruction Execution** (line 91):
   - Calls `tester.execute(executor, arena, &instruction)`
   - The executor reads `b` from register `rs1`
   - The executor reads `c` from register `rs2`
   - Performs ALU operation: `a = run_alu(opcode, b, c)`
   - Writes result `a` to register `rd`

5. **Result Verification** (lines 93-95):
   - Computes expected result: `run_alu::<RV32_REGISTER_NUM_LIMBS, RV32_CELL_BITS>(opcode, &b, &c)`
   - Reads actual result from register `rd`: `tester.read::<RV32_REGISTER_NUM_LIMBS>(1, rd)`
   - Asserts they match: `assert_eq!(a, tester.read(...))`

#### Key Characteristics:
- ✅ `c` **IS** written to register `rs2`
- ✅ The instruction flag indicates register mode (`1`)
- ✅ The executor reads `c` from the register
- ✅ Standard register-to-register operation

---

### Case 3: `is_imm = None` (Random Mode)

When `is_imm` is `None`, the function randomly chooses between immediate and register mode with equal probability (50% each).

#### Step-by-Step Execution:

1. **Operand `b` Preparation** (line 68):
   - Same as Cases 1 and 2: uses provided value or generates random bytes

2. **Mode Selection** (line 69):
   - Evaluates `is_imm.unwrap_or(rng.gen_bool(0.5))`
   - Since `is_imm` is `None`, `unwrap_or` uses the fallback: `rng.gen_bool(0.5)`
   - This randomly returns `true` or `false` with 50% probability each
   - **If random value is `true`**: Follows the same path as Case 1 (immediate mode)
   - **If random value is `false`**: Follows the same path as Case 2 (register mode)

3. **Subsequent Steps**:
   - The execution path depends on the randomly chosen mode
   - All other steps follow the same logic as the corresponding case above

#### Key Characteristics:
- ⚠️ **Non-deterministic behavior**: Different runs may produce different instruction encodings
- ⚠️ Requires a seeded RNG for reproducible tests
- ✅ Useful for fuzz testing and randomized test generation
- ✅ Tests both immediate and register modes in a single test suite

---

## Important Notes

### Immediate Value Encoding

When in immediate mode, the immediate value is encoded as a 12-bit sign-extended value:
- The value is extracted from `c` bytes using: `(u32::from_le_bytes(c) & 0xFFFFFF) as usize`
- The `& 0xFFFFFF` mask ensures only 24 bits are used (12-bit immediate sign-extended)
- If bit 11 (0x800) is set, it's sign-extended to the full 32-bit value

### Register vs Immediate Flag

The instruction encoding includes a flag (5th element) that indicates the mode:
- `0`: Immediate mode (`rs2` field contains immediate value)
- `1`: Register mode (`rs2` field is a register pointer)

### ALU Operation

The `run_alu` function performs the actual ALU computation:
- Takes `opcode`, `b`, and `c` as inputs
- Returns the result as an array of bytes
- This is used both for execution (by the executor) and verification (by the test)

### Execution Record

The executor creates an execution record (`BaseAluCoreRecord`) containing:
- `b`: First operand
- `c`: Second operand (same value regardless of immediate/register mode)
- `local_opcode`: The ALU operation opcode

Note that the execution record stores `c` as bytes, not distinguishing between immediate and register modes—that distinction is only relevant for instruction encoding and execution.

---

## Example Usage

```rust
// Immediate mode: ADD with immediate value
set_and_execute(
    &mut tester,
    &mut executor,
    &mut arena,
    &mut rng,
    BaseAluOpcode::ADD,
    Some([0x78, 0x56, 0x34, 0x12]),  // b = 0x12345678
    Some(true),                       // Force immediate mode
    Some([0x0A, 0x00, 0x00, 0x00]),  // c = 0x0000000A (immediate)
);

// Register mode: SUB with register value
set_and_execute(
    &mut tester,
    &mut executor,
    &mut arena,
    &mut rng,
    BaseAluOpcode::SUB,
    Some([0x78, 0x56, 0x34, 0x12]),  // b = 0x12345678
    Some(false),                      // Force register mode
    Some([0x0A, 0x00, 0x00, 0x00]),  // c = 0x0000000A (written to register)
);

// Random mode: AND with random mode selection
set_and_execute(
    &mut tester,
    &mut executor,
    &mut arena,
    &mut rng,
    BaseAluOpcode::AND,
    None,                             // Random b
    None,                             // Random mode (50% immediate, 50% register)
    None,                             // Random c
);
```

---

## Summary Table

| `is_imm` Value | Mode | `c_imm` | `c` Written to Register? | Instruction Flag | `c` Source |
|----------------|------|---------|---------------------------|------------------|-------------|
| `Some(true)`   | Immediate | `Some(imm)` | ❌ No | `0` | Encoded in instruction |
| `Some(false)`  | Register  | `None`      | ✅ Yes | `1` | Read from register `rs2` |
| `None`         | Random    | Random      | Random | Random | Depends on random choice |
