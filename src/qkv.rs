use crate::utils::rw::ReaderWriter;
use crate::utils::types::QuickKVConfig;

#[derive(Debug)]
pub struct QuickKV {
    pub config: QuickKVConfig,
    pub rw: ReaderWriter,
}

impl QuickKV {
    pub fn new(config: Option<QuickKVConfig>) -> Self {
        let config =  match config {
            Some(config) => config,
            None => QuickKVConfig::default(),
        };

        QuickKV {
            config: config.clone(),
            rw: ReaderWriter::new(config),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quickkv_new() {
        let quickkv = QuickKV::new(None);
        assert_eq!(quickkv.config.db_file, "db.qkv".to_string().into());
        assert_eq!(quickkv.config.max_db_size, None);

        std::fs::remove_file("db.qkv").unwrap();
    }

    #[test]
    fn test_quickkv_new_with_config() {
        let config = QuickKVConfig {
            db_file: Some("test.qkv".to_string()),
            max_db_size: Some(100),
        };
        let quickkv = QuickKV::new(Some(config));
        assert_eq!(quickkv.config.db_file, "test.qkv".to_string().into());
        assert_eq!(quickkv.config.max_db_size, Some(100));

        std::fs::remove_file("test.qkv").unwrap();
    }
}
