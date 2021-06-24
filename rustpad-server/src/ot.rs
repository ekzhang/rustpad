//! Helper methods for working with operational transformation.

use operational_transform::{Operation, OperationSeq};

/// Return the new index of a position in the string.
pub fn transform_index(operation: &OperationSeq, position: u32) -> u32 {
    let mut index = position as i32;
    let mut new_index = index;
    for op in operation.ops() {
        match op {
            &Operation::Retain(n) => index -= n as i32,
            Operation::Insert(s) => new_index += bytecount::num_chars(s.as_bytes()) as i32,
            &Operation::Delete(n) => {
                new_index -= std::cmp::min(index, n as i32);
                index -= n as i32;
            }
        }
        if index < 0 {
            break;
        }
    }
    new_index as u32
}
