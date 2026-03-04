use std::{array, sync::Arc};

use openvm_circuit::{arch::{Arena, ExecutionBridge, PreflightExecutor, instructions::{LocalOpcode, VmOpcode, instruction::Instruction, riscv::RV32_REGISTER_NUM_LIMBS}, testing::{BITWISE_OP_LOOKUP_BUS, TestBuilder, TestChipHarness, VmChipTestBuilder, memory::gen_pointer}}, openvm_stark_sdk::{openvm_stark_backend::p3_field::FieldAlgebra, p3_baby_bear::BabyBear}, system::memory::{SharedMemoryHelper, offline_checker::MemoryBridge}};
use openvm_circuit_primitives::bitwise_op_lookup::{BitwiseOperationLookupAir, BitwiseOperationLookupBus, BitwiseOperationLookupChip, SharedBitwiseOperationLookupChip};
use openvm_rv32im_circuit::{BaseAluCoreAir, BaseAluFiller, Rv32BaseAluAir, Rv32BaseAluChip, Rv32BaseAluExecutor, adapters::{RV_IS_TYPE_IMM_BITS, RV32_CELL_BITS, Rv32BaseAluAdapterAir, Rv32BaseAluAdapterExecutor, Rv32BaseAluAdapterFiller}, run_alu};
use openvm_rv32im_transpiler::BaseAluOpcode;
use rand::{Rng, rngs::StdRng};

use crate::harness::{F, Harness, MAX_INS_CAPACITY};


pub fn generate_rv32_is_type_immediate(rng: &mut StdRng) -> (usize, [u8; RV32_REGISTER_NUM_LIMBS]) {
    let mut imm: u32 = rng.gen_range(0..(1 << RV_IS_TYPE_IMM_BITS));
    if (imm & 0x800) != 0 {
        imm |= !0xFFF
    }
    (
        (imm & 0xFFFFFF) as usize,
        [
            imm as u8,
            (imm >> 8) as u8,
            (imm >> 16) as u8,
            (imm >> 16) as u8,
        ],
    )
}

pub fn rv32_rand_write_register_or_imm<const NUM_LIMBS: usize>(
    tester: &mut impl TestBuilder<BabyBear>,
    rs1_writes: [u8; NUM_LIMBS],
    rs2_writes: [u8; NUM_LIMBS],
    imm: Option<usize>,
    opcode_with_offset: usize,
    rng: &mut StdRng,
) -> (Instruction<BabyBear>, usize) {
    let rs2_is_imm = imm.is_some();

    let rs1 = gen_pointer(rng, NUM_LIMBS);
    let rs2 = imm.unwrap_or_else(|| gen_pointer(rng, NUM_LIMBS));
    let rd = gen_pointer(rng, NUM_LIMBS);

    tester.write::<NUM_LIMBS>(1, rs1, rs1_writes.map(BabyBear::from_canonical_u8));
    if !rs2_is_imm {
        tester.write::<NUM_LIMBS>(1, rs2, rs2_writes.map(BabyBear::from_canonical_u8));
    }

    (
        Instruction::from_usize(
            VmOpcode::from_usize(opcode_with_offset),
            [rd, rs1, rs2, 1, if rs2_is_imm { 0 } else { 1 }],
        ),
        rd,
    )
}



pub fn set_and_execute<RA: Arena, E: PreflightExecutor<F, RA>>(
    tester: &mut impl TestBuilder<F>,
    executor: &mut E,
    arena: &mut RA,
    rng: &mut StdRng,
    opcode: BaseAluOpcode,
    b: Option<[u8; RV32_REGISTER_NUM_LIMBS]>,
    is_imm: Option<bool>,
    c: Option<[u8; RV32_REGISTER_NUM_LIMBS]>,
) {
    let b = b.unwrap_or(array::from_fn(|_| rng.gen_range(0..=u8::MAX)));
    let (c_imm, c) = if is_imm.unwrap_or(rng.gen_bool(0.5)) {
        let (imm, c) = if let Some(c) = c {
            ((u32::from_le_bytes(c) & 0xFFFFFF) as usize, c)
        } else {
            generate_rv32_is_type_immediate(rng)
        };
        (Some(imm), c)
    } else {
        (
            None,
            c.unwrap_or(array::from_fn(|_| rng.gen_range(0..=u8::MAX))),
        )
    };

    let (instruction, rd) = rv32_rand_write_register_or_imm(
        tester,
        b,
        c,
        c_imm,
        opcode.global_opcode().as_usize(),
        rng,
    );
    tester.execute(executor, arena, &instruction);

    let a = run_alu::<RV32_REGISTER_NUM_LIMBS, RV32_CELL_BITS>(opcode, &b, &c)
        .map(F::from_canonical_u8);
    assert_eq!(a, tester.read::<RV32_REGISTER_NUM_LIMBS>(1, rd))
}

fn create_harness_fields(
    memory_bridge: MemoryBridge,
    execution_bridge: ExecutionBridge,
    bitwise_chip: Arc<BitwiseOperationLookupChip<RV32_CELL_BITS>>,
    memory_helper: SharedMemoryHelper<F>,
) -> (Rv32BaseAluAir, Rv32BaseAluExecutor, Rv32BaseAluChip<F>) {
    let air = Rv32BaseAluAir::new(
        Rv32BaseAluAdapterAir::new(execution_bridge, memory_bridge, bitwise_chip.bus()),
        BaseAluCoreAir::new(bitwise_chip.bus(), BaseAluOpcode::CLASS_OFFSET),
    );
    let executor = Rv32BaseAluExecutor::new(
        Rv32BaseAluAdapterExecutor::new(),
        BaseAluOpcode::CLASS_OFFSET,
    );
    let chip = Rv32BaseAluChip::new(
        BaseAluFiller::new(
            Rv32BaseAluAdapterFiller::new(bitwise_chip.clone()),
            bitwise_chip,
            BaseAluOpcode::CLASS_OFFSET,
        ),
        memory_helper,
    );
    (air, executor, chip)
}

pub fn create_harness(
    tester: &VmChipTestBuilder<F>,
) -> (
    Harness,
    (
        BitwiseOperationLookupAir<RV32_CELL_BITS>,
        SharedBitwiseOperationLookupChip<RV32_CELL_BITS>,
    ),
) {
    let bitwise_bus = BitwiseOperationLookupBus::new(BITWISE_OP_LOOKUP_BUS);
    let bitwise_chip = Arc::new(BitwiseOperationLookupChip::<RV32_CELL_BITS>::new(
        bitwise_bus,
    ));

    let (air, executor, chip) = create_harness_fields(
        tester.memory_bridge(),
        tester.execution_bridge(),
        bitwise_chip.clone(),
        tester.memory_helper(),
    );
    let harness = Harness::with_capacity(executor, air, chip, MAX_INS_CAPACITY);

    (harness, (bitwise_chip.air, bitwise_chip))
}