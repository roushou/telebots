//! Presentation: the model list rendered into a `Block`.

use cloudflare_ai::ImageModel;
use telebots_core::Block;

/// The model list shown after the built-in `/help` command list.
pub fn model_table() -> Block {
    let mut b = Block::new();
    b.line("Models — prefix /imagine with one (default flux-1-schnell):");
    for model in ImageModel::ALL {
        b.row([model.aliases().join(", "), model.description().to_string()]);
    }
    b
}
