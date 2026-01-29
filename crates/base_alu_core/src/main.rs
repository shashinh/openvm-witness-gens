use base_alu_core::harness::build_execution_record;
use openvm_rv32im_transpiler::BaseAluOpcode;

fn main() {
    let opcode = BaseAluOpcode::ADD;
    let b = Some([0x78, 0x56, 0x34, 0x12]);
    let c = Some([0xF0, 0xDE, 0xBC, 0x00]);
    let is_imm = Some(false);
    
    build_execution_record(opcode, b, c, is_imm);
}
