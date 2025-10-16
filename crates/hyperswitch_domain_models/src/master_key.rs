pub trait MasterKeyInterface {
    fn get_master_key(&self) -> &[u8];
}
