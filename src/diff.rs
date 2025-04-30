use std::path::PathBuf;

pub trait BinaryDiff: Send + Sync + 'static {
    fn create_delta(&self, old_delta: &[u8], new_delta: &[u8]) -> Result<Vec<u8>, String>;
    fn apply_delta(&self, base_data: &[u8], delta: &[u8]) -> Result<Vec<u8>, String>;
    fn get_name(&self) -> &'static str;
    //To be used for algorithm selection
    fn is_suitable(&self, _old_path: Option<&PathBuf>, _new_path: Option<&PathBuf>) -> bool {
        true // For now
    }
}

pub type CreateDiffAlgorithm = unsafe fn() -> Box<dyn BinaryDiff>;
const PLUGIN_CREATE_FN_NAME: &str = "create_diff_algorithm";
