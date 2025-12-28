use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
pub enum Operation {
    Insert,
    Delete,
    StartTxn,
    CommitTxn
}