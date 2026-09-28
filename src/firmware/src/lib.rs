pub mod board;

/// The image's `esp_app_desc_t`, which ESP-IDF's bootloader and the installer read: the
/// project and the version of `app::image`. esp-idf-sys's `esp_app_desc!` would say the
/// Cargo package's version instead. Each image's binary invokes it once.
#[macro_export]
macro_rules! image_description {
    () => {
        #[no_mangle]
        #[used]
        #[link_section = ".rodata_desc"]
        #[allow(non_upper_case_globals)]
        pub static esp_app_desc: esp_idf_svc::sys::esp_app_desc_t = esp_idf_svc::sys::esp_app_desc_t {
            magic_word: esp_idf_svc::sys::ESP_APP_DESC_MAGIC_WORD,
            secure_version: 0,
            reserv1: [0; 2],
            version: app::image::c_text(app::image::VERSION),
            project_name: app::image::c_text(app::image::PROJECT),
            time: [0; 16],
            date: [0; 16],
            idf_ver: app::image::c_text(esp_idf_svc::sys::const_format::formatcp!(
                "v{}.{}.{}",
                esp_idf_svc::sys::ESP_IDF_VERSION_MAJOR,
                esp_idf_svc::sys::ESP_IDF_VERSION_MINOR,
                esp_idf_svc::sys::ESP_IDF_VERSION_PATCH
            )),
            app_elf_sha256: [0; 32],
            min_efuse_blk_rev_full: esp_idf_svc::sys::CONFIG_ESP_EFUSE_BLOCK_REV_MIN_FULL as u16,
            max_efuse_blk_rev_full: esp_idf_svc::sys::CONFIG_ESP_EFUSE_BLOCK_REV_MAX_FULL as u16,
            // log2 of CONFIG_MMU_PAGE_SIZE, 64 KB, as ESP-IDF writes it.
            mmu_page_size: 16,
            reserv3: [0; 3],
            reserv2: [0; 18],
        };
    };
}
