#[path = "dynamic_model/support.rs"]
mod dynamic_model_support;

use dynamic_model_support::{TEST_MUTEX, TestContext};
use maenifold_core::vector;

// T-10.11: RTM FR-18 / RTM-098 — when both tokenizer.json and manual fallback assets exist,
// runtime loading must prefer tokenizer.json rather than rebuilding from vocab.txt.
#[test]
fn t10_11_tokenizer_json_is_preferred_over_vocab_txt() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();
    ctx.install_valid_shadow_model_with_fallback_assets("preferred-tokenizer.onnx");
    ctx.write_vocab("this is intentionally not a valid wordpiece vocab");
    ctx.write_tokenizer_config(r#"{"pad_token":"[PAD]","model_max_length":96}"#);
    ctx.write_tokenizer_json();

    vector::load_model().expect("load model via tokenizer.json");

    assert_eq!(
        vector::model_filename().as_deref(),
        Some("preferred-tokenizer.onnx")
    );
    assert_eq!(vector::max_tokens(), Some(96));
    assert_eq!(vector::embedding_dimensions(), Some(384));
    assert_eq!(
        vector::generate_embedding("tokenizer preference")
            .expect("embedding from tokenizer.json")
            .len(),
        384
    );
}

// T-10.11: RTM FR-18 / RTM-099 — tokenizer_config.json should supply model_max_length when
// present, and runtime loading should fall back to 512 when the field is absent.
#[test]
fn t10_11_max_tokens_reads_config_and_defaults_to_512() {
    let _guard = TEST_MUTEX.lock();

    {
        let ctx = TestContext::new();
        ctx.install_valid_shadow_model_with_fallback_assets("configured-max-tokens.onnx");
        ctx.write_tokenizer_config(r#"{"pad_token":"[PAD]","model_max_length":128}"#);
        ctx.write_tokenizer_json();

        vector::load_model().expect("load configured max tokens model");
        assert_eq!(vector::max_tokens(), Some(128));
    }

    {
        let ctx = TestContext::new();
        ctx.install_valid_shadow_model_with_fallback_assets("default-max-tokens.onnx");
        ctx.write_tokenizer_config(r#"{"pad_token":"[PAD]"}"#);
        ctx.write_tokenizer_json();

        vector::load_model().expect("load default max tokens model");
        assert_eq!(vector::max_tokens(), Some(512));
    }
}
