use base_alu_core::harness::trace_gen;
use openvm_rv32im_transpiler::BaseAluOpcode;

fn main() {
    let opcode = BaseAluOpcode::ADD;
    let b = Some([0x78, 0x56, 0x34, 0x12]);
    let c = Some([0xF0, 0xDE, 0xBC, 0x00]);
    //is_imm isn't strictly a public input, but the testing infrastructure requires it
    //let is_imm = Some(false);
    let is_imm = Some(false);
    
    //TODO:
    // 1. trait fn for converting fuzzer output to witness gen input
    // 2. trait fn that the fuzz test can invoke to initiate trace gen with these inputs
    trace_gen(opcode, b, c, is_imm);
}
