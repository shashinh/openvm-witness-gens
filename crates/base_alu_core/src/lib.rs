use openvm_circuit::{arch::testing::TestChipHarness, openvm_stark_sdk::p3_baby_bear::BabyBear};
use openvm_rv32im_circuit::{Rv32BaseAluAir, Rv32BaseAluChip, Rv32BaseAluExecutor};

pub mod arch;
pub mod harness;

// pub const MAX_INS_CAPACITY: usize = 128;
// pub type F = BabyBear;
// pub type Harness = TestChipHarness<F, Rv32BaseAluExecutor, Rv32BaseAluAir, Rv32BaseAluChip<F>>;