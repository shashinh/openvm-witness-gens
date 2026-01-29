use std::borrow::Borrow;

use openvm_circuit::arch::RowMajorMatrixArena;
use openvm_circuit::arch::instructions::{LocalOpcode, riscv::RV32_REGISTER_NUM_LIMBS};
use openvm_circuit::arch::testing::{TestChipHarness, VmChipTestBuilder};
use openvm_circuit::openvm_stark_sdk::openvm_stark_backend::p3_air::BaseAir;
use openvm_circuit::openvm_stark_sdk::openvm_stark_backend::p3_matrix::Matrix;
use openvm_circuit::openvm_stark_sdk::openvm_stark_backend::p3_matrix::dense::RowMajorMatrix;
use openvm_circuit::openvm_stark_sdk::p3_baby_bear::BabyBear;
use openvm_rv32im_circuit::adapters::{RV32_CELL_BITS, Rv32BaseAluAdapterAir};
use openvm_rv32im_circuit::{BaseAluCoreAir, BaseAluCoreCols, Rv32BaseAluAir, Rv32BaseAluChip};
use openvm_rv32im_circuit::{Rv32BaseAluExecutor, adapters::Rv32BaseAluAdapterExecutor};
use openvm_rv32im_transpiler::BaseAluOpcode;
use rand::{rngs::StdRng, SeedableRng};

use crate::arch::test::set_and_execute;
use crate::arch::test::create_harness;

pub const MAX_INS_CAPACITY: usize = 128;
pub type F = BabyBear;
pub type Harness = TestChipHarness<F, Rv32BaseAluExecutor, Rv32BaseAluAir, Rv32BaseAluChip<F>>;

pub fn build_execution_record(
    opcode: BaseAluOpcode,
    b: Option<[u8; RV32_REGISTER_NUM_LIMBS]>,
    c: Option<[u8; RV32_REGISTER_NUM_LIMBS]>,
    is_imm: Option<bool>,
) {
    //set up harness
    let mut tester = VmChipTestBuilder::default();
    let (mut harness, bitwise) = create_harness(&tester);

    //width metadata for later
    let air_width = <Rv32BaseAluAir as BaseAir<F>>::width(&harness.air);
    let adapter_width = <Rv32BaseAluAdapterAir as BaseAir<F>>::width(&harness.air.adapter);
    let core_width = <BaseAluCoreAir<RV32_REGISTER_NUM_LIMBS, RV32_CELL_BITS> as BaseAir<F>>::width(&harness.air.core);    //extract the 


    //initialize inputs for execution record
    //we don't need the rng since all inputs are deterministic, but we need to pass it to the function
    let mut rng = StdRng::from_entropy();
    
    set_and_execute(
        &mut tester,
        &mut harness.executor,
        &mut harness.arena,
        &mut rng,
        opcode,
        b,
        is_imm,
        c,
    );

    //this invokes generate_proving_ctx and then fill_trace_row, which performs trace gen.
    let tester = tester.build().load(harness);

    let air_ctxs = tester.air_ctxs;

    // bit of a hack, but we know the name of the air we want
    let base_alu_core_air_ctx = air_ctxs.iter().find(|(air, _)| air.name().contains("BaseAluCore"));
    let mut trace : RowMajorMatrix<F> = base_alu_core_air_ctx.unwrap().1.common_main.as_ref().unwrap().as_ref().clone().to_row_major_matrix();
    let width = trace.width();
    //if all went well, the trace width should match the combined (adapter+core) air width
    assert_eq!(width, air_width);

    //since we only executed one instruction, the trace should have height 1
    assert_eq!(trace.height(), 1);

    let trace_row = trace.row_slice(0);
    let trace_row: &[F] = (*trace_row).borrow();

    // println!("trace row: {:#?}", trace_row);

    let (_adapter_row, core_row) = trace_row.split_at(adapter_width);

    let core_cols: &BaseAluCoreCols<_, RV32_REGISTER_NUM_LIMBS, RV32_CELL_BITS> = core_row.borrow();

    println!("core cols: {:?}", core_cols);





}
