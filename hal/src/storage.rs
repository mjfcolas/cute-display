use crate::Fault;

pub trait FileStorage {
    fn root_entries(&self) -> Result<Vec<String>, Fault>;
    fn capacity_bytes(&self) -> Result<u64, Fault>;
}
