//! Cost of a completion, derived from per-model token prices.
//!
//! Prices are Cloudflare Workers AI's per-million-token rates, expressed in
//! micro-USD per million tokens so cost can be computed with integer math.

use cloudflare_ai::{TextModel, Usage};

/// Computes completion cost in micro-USD from token usage.
pub struct Pricing;

impl Pricing {
    /// Cost of one completion in micro-USD (millionths of a dollar).
    pub fn cost_micro_usd(model: TextModel, usage: &Usage) -> u64 {
        // (micro-USD per million tokens) for input and output.
        let (input, output) = match model {
            TextModel::Llama318b => (45_000, 384_000),
            TextModel::Llama323b => (51_000, 335_000),
            TextModel::Llama3370b => (293_000, 2_253_000),
            TextModel::DeepseekR132b => (497_000, 4_881_000),
            _ => (0, 0),
        };
        usage.prompt_tokens * input / 1_000_000 + usage.completion_tokens * output / 1_000_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(prompt: u64, completion: u64) -> Usage {
        Usage {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
        }
    }

    #[test]
    fn cost_combines_input_and_output_prices() {
        // 1M prompt + 1M completion on llama-3.1-8b = $0.045 + $0.384 = $0.429
        // = 429,000 micro-USD.
        let cost = Pricing::cost_micro_usd(TextModel::Llama318b, &usage(1_000_000, 1_000_000));
        assert_eq!(cost, 429_000);
    }

    #[test]
    fn cost_scales_with_tokens() {
        let cost = Pricing::cost_micro_usd(TextModel::Llama323b, &usage(500_000, 1_000_000));
        // 0.5M * $0.051 + 1M * $0.335 = $0.0255 + $0.335 = $0.3605 → 360,500.
        assert_eq!(cost, 360_500);
    }

    #[test]
    fn deepseek_is_priced_higher() {
        let cost = Pricing::cost_micro_usd(TextModel::DeepseekR132b, &usage(1_000_000, 1_000_000));
        assert_eq!(cost, 5_378_000);
    }
}
